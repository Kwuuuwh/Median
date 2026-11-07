use crate::time::{DAY, HOUR, Instant};

/// Near price window in seconds (48 hours); prices are read from it first.
pub const NEAR: Instant = 48 * HOUR;

/// Far price window in seconds (90 days), used when the near window has fewer than `THIN` trades.
pub const FAR: Instant = 90 * DAY;

/// Maximum age in seconds of an order book read that the agent may decide on.
pub const FRESH: Instant = 180;

/// Minimum trade volume for the near window to be used.
pub const THIN: u64 = 10;

/// Fraction by which the near median may be below the far median before the market counts as
/// falling.
pub const SLIDE: f64 = 0.15;

/// State of the market cache and of warframe.market requests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Market {
    /// Number of times the account's own orders have been read from warframe.market.
    pub own: u32,
    /// Since when warframe.market has not responded to any request; `None` while it responds.
    pub failing_since: Option<Instant>,
}

/// Market cache event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketEvent {
    /// The account's own orders were read and saved to the account store.
    OrdersRead,
    /// warframe.market has not responded to any request since `since`.
    Failing { since: Instant },
    /// warframe.market responds again.
    Answering,
}

/// Market state after `event`.
pub fn update(market: &Market, event: MarketEvent) -> Market {
    match event {
        MarketEvent::OrdersRead => Market {
            own: market.own + 1,
            ..*market
        },
        MarketEvent::Failing { since } => Market {
            failing_since: Some(since),
            ..*market
        },
        MarketEvent::Answering => Market {
            failing_since: None,
            ..*market
        },
    }
}

/// One time bucket of closed trades (an hour or a day), with the fields pricing uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Traded {
    pub starts: Instant,
    pub volume: u64,
    pub median: f64,
    /// Lowest price traded in the bucket.
    pub low: f64,
}

/// One point of a price curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub at: Instant,
    pub platinum: f64,
    /// Weight of the point: the number of closed trades, or of order book reads.
    pub weight: u64,
}

/// Closed trades from `from` on as price curve points, oldest first; empty buckets are skipped.
pub fn curve(traded: &[Traded], from: Instant) -> Vec<Point> {
    let mut points: Vec<Point> = traded
        .iter()
        .filter(|bucket| bucket.starts >= from && bucket.volume > 0)
        .map(|bucket| Point {
            at: bucket.starts,
            platinum: bucket.median,
            weight: bucket.volume,
        })
        .collect();

    points.sort_by_key(|point| point.at);
    points
}

/// Trade price of an item, and the data it was computed from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fair {
    pub platinum: f64,
    pub volume: u64,
    pub window: Instant,
    /// Median distance of a bucket's price from `platinum`.
    pub spread: f64,
}

/// Volume-weighted median of closed trades over the `NEAR` window, or over `FAR` when the near
/// volume is below `thin`; `None` when neither window has trades.
pub fn fair(traded: &[Traded], now: Instant, thin: u64) -> Option<Fair> {
    for window in [NEAR, FAR] {
        let within: Vec<&Traded> = traded
            .iter()
            .filter(|bucket| bucket.starts >= now - window && bucket.volume > 0)
            .collect();
        let volume: u64 = within.iter().map(|bucket| bucket.volume).sum();
        if volume == 0 || (volume < thin && window == NEAR) {
            continue;
        }
        let platinum = weighted_median(&within, volume);
        return Some(Fair {
            platinum,
            volume,
            window,
            spread: spread(&within, platinum),
        });
    }
    None
}

/// Multiple of the trade price above which the lowest ask is not taken as a real price.
pub const OUTLIER: f64 = 3.0;

/// Fraction of the median in-game ask below which an ask counts as a give-away, not a price.
pub const LONE: f64 = 0.5;

/// Minimum distinct sellers of give-away asks for the market to count as having moved to them.
pub const MOVED_SELLERS: usize = 2;

/// Minimum copies in give-away asks for the market to count as having moved to them.
pub const MOVED_COPIES: u32 = 3;

/// One in-game seller's ask: price, copies and seller name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seller<'a> {
    pub platinum: u32,
    pub quantity: u32,
    pub name: &'a str,
}

/// Lowest in-game ask, skipping give-aways below `LONE` of the median ask unless the market has
/// moved to them (`MOVED_SELLERS`, `MOVED_COPIES`); `None` without sellers.
pub fn held_ask(sellers: &[Seller<'_>]) -> Option<u32> {
    let mut prices: Vec<u32> = sellers.iter().map(|seller| seller.platinum).collect();
    prices.sort_unstable();
    let middle = f64::from(*prices.get(prices.len() / 2)?);
    let is_give_away = |seller: &&Seller<'_>| f64::from(seller.platinum) < middle * LONE;

    let give_aways: Vec<&Seller<'_>> = sellers.iter().filter(is_give_away).collect();
    let names: std::collections::HashSet<&str> =
        give_aways.iter().map(|seller| seller.name).collect();
    let copies: u32 = give_aways.iter().map(|seller| seller.quantity).sum();
    let has_moved = names.len() >= MOVED_SELLERS && copies >= MOVED_COPIES;

    sellers
        .iter()
        .filter(|seller| has_moved || !is_give_away(seller))
        .map(|seller| seller.platinum)
        .min()
}

/// Lowest in-game ask, or the trade price when the ask is above `OUTLIER` times the trade price.
pub fn credible_ask(lowest: Option<u32>, fair: Option<&Fair>) -> Option<u32> {
    match (lowest, fair) {
        (Some(ask), Some(fair)) if f64::from(ask) > fair.platinum * OUTLIER => {
            Some(fair.platinum.round().clamp(0.0, f64::from(u32::MAX)) as u32)
        }
        _ => lowest,
    }
}

/// Whether the median price of the last two days is more than `SLIDE` below that of the earlier
/// days up to 90 days back; `false` when either window has fewer than `THIN` trades.
pub fn falling(hours: &[Traded], days: &[Traded], now: Instant) -> bool {
    let near: Vec<&Traded> = match hours.is_empty() {
        true => days
            .iter()
            .filter(|bucket| bucket.starts >= now - NEAR)
            .collect(),
        false => hours
            .iter()
            .filter(|bucket| bucket.starts >= now - NEAR)
            .collect(),
    };
    let far: Vec<&Traded> = days
        .iter()
        .filter(|bucket| bucket.starts < now - NEAR && bucket.starts >= now - FAR)
        .collect();
    let volume = |buckets: &[&Traded]| buckets.iter().map(|bucket| bucket.volume).sum::<u64>();
    let (near_volume, far_volume) = (volume(&near), volume(&far));
    if near_volume < THIN || far_volume < THIN {
        return false;
    }
    weighted_median(&near, near_volume) < weighted_median(&far, far_volume) * (1.0 - SLIDE)
}

/// Buckets more than this many median deviations from the median are left out of the price.
const STRAY: f64 = 3.0;

/// Trade price of a pair over the last two days from hourly buckets, without single-trade hours
/// and buckets more than `STRAY` deviations away; from daily buckets over 90 days when the hours
/// have fewer than `THIN` trades.
pub fn realized(hours: &[Traded], days: &[Traded], now: Instant) -> Option<Fair> {
    let settled = |buckets: &[Traded], window: Instant, least: u64| -> Option<Fair> {
        let within: Vec<&Traded> = buckets
            .iter()
            .filter(|bucket| bucket.starts >= now - window && bucket.volume >= least)
            .collect();
        let volume: u64 = within.iter().map(|bucket| bucket.volume).sum();
        if volume < THIN {
            return None;
        }
        let middle = weighted_median(&within, volume);
        let deviation = spread(&within, middle);
        let kept: Vec<&Traded> = within
            .into_iter()
            .filter(|bucket| (bucket.median - middle).abs() <= STRAY * deviation)
            .collect();
        let volume: u64 = kept.iter().map(|bucket| bucket.volume).sum();
        let platinum = weighted_median(&kept, volume);
        Some(Fair {
            platinum,
            volume,
            window,
            spread: spread(&kept, platinum),
        })
    };
    settled(hours, NEAR, 2).or_else(|| settled(days, FAR, 1))
}

/// Volume-weighted median: the bucket price at which half the traded volume is at or below it.
fn weighted_median(buckets: &[&Traded], volume: u64) -> f64 {
    let mut sorted: Vec<&&Traded> = buckets.iter().collect();
    sorted.sort_by(|a, b| a.median.total_cmp(&b.median));

    let half = volume as f64 / 2.0;
    let mut seen = 0.0;
    for bucket in &sorted {
        seen += bucket.volume as f64;
        if seen >= half {
            return bucket.median;
        }
    }
    sorted
        .last()
        .map(|bucket| bucket.median)
        .unwrap_or_default()
}

/// Median distance of the bucket prices from `middle`.
fn spread(buckets: &[&Traded], middle: f64) -> f64 {
    let mut distances: Vec<f64> = buckets
        .iter()
        .map(|bucket| (bucket.median - middle).abs())
        .collect();
    distances.sort_by(f64::total_cmp);
    distances
        .get(distances.len() / 2)
        .copied()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lone_ask_far_over_what_trades_gives_way_to_the_trades() {
        let traded = Fair {
            platinum: 10.0,
            volume: 4,
            window: FAR,
            spread: 0.0,
        };

        assert_eq!(credible_ask(Some(500), Some(&traded)), Some(10));
        assert_eq!(credible_ask(Some(25), Some(&traded)), Some(25));
        assert_eq!(credible_ask(Some(500), None), Some(500));
        assert_eq!(credible_ask(None, Some(&traded)), None);
    }

    fn sellers(asks: &[(u32, u32, &'static str)]) -> Vec<Seller<'static>> {
        asks.iter()
            .map(|&(platinum, quantity, name)| Seller {
                platinum,
                quantity,
                name,
            })
            .collect()
    }

    #[test]
    fn a_lone_give_away_is_passed_over() {
        let book = sellers(&[(20, 1, "stray"), (98, 1, "a"), (109, 2, "b"), (110, 1, "c")]);
        assert_eq!(held_ask(&book), Some(98));
    }

    #[test]
    fn one_seller_with_many_copies_is_still_a_give_away() {
        let book = sellers(&[(20, 9, "bait"), (98, 1, "a"), (109, 2, "b"), (110, 1, "c")]);
        assert_eq!(held_ask(&book), Some(98));
    }

    #[test]
    fn several_sellers_with_copies_enough_move_the_market() {
        let book = sellers(&[
            (20, 2, "x"),
            (22, 1, "y"),
            (98, 1, "a"),
            (109, 2, "b"),
            (110, 1, "c"),
            (112, 1, "d"),
        ]);
        assert_eq!(held_ask(&book), Some(20));
    }

    #[test]
    fn an_ask_over_half_the_middle_is_a_price() {
        let book = sellers(&[(60, 1, "cheap"), (98, 1, "a"), (110, 1, "b")]);
        assert_eq!(held_ask(&book), Some(60));
        assert_eq!(held_ask(&sellers(&[(20, 1, "only")])), Some(20));
        assert_eq!(held_ask(&[]), None);
    }

    #[test]
    fn reading_our_own_orders_counts() {
        assert_eq!(update(&Market::default(), MarketEvent::OrdersRead).own, 1);
        let failing = update(&Market::default(), MarketEvent::Failing { since: 40 });
        assert_eq!(failing.failing_since, Some(40));
        assert_eq!(
            update(&failing, MarketEvent::OrdersRead).failing_since,
            Some(40)
        );
        assert_eq!(update(&failing, MarketEvent::Answering).failing_since, None);
    }

    #[test]
    fn the_curve_leaves_out_what_starts_before_the_period() {
        let traded = [bucket(10 * DAY, 3, 12.0), bucket(DAY, 2, 15.0)];

        let drawn = curve(&traded, NOW - 2 * DAY);

        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].at, NOW - DAY);
        assert_eq!(drawn[0].platinum, 15.0);
        assert_eq!(drawn[0].weight, 2);
    }

    #[test]
    fn a_bucket_nothing_changed_hands_in_is_not_a_point() {
        let traded = [bucket(DAY, 0, 15.0)];

        assert!(curve(&traded, 0).is_empty());
    }

    const NOW: Instant = 1_760_000_000;

    fn bucket(ago: Instant, volume: u64, median: f64) -> Traded {
        Traded {
            starts: NOW - ago,
            volume,
            median,
            low: median,
        }
    }

    #[test]
    fn nothing_traded_means_no_price_rather_than_zero() {
        assert_eq!(fair(&[], NOW, THIN), None);
        assert_eq!(fair(&[bucket(HOUR, 0, 20.0)], NOW, THIN), None);
    }

    #[test]
    fn a_busy_item_is_priced_from_the_near_window() {
        let traded = [
            bucket(HOUR, 20, 19.0),
            bucket(7_200, 30, 20.0),
            bucket(60 * DAY, 500, 70.0),
        ];
        let fair = fair(&traded, NOW, THIN).expect("there is a price");
        assert_eq!(fair.window, NEAR);
        assert_eq!(fair.volume, 50);
        assert_eq!(fair.platinum, 20.0);
    }

    #[test]
    fn a_thin_near_window_widens_to_the_far_one() {
        let traded = [
            bucket(HOUR, 2, 19.0),
            bucket(30 * DAY, 40, 12.0),
            bucket(60 * DAY, 40, 11.0),
        ];
        let fair = fair(&traded, NOW, THIN).expect("there is a price");
        assert_eq!(fair.window, FAR);
        assert_eq!(fair.volume, 82);
    }

    #[test]
    fn the_weight_is_what_changed_hands_not_the_number_of_buckets() {
        // Two thin buckets at 100 must not outvote one heavy bucket at 20.
        let traded = [
            bucket(HOUR, 1, 100.0),
            bucket(7_200, 1, 100.0),
            bucket(10_800, 40, 20.0),
        ];
        let fair = fair(&traded, NOW, THIN).expect("there is a price");
        assert_eq!(fair.platinum, 20.0);
    }

    #[test]
    fn a_single_outlier_does_not_become_the_price() {
        let traded = [
            bucket(HOUR, 10, 19.0),
            bucket(7_200, 10, 20.0),
            bucket(10_800, 1, 5_000.0),
        ];
        let fair = fair(&traded, NOW, THIN).expect("there is a price");
        assert_eq!(fair.platinum, 20.0);
    }

    #[test]
    fn buckets_older_than_the_far_window_are_left_out() {
        let traded = [bucket(FAR + DAY, 500, 70.0)];
        assert_eq!(fair(&traded, NOW, THIN), None);
    }

    #[test]
    fn the_window_the_price_came_from_is_said_out_loud() {
        let near = [bucket(HOUR, 50, 20.0)];
        let far = [bucket(40 * DAY, 50, 20.0)];
        assert_eq!(fair(&near, NOW, THIN).unwrap().window, NEAR);
        assert_eq!(fair(&far, NOW, THIN).unwrap().window, FAR);
    }

    #[test]
    fn a_median_well_under_the_months_before_is_falling() {
        let hours = [bucket(HOUR, 20, 30.0)];
        let days = [bucket(10 * DAY, 40, 50.0)];

        assert!(falling(&hours, &days, NOW));
    }

    #[test]
    fn a_small_dip_is_not_falling() {
        let hours = [bucket(HOUR, 20, 46.0)];
        let days = [bucket(10 * DAY, 40, 50.0)];

        assert!(!falling(&hours, &days, NOW));
    }

    #[test]
    fn too_few_trades_say_nothing_about_falling() {
        let hours = [bucket(HOUR, 3, 10.0)];
        let days = [bucket(10 * DAY, 40, 50.0)];

        assert!(!falling(&hours, &days, NOW));
    }

    #[test]
    fn without_hours_the_last_two_days_stand_for_them() {
        let days = [bucket(DAY, 20, 30.0), bucket(10 * DAY, 40, 50.0)];

        assert!(falling(&[], &days, NOW));
    }

    #[test]
    fn a_lone_dear_hour_does_not_move_what_a_copy_changed_hands_at() {
        let mut hours: Vec<Traded> = (1..=20).map(|ago| bucket(ago * HOUR, 2, 15.0)).collect();
        hours.push(bucket(21 * HOUR, 1, 40.0));

        let price = realized(&hours, &[], NOW).expect("a price");
        assert_eq!(price.platinum, 15.0);
    }

    #[test]
    fn a_stray_hour_is_left_out_of_the_price() {
        let mut hours: Vec<Traded> = (1..=20)
            .map(|ago| bucket(ago * HOUR, 2, 15.0 + (ago % 3) as f64))
            .collect();
        hours.push(bucket(21 * HOUR, 30, 90.0));

        let price = realized(&hours, &[], NOW).expect("a price");
        assert!(price.platinum <= 17.0, "{price:?}");
    }

    #[test]
    fn thin_hours_fall_back_to_the_days() {
        let days = [bucket(DAY, 30, 22.0)];

        let price = realized(&[bucket(HOUR, 2, 15.0)], &days, NOW).expect("a price");
        assert_eq!(price.platinum, 22.0);
    }

    #[test]
    fn the_spread_is_how_far_hours_usually_stray_from_the_price() {
        let hours: Vec<Traded> = (1..=20)
            .map(|ago| bucket(ago * HOUR, 2, [84.0, 88.0, 92.0][(ago % 3) as usize]))
            .collect();

        let price = realized(&hours, &[], NOW).expect("a price");
        assert_eq!(price.platinum, 88.0);
        assert_eq!(price.spread, 4.0);
    }
}
