use std::time::{Duration, Instant};

use median_market::{
    Budget, Client, Daily, DayFiles, Doorman, Gate, Host, MarketError, Presence, PriceOrder,
    RivenSearch, Selector,
};

fn client() -> (Client, Doorman) {
    let (gate, doorman) = Gate::open(
        Budget {
            per_second: 1.0,
            burst: 1.0,
            failures_before_open: 3,
            cooldown: Duration::from_secs(60),
        },
        Instant::now(),
    );
    let client = Client::new(
        gate,
        Host::default(),
        concat!("median-market-livecheck/", env!("CARGO_PKG_VERSION")),
    )
    .expect("client builds");
    (client, doorman)
}

#[tokio::test]
#[ignore = "goes to the network"]
async fn an_item_declares_its_variant_space() {
    let (market, doorman) = client();

    let relic = market
        .item(doorman.admit().await.expect("a token"), "lith_r2_relic")
        .await
        .expect("relic answers");
    let subtypes = relic.subtypes.expect("a relic declares subtypes");
    assert!(
        subtypes.contains(&"radiant".to_string()) && subtypes.contains(&"intact".to_string()),
        "unexpected relic subtypes: {subtypes:?}"
    );

    let ranked = market
        .item(
            doorman.admit().await.expect("a token"),
            "primed_cleanse_corrupted",
        )
        .await
        .expect("mod answers");
    assert_eq!(ranked.max_rank, Some(10));

    let ayatan = market
        .item(
            doorman.admit().await.expect("a token"),
            "ayatan_anasa_sculpture",
        )
        .await
        .expect("sculpture answers");
    assert!(
        ayatan.max_amber_stars.is_some() && ayatan.max_cyan_stars.is_some(),
        "a sculpture should declare both kinds of star"
    );

    let part = market
        .item(
            doorman.admit().await.expect("a token"),
            "mirage_prime_systems_blueprint",
        )
        .await
        .expect("part answers");
    assert!(part.max_rank.is_none() && part.subtypes.is_none());
    assert!(part.tradable);
}

#[tokio::test]
#[ignore = "goes to the network"]
async fn statistics_arrive_in_four_series() {
    let (market, doorman) = client();
    let stats = market
        .statistics(
            doorman.admit().await.expect("a token"),
            "mirage_prime_systems_blueprint",
        )
        .await
        .expect("statistics answer");

    assert!(
        !stats.closed_daily.is_empty(),
        "ninety days of closed buckets should not be empty for a traded part"
    );
    assert!(
        !stats.live_daily.is_empty(),
        "the live series should not be empty"
    );
    // The hourly series may be empty for a quiet item, so only its shape is checked.
    for bucket in &stats.closed_hourly {
        assert!(
            bucket.datetime.len() >= 13,
            "odd datetime: {}",
            bucket.datetime
        );
    }
}

#[tokio::test]
#[ignore = "goes to the network"]
async fn one_answer_carries_the_whole_book_including_offline_sellers() {
    let (market, doorman) = client();
    let book = market
        .book(
            doorman.admit().await.expect("a token"),
            "mirage_prime_systems_blueprint",
        )
        .await
        .expect("book answers");

    let rows = book.rows().len();
    assert!(
        rows > 50,
        "a traded part should have a deep book, got {rows}"
    );
    assert!(
        book.rows()
            .iter()
            .any(|order| order.user.status == Presence::Offline),
        "the book should carry offline sellers too"
    );

    let whole = book.snapshot(&Selector::Whole);
    let lowest = whole.lowest.expect("somebody sells");
    let reachable = whole.lowest_ingame.expect("somebody reachable sells");
    assert!(
        reachable >= lowest,
        "reachable {reachable} cannot be cheaper than the cheapest {lowest}"
    );
    assert!(whole.sellers_ingame >= 1);
}

#[tokio::test]
#[ignore = "goes to the network"]
async fn the_book_tells_ranks_apart() {
    let (market, doorman) = client();
    let book = market
        .book(
            doorman.admit().await.expect("a token"),
            "primed_cleanse_corrupted",
        )
        .await
        .expect("book answers");

    let bare = book.snapshot(&Selector::Rank(0)).lowest;
    let maxed = book.snapshot(&Selector::Rank(10)).lowest;
    assert!(
        bare.is_some() && maxed.is_some(),
        "both ranks should be on sale"
    );
    assert!(
        maxed > bare,
        "a maxed mod should not be cheaper than a bare one: {maxed:?} vs {bare:?}"
    );
}

#[tokio::test]
#[ignore = "goes to the network"]
async fn an_item_the_market_does_not_list_says_so() {
    let (market, doorman) = client();
    let answer = market
        .item(
            doorman.admit().await.expect("a token"),
            "no_such_item_at_all_17082026",
        )
        .await;
    match answer {
        Err(MarketError::NotListed(_)) => {}
        other => panic!("expected NotListed, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "goes to the network"]
async fn v1_statistics_still_answer() {
    let (market, doorman) = client();
    let statistics = market
        .statistics(doorman.admit().await.expect("a token"), "lith_r2_relic")
        .await;
    assert!(
        !matches!(statistics, Err(MarketError::Retired(_))),
        "v1 statistics answered Retired — the whole price history source is gone"
    );
}

#[tokio::test]
#[ignore]
async fn a_day_file_carries_the_whole_market_with_its_variants() {
    let day_files = DayFiles::new(concat!(
        "median-market-livecheck/",
        env!("CARGO_PKG_VERSION")
    ))
    .expect("client builds");

    let mut found = None;
    for back in 1..=4 {
        let date = median_market::day_of(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("after the epoch")
                .as_secs() as i64
                - back * 86_400,
        );
        if let Some(rows) = day_files.day(&date).await.expect("the host answers") {
            found = Some((date, rows));
            break;
        }
    }

    let (date, rows) = found.expect("one of the last four days is there");
    let closed: Vec<&Daily> = rows.iter().filter(|row| row.is_closed()).collect();
    let ranked = rows
        .iter()
        .filter(|row| row.bucket.mod_rank.is_some())
        .count();
    let subtyped = rows
        .iter()
        .filter(|row| row.bucket.subtype.is_some())
        .count();

    println!(
        "{date}: {} rows, {} closed, {ranked} ranked, {subtyped} subtyped",
        rows.len(),
        closed.len()
    );

    assert!(rows.len() > 5_000, "a day is smaller than expected");
    assert!(!closed.is_empty(), "no closed trades in a whole day");
    assert!(ranked > 0 && subtyped > 0, "the file lost its variants");
    assert!(
        rows.iter().all(|row| row.bucket.extra.is_empty()),
        "a row carried a field this build does not know"
    );
}

#[tokio::test]
#[ignore = "goes to the network"]
async fn riven_auctions_read_from_the_search_and_the_recent_feed() {
    let (market, doorman) = client();

    let search = RivenSearch {
        weapon: "torid".into(),
        buyout_only: true,
        order: Some(PriceOrder::Cheapest),
        ..RivenSearch::default()
    };
    let found = market
        .riven_auctions(doorman.admit().await.expect("a token"), &search)
        .await
        .expect("the search answers");
    assert!(found.iter().all(|auction| auction.weapon == "torid"));
    assert!(found.iter().all(|auction| auction.buyout_price.is_some()));

    tokio::time::sleep(Duration::from_millis(3500)).await;
    let recent = market
        .recent_auctions(doorman.admit().await.expect("a token"))
        .await
        .expect("the feed answers");
    assert!(!recent.is_empty(), "the feed holds rivens most of the time");

    tokio::time::sleep(Duration::from_millis(3500)).await;
    let one = market
        .auction(doorman.admit().await.expect("a token"), &recent[0].id)
        .await
        .expect("one auction answers");
    assert_eq!(one.map(|auction| auction.id), Some(recent[0].id.clone()));
}
