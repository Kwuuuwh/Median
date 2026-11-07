use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::Duration;

use median_core::{
    FRESH, Holder, Interest, LEAST, Means, Measured, Need, Policy, Reason, SLOTS, Sight, Strategy,
    ceiling, copies, seats, seconds,
};
use median_market::{Book, Order, Side};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::account;
use crate::agent::gear::{Gear, asks_for, best, commits, overtaken_by, updated};
use crate::agent::rank_up;
use crate::market::{Variant, named_by_slug, priced_at, sighted};

/// Buy orders from reachable users, except the order `mine`.
fn rivals<'a>(book: &'a Book, mine: &'a str) -> impl Iterator<Item = &'a Order> {
    book.rows()
        .iter()
        .filter(move |row| row.side == Side::Buy && row.user.status.reachable() && row.id != mine)
}

/// Splits the agent's bids into the oldest bid on each pair, which is kept, and the newer
/// duplicates. Bids without a slug are kept.
fn duplicates(mine: Vec<account::Standing>) -> (Vec<account::Standing>, Vec<account::Standing>) {
    let mut kept: HashMap<(String, String), account::Standing> = HashMap::new();
    let mut nameless = Vec::new();
    let mut young = Vec::new();

    for order in mine {
        let Some(slug) = order.slug.clone() else {
            nameless.push(order);
            continue;
        };
        match kept.entry((slug, order.variant.clone())) {
            Entry::Vacant(seat) => {
                seat.insert(order);
            }
            Entry::Occupied(mut seat) => match order.created_at < seat.get().created_at {
                true => young.push(seat.insert(order)),
                false => young.push(order),
            },
        }
    }
    (kept.into_values().chain(nameless).collect(), young)
}

/// Withdraws the duplicate bids on each pair, keeping the oldest; returns the kept bids.
async fn fold(gear: &Gear, mine: Vec<account::Standing>) -> Vec<account::Standing> {
    let (kept, young) = duplicates(mine);
    for order in young {
        if gear.drop_order(&order.id).await {
            info!(
                order_id = order.id,
                slug = order.slug,
                platinum = order.platinum,
                "agent duplicate bid withdrawn"
            );
        }
    }
    kept
}

/// Whether a bid is at the minimum price `LEAST`, where it buys nothing.
fn leftover(order: &account::Standing) -> bool {
    order.platinum <= LEAST
}

/// Withdraws every bid at the minimum price; returns the other bids.
async fn sweep(gear: &Gear, mine: Vec<account::Standing>) -> Vec<account::Standing> {
    let (leftovers, rest): (Vec<_>, Vec<_>) = mine.into_iter().partition(leftover);
    for order in leftovers {
        if gear.drop_order(&order.id).await {
            info!(
                order_id = order.id,
                slug = order.slug,
                "agent bid at minimum price withdrawn"
            );
        }
    }
    rest
}

/// Interval between repricing passes when nothing wakes the loop earlier.
const PAUSE: Duration = Duration::from_secs(45);

/// Delay before a pass after a wake-up, so passes stay this far apart however often the loop is
/// woken. Without it, three bots outbidding each other at socket speed would use up the whole
/// spread in seconds.
const SOONEST: Duration = Duration::from_secs(5);

/// `means` with this bid's own commitment and order slot released, so the bid is evaluated
/// against the room it already takes.
fn without(means: &Means, order: &account::Standing) -> Means {
    let own = commits(order);
    Means {
        tied: means.tied.saturating_sub(own),
        free: means.free.map(|free| free.saturating_add(own)),
        orders: means.orders.saturating_sub(1),
        ..*means
    }
}

/// Visible bids whose quantity differs from what the budget allows, with the quantity each
/// should have.
fn restocks<'a>(
    mine: &'a [account::Standing],
    policy: &Policy,
    means: &Means,
) -> Vec<(&'a account::Standing, u32)> {
    mine.iter()
        .filter(|order| order.visible)
        .filter_map(|order| {
            restock(order, policy, means, order.platinum).map(|asked| (order, asked))
        })
        .collect()
}

/// Quantity a bid should have at price `entry`: what the budget ceiling allows, within the
/// policy's limit and at least one lot; `None` when the bid already has that quantity.
fn restock(order: &account::Standing, policy: &Policy, means: &Means, entry: u32) -> Option<u32> {
    let asked = copies(policy, &without(means, order), entry).max(order.per_trade.max(1));
    (order.quantity != asked).then_some(asked)
}

/// Fits the platinum the agent's bids commit into the budget: cuts quantities, hides the bids
/// that do not fit, lowest expected value first, and shows hidden ones that fit again; returns
/// whether any bid was hidden or shown.
async fn hold(gear: &Gear, mine: &[account::Standing], policy: &Policy, means: &Means) -> bool {
    for (order, asked) in restocks(mine, policy, means) {
        let change = serde_json::json!({ "quantity": asked });
        match gear.amend(&order.id, change).await {
            Ok(_) => info!(
                order_id = order.id,
                slug = order.slug,
                quantity_was = order.quantity,
                quantity = asked,
                "agent bid quantity cut to budget"
            ),
            Err(why) => warn!(
                order_id = order.id,
                error = why,
                "agent bid quantity change refused"
            ),
        }
    }

    let cap = ceiling(policy, means);
    let mut moved = false;
    let mut showing: Vec<&account::Standing> = mine.iter().filter(|o| o.visible).collect();
    let mut over = showing.iter().map(|o| commits(o)).sum::<u32>();
    showing.sort_by_key(|order| order.promised.unwrap_or(0));
    for order in showing {
        if over <= cap {
            break;
        }
        if !show(gear, order, false).await {
            continue;
        }
        moved = true;
        over = over.saturating_sub(commits(order));
        info!(
            order_id = order.id,
            slug = order.slug,
            committed = over,
            budget = cap,
            "agent bid hidden, over budget"
        );
    }

    let mut hidden: Vec<&account::Standing> = mine.iter().filter(|o| !o.visible).collect();
    hidden.sort_by_key(|order| Reverse(order.promised.unwrap_or(0)));
    for order in hidden {
        let asks = asks_for(order);
        if over.saturating_add(asks) > cap || !show(gear, order, true).await {
            continue;
        }
        moved = true;
        over = over.saturating_add(asks);
        info!(
            order_id = order.id,
            slug = order.slug,
            committed = over,
            budget = cap,
            "agent bid shown, within budget"
        );
    }
    moved
}

/// Shows or hides one of the agent's bids; returns whether the market accepted the change.
async fn show(gear: &Gear, order: &account::Standing, visible: bool) -> bool {
    match gear
        .amend(&order.id, serde_json::json!({ "visible": visible }))
        .await
    {
        Ok(_) => true,
        Err(why) => {
            warn!(
                order_id = order.id,
                visible,
                error = why,
                "agent bid visibility change refused"
            );
            false
        }
    }
}

/// Counts one outcome for the summary line of the repricing pass.
fn tally(outcomes: &mut HashMap<&'static str, u32>, word: &'static str) {
    *outcomes.entry(word).or_insert(0) += 1;
}

/// Most bids withdrawn per pass to free order slots. The slots are shared with the user, but
/// withdrawing a dozen old bids because the user listed many orders in one minute is worse than
/// freeing the rest over later passes.
const GIVEN: usize = 3;

/// Refused withdrawals in a row after which the pass stops withdrawing.
const REFUSALS: u32 = 2;

/// The agent's bids in withdrawal order: hidden bids first, since their expected value is zero
/// while hidden, then lowest expected value, then lowest traded volume of the pair.
fn giving_order<'a>(
    mut ours: Vec<&'a account::Standing>,
    moved: &HashMap<(String, String), u32>,
) -> Vec<&'a account::Standing> {
    ours.sort_by_key(|order| {
        let pair = (
            order.slug.clone().unwrap_or_default(),
            order.variant.clone(),
        );
        (
            order.visible,
            order.promised.unwrap_or(0),
            moved.get(&pair).copied().unwrap_or(0),
        )
    });
    ours
}

/// Withdraws agent bids, at most `GIVEN` per pass, while the order slots are full; returns
/// whether any was withdrawn, so the pass reads the open orders again.
///
/// The slots are shared: the market's 100 slots count every open order of the account, those
/// the user placed by hand included. When the user fills them, the agent withdraws its own bids
/// until a slot is free, instead of rejecting every opportunity for lack of a slot.
///
/// Withdrawal order is `giving_order`: hidden bids, then lowest expected value, then lowest
/// traded volume. Sell orders are never withdrawn, since each is for a copy already paid for,
/// and no order the user placed is touched.
async fn give_way(gear: &Gear, standing: &[account::Standing]) -> bool {
    // Two limits apply. The market allows `SLOTS` open orders per account in all, and the
    // agent may use only its own share of them: past that every opportunity is rejected as
    // `too-many-orders`, repricing a bid already open included.
    //
    // The agent's share is not a setting. It is `SLOTS` less the user's own open orders, less
    // room for them to list more, so every order the user lists shrinks the agent's share by
    // one and the lowest-paying agent bid is withdrawn for it. Both limits are checked here: if
    // the share were checked only when placing new bids, the agent would stay over it once the
    // user listed more.
    let slots = SLOTS as usize;
    let mut slots_used = standing.len();
    let mut agent_slots_used = standing.iter().filter(|order| order.by_agent).count();
    let agent_slots = seats((slots_used - agent_slots_used) as u32) as usize;
    if slots_used < slots && agent_slots_used <= agent_slots {
        return false;
    }

    let ours: Vec<&account::Standing> = standing
        .iter()
        .filter(|order| order.by_agent && !order.selling)
        .collect();
    if ours.is_empty() {
        // The agent has no open bid, only sell orders for copies already paid for. Those are
        // never withdrawn; the log line records why the slots stay full.
        info!(
            slots_used,
            agent_slots_used, slots, agent_slots, "order slots full, no agent bid to withdraw"
        );
        return false;
    }

    // Traded volume breaks ties between bids of equal expected value, so it is read only for
    // the agent's bids, and only when a slot has to be freed.
    let now = seconds();
    let mut moved: HashMap<(String, String), u32> = HashMap::new();
    for order in &ours {
        let Some(slug) = order.slug.clone() else {
            continue;
        };
        let pair = (slug.clone(), order.variant.clone());
        if moved.contains_key(&pair) {
            continue;
        }
        let of = Variant::parse(&order.variant);
        let traded = priced_at(&gear.market_cache, &slug, &of, now)
            .await
            .ok()
            .and_then(|reading| reading.traded)
            .unwrap_or(0);
        moved.insert(pair, traded);
    }

    let ours = giving_order(ours, &moved);

    let mut given = 0;
    let mut refused = 0;
    for order in ours {
        if (slots_used < slots && agent_slots_used <= agent_slots) || given >= GIVEN {
            break;
        }
        // A single refusal can be one stale order id. Two in a row point to the session or the
        // market, and trying the remaining bids would only repeat the same warning in the log.
        if !gear.drop_order(&order.id).await {
            refused += 1;
            if refused >= REFUSALS {
                break;
            }
            continue;
        }
        slots_used -= 1;
        agent_slots_used -= 1;
        given += 1;
        info!(
            order_id = order.id,
            slug = order.slug,
            expected_value = order.promised,
            slots_used,
            agent_slots_used,
            slots,
            agent_slots,
            "agent bid withdrawn to free an order slot"
        );
    }
    given > 0
}

/// Withdraws a bid that is no longer profitable, logging `why`.
async fn withdraw(gear: &Gear, order: &account::Standing, slug: &str, why: &str) {
    if gear.drop_order(&order.id).await {
        info!(
            order_id = order.id,
            slug,
            reason = why,
            "agent bid withdrawn, no longer profitable"
        );
    }
}

/// Reprices the agent's bids and withdraws unprofitable ones while the agent runs.
pub async fn tend(gear: Arc<Gear>, running: Arc<Mutex<bool>>) -> Result<(), String> {
    let nudge = Arc::clone(&gear.nudge);
    let mut presence = gear.presence.watch();

    loop {
        if *running.lock().await {
            watch(&gear).await;
        }
        // The next pass runs after `PAUSE`, after a nudge from a book or order that affects the
        // agent's bids, or when the account goes in or out of game.
        tokio::select! {
            () = sleep(PAUSE) => {}
            () = nudge.notified() => sleep(SOONEST).await,
            Ok(()) = presence.changed() => {}
        }
    }
}

async fn watch(gear: &Gear) {
    let policy = gear.policy();
    let mut standing = gear.standing_now().await;
    // Free an order slot before evaluating anything: full slots reject every opportunity,
    // whatever it is worth.
    if give_way(gear, &standing).await {
        standing = gear.standing_now().await;
    }

    let mine: Vec<account::Standing> = standing
        .iter()
        .filter(|order| order.by_agent && !order.selling)
        .cloned()
        .collect();
    if mine.is_empty() {
        return;
    }
    let mine = sweep(gear, fold(gear, mine).await).await;
    let mine_count = mine.len();

    let mut means = gear.means(&standing);
    let mut mine = mine;
    // Hiding or showing bids changes the free platinum and the bids to reprice.
    if hold(gear, &mine, &policy, &means).await {
        let standing = gear.standing_now().await;
        means = gear.means(&standing);
        mine = standing
            .into_iter()
            .filter(|order| order.by_agent && !order.selling)
            .collect();
    }
    // Nobody can sell to a hidden bid, so it is neither repriced nor touched until it is shown.
    let mine: Vec<account::Standing> = mine.into_iter().filter(|order| order.visible).collect();
    let now = seconds();

    let mut named = Vec::with_capacity(mine.len());
    for order in mine {
        let slug = match order.slug.clone() {
            Some(slug) => Some(slug),
            None => gear
                .market_cache
                .slug_of(order.item_id.clone())
                .await
                .ok()
                .flatten(),
        };
        if let Some(slug) = slug {
            named.push((order, slug));
        }
    }

    gear.needs.declare(
        Holder::Agent,
        named
            .iter()
            .map(|(order, slug)| Need {
                slug: slug.clone(),
                variant: order.variant.clone(),
                interest: Interest::Committed,
            })
            .collect(),
    );

    let mut outcomes: HashMap<&'static str, u32> = HashMap::new();

    let watched = named.len();

    for (order, slug) in named {
        let of = Variant::parse(&order.variant);

        // Only the scheduler sends market requests. A bid whose book is older than `FRESH`
        // waits for a later pass; the need declared above makes the scheduler fetch a new book.
        let kept = match gear.books.look(&slug) {
            Some((taken, book)) if now - taken < FRESH => book,
            _ => {
                tally(&mut outcomes, "stale-book");
                continue;
            }
        };
        // The book holds every variant of the item; the bid is compared only with its own
        // variant.
        let book = Book::from(kept.of(&of.selector()).cloned().collect::<Vec<_>>());
        let Ok(sighted) = sighted(&gear.market_cache, &slug, &of, now).await else {
            tally(&mut outcomes, "no-reading");
            continue;
        };

        // Rival bidders on this pair, and how many changed their price between two book reads.
        // Such a change is taken as a bot watching the market; a human does not reprice that
        // fast.
        let seen: Vec<(String, String)> = rivals(&book, &order.id)
            .map(|row| (row.user.name.clone(), row.updated_at.clone()))
            .collect();
        let movers = gear
            .market_cache
            .keep_rivals(&slug, &order.variant, seen)
            .await
            .unwrap_or(0);

        let item = named_by_slug(&gear.catalog, "en", &slug)
            .await
            .ok()
            .flatten()
            .map(|known| known.item)
            .unwrap_or_default();
        // Evaluated as if the slot were empty: the slot holds this very bid, so its slot and
        // its commitment are released before the policy check.
        let room = without(&means, &order);
        let ranked = rank_up::ranked_resale(gear, &slug, &of, now).await;
        let top = best(&book, Side::Buy, &order.id);
        let sight = Sight {
            // No other in-game bid: nothing to match, so the price stays.
            lead: top.unwrap_or(order.platinum),
            resale: ranked.as_ref().map(|(seen, _)| seen.resale()),
            ..sighted.sight(&item, now)
        };
        let rates = gear.rates();
        let quoted = Measured { rates: &rates }.quote(&policy, &room, &sight);
        let price = match &quoted {
            Ok(quote) => quote.platinum,
            Err(_) => order.platinum,
        };
        // At the top price, but a rival at the same price was updated later and is listed above.
        let ours = updated(&order, &book);
        let passed = match top == Some(price) {
            true => overtaken_by(&book, Side::Buy, &order.id, price, &ours),
            false => None,
        };

        let stock = restock(&order, &policy, &means, price);
        match quoted {
            Ok(_) if price != order.platinum || stock.is_some() => {
                let mut change = serde_json::Map::new();
                if price != order.platinum {
                    change.insert("platinum".into(), price.into());
                }
                if let Some(stock) = stock {
                    change.insert("quantity".into(), stock.into());
                }
                let raised = price > order.platinum;
                let direction = match raised {
                    true => "raised",
                    false => "lowered",
                };
                tally(&mut outcomes, direction);
                match gear.amend(&order.id, change.into()).await {
                    Ok(_) => info!(
                        order_id = order.id,
                        slug,
                        platinum_was = order.platinum,
                        platinum = price,
                        direction,
                        rivals_moved = movers,
                        "agent bid repriced"
                    ),
                    Err(why) => {
                        tally(&mut outcomes, "would-not-move");
                        warn!(
                            order_id = order.id,
                            slug,
                            error = why,
                            "agent bid reprice refused"
                        )
                    }
                }
            }
            Ok(_) => match passed {
                Some(beat) => tally(&mut outcomes, gear.touch(&order, beat).await.as_code()),
                None => tally(&mut outcomes, "stands"),
            },
            // Only `hold` decides which bids are visible, at each bid's current price. A price
            // change the budget cannot cover is skipped and the bid keeps its price: hiding it
            // here would only have `hold` show it again next pass, toggling it every pass.
            Err(Reason::NoCapital) => tally(&mut outcomes, "no-capital"),
            // Only a reason about the opportunity itself withdraws a bid. Full order slots do
            // not mean the bid is not worth keeping.
            Err(
                reason @ (Reason::BelowFloor
                | Reason::TooDear
                | Reason::ThinVolume
                | Reason::Blacklisted
                | Reason::NotWhitelisted),
            ) => {
                tally(&mut outcomes, reason.as_code());
                // Logs the repricing inputs next to the expected value from placement: when the
                // two evaluations disagree, a bid is withdrawn moments after it was placed.
                info!(
                    order_id = order.id,
                    slug,
                    platinum = order.platinum,
                    target_platinum = sight.lead,
                    top_bid = sight.top_bid,
                    expected_value = order.promised,
                    reason = reason.as_code(),
                    "agent bid judged unprofitable"
                );
                withdraw(gear, &order, &slug, reason.as_code()).await;
            }
            // Other refusals are counted, so the summary line distinguishes a pass in which
            // every bid was refused from a pass with nothing to do.
            Err(reason) => tally(&mut outcomes, reason.as_code()),
        }
    }

    // One summary line per pass, whatever happened, so a pass with nothing to do can be told
    // apart from a pass in which every change was refused.
    let mut roll: Vec<(&str, u32)> = outcomes.into_iter().collect();
    roll.sort_unstable();
    let words: Vec<String> = roll
        .iter()
        .map(|(word, count)| format!("{word}={count}"))
        .collect();
    debug!(
        pairs_watched = watched,
        bids_open = mine_count,
        outcomes = words.join(" "),
        "agent bid round finished"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(id: &str, platinum: u32, born: &str) -> String {
        format!(
            r#"{{"id":"{id}","type":"buy","platinum":{platinum},"quantity":1,
                "createdAt":"{born}","updatedAt":"{born}",
                "user":{{"ingameName":"{id}","status":"ingame"}}}}"#
        )
    }

    /// A bid from an unreachable user, whose status is offline.
    fn away(id: &str, platinum: u32, born: &str) -> String {
        format!(
            r#"{{"id":"{id}","type":"buy","platinum":{platinum},"quantity":1,
                "createdAt":"{born}","updatedAt":"{born}",
                "user":{{"ingameName":"{id}","status":"offline"}}}}"#
        )
    }

    /// A full book built from JSON order rows.
    fn whole(rows: &[String]) -> Book {
        Book::from(
            serde_json::from_str::<Vec<Order>>(&format!("[{}]", rows.join(",")))
                .expect("orders parse"),
        )
    }

    const OURS: &str = "2026-08-01T00:00:00Z";

    #[test]
    fn standing_alone_leaves_no_top_to_match() {
        let empty = whole(&[bid("mine", 20, OURS)]);

        assert_eq!(best(&empty, Side::Buy, "mine"), None);
    }

    #[test]
    fn the_top_is_the_dearest_reachable_bid_besides_ours() {
        let held = whole(&[
            bid("mine", 90, OURS),
            bid("late", 30, "2026-08-20T00:00:00Z"),
            bid("early", 28, "2026-07-01T00:00:00Z"),
        ]);

        assert_eq!(best(&held, Side::Buy, "mine"), Some(30));
    }

    #[test]
    fn rivals_nobody_can_reach_do_not_take_a_place() {
        let held = whole(&[
            bid("mine", 20, OURS),
            away("asleep", 360, "2026-07-01T00:00:00Z"),
            away("also", 300, "2026-07-01T00:00:00Z"),
            bid("awake", 66, "2026-08-20T00:00:00Z"),
        ]);

        // The two unreachable bids are far above the agent's and are ignored; the price to
        // match is that of the only reachable rival.
        assert_eq!(best(&held, Side::Buy, "mine"), Some(66));
    }

    #[test]
    fn the_edges_alone_give_the_top_the_whole_book_would() {
        let edges = whole(&[
            bid("mine", 41, OURS),
            bid("one", 60, "2026-08-20T00:00:00Z"),
            bid("two", 55, "2026-08-20T00:00:00Z"),
        ]);

        assert_eq!(best(&edges, Side::Buy, "mine"), Some(60));
    }

    #[test]
    fn a_rival_level_and_updated_after_us_stands_above() {
        let held = whole(&[
            bid("mine", 30, OURS),
            bid("late", 30, "2026-08-20T00:00:00Z"),
            bid("later", 30, "2026-08-21T00:00:00Z"),
        ]);

        assert_eq!(
            overtaken_by(&held, Side::Buy, "mine", 30, OURS),
            Some("2026-08-21T00:00:00Z"),
            "the newest of them is the one to beat"
        );
    }

    #[test]
    fn a_rival_updated_before_us_stands_below_whatever_its_age() {
        let held = whole(&[
            bid("mine", 30, OURS),
            bid("early", 30, "2026-07-01T00:00:00Z"),
        ]);

        assert_eq!(overtaken_by(&held, Side::Buy, "mine", 30, OURS), None);
    }

    #[test]
    fn only_a_reachable_rival_at_our_very_price_takes_our_place() {
        let held = whole(&[
            bid("mine", 30, OURS),
            away("asleep", 30, "2026-08-20T00:00:00Z"),
            bid("dearer", 31, "2026-08-20T00:00:00Z"),
        ]);

        assert_eq!(overtaken_by(&held, Side::Buy, "mine", 30, OURS), None);
    }

    #[test]
    fn our_update_is_the_later_of_the_history_and_the_book() {
        let order = ours("mine", "gram_prime_set", 30, OURS);
        let touched = whole(&[bid("mine", 30, "2026-08-02T00:00:00Z")]);
        let behind = whole(&[bid("mine", 30, "2026-07-01T00:00:00Z")]);

        assert_eq!(updated(&order, &touched), "2026-08-02T00:00:00Z");
        assert_eq!(updated(&order, &behind), OURS);
    }

    fn ours(id: &str, slug: &str, platinum: u32, born: &str) -> account::Standing {
        account::Standing {
            id: id.into(),
            item_id: "y".into(),
            slug: Some(slug.into()),
            variant: String::new(),
            selling: false,
            platinum,
            quantity: 1,
            per_trade: 1,
            visible: true,
            by_agent: true,
            promised: None,
            created_at: born.into(),
            updated_at: born.into(),
        }
    }

    fn means(free: u32, tied: u32) -> Means {
        Means {
            held: None,
            free: Some(free),
            tied,
            exchanges: Some(10),
            orders: 4,
            hand: 0,
        }
    }

    fn policy() -> Policy {
        Policy {
            most_per_order: 2,
            most_tied: 500,
            ..Policy::default()
        }
    }

    #[test]
    fn a_bid_that_already_stands_is_judged_without_its_own_seat() {
        let policy = Policy {
            most_tied: 500,
            ..Policy::default()
        };
        let full = Means {
            orders: median_core::seats(0),
            ..means(300, 0)
        };
        let standing = ours("bid", "gram_prime_set", 30, OURS);
        let chance = median_core::Chance {
            item: "/Lotus/Gram".into(),
            entry: 30,
            worth: Some(60),
            volume: Some(50),
            per_trade: 1,
            copies: 1,
            listed: false,
            falling: false,
        };

        // Every agent slot is taken, so a bid that already holds one must not be refused its
        // own slot.
        assert!(matches!(
            median_core::Permit::issue(&policy, &chance, &full),
            Err(median_core::Reason::TooManyOrders)
        ));
        assert!(median_core::Permit::issue(&policy, &chance, &without(&full, &standing)).is_ok());
    }

    /// Every order the user places reduces the agent's share of order slots by one, so the share
    /// follows the user's orders instead of being a fixed number the user must adjust.
    #[test]
    fn what_the_person_puts_up_bends_the_agents_share_down() {
        assert_eq!(
            median_core::seats(12),
            median_core::seats(0).saturating_sub(12)
        );
        assert!(median_core::seats(0) < median_core::SLOTS);
    }

    /// When the user's orders fill all 100 market slots, the agent's share is zero, not a
    /// share it can never use.
    #[test]
    fn a_slate_the_person_filled_leaves_the_agent_nothing() {
        assert_eq!(median_core::seats(median_core::SLOTS), 0);
    }

    #[test]
    fn a_bid_asks_for_what_is_left_once_its_own_room_is_given_back() {
        let mut bid = ours("bid", "gram_prime_set", 100, OURS);
        bid.quantity = 2;

        // Of the 500 platinum budget, 200 is this bid's own commitment and the rest is free.
        assert_eq!(restock(&bid, &policy(), &means(300, 200), 100), None);
    }

    #[test]
    fn a_bid_is_cut_to_what_the_platinum_covers() {
        let mut bid = ours("bid", "gram_prime_set", 100, OURS);
        bid.quantity = 2;
        let tight = Policy {
            most_tied: 150,
            ..policy()
        };

        assert_eq!(restock(&bid, &tight, &means(0, 200), 100), Some(1));
    }

    #[test]
    fn what_a_bid_commits_is_every_copy_it_asks_for() {
        let mut bid = ours("bid", "gram_prime_set", 40, OURS);
        bid.quantity = 2;

        assert_eq!(commits(&bid), 80);
    }

    #[test]
    fn a_bid_taking_a_lot_commits_the_whole_lot() {
        let mut bid = ours("bid", "arcane_energize", 40, OURS);
        bid.quantity = 6;
        bid.per_trade = 6;

        assert_eq!(commits(&bid), 240);
    }

    #[test]
    fn giving_a_bids_own_room_back_leaves_the_ceiling_where_it_was() {
        let mut bid = ours("bid", "gram_prime_set", 60, OURS);
        bid.quantity = 2;
        let purse = Policy {
            tied_from_purse: true,
            ..policy()
        };
        let held = means(80, 120);

        assert_eq!(ceiling(&purse, &held), 200);
        assert_eq!(
            ceiling(&purse, &without(&held, &bid)),
            200,
            "the purse is the purse"
        );
        assert_eq!(
            without(&held, &bid).tied,
            0,
            "every copy the bid asks for comes back"
        );
    }

    /// An agent bid with its price and expected value.
    fn ranked(id: &str, slug: &str, platinum: u32, promised: u32) -> account::Standing {
        let mut order = ours(id, slug, platinum, OURS);
        order.promised = Some(promised);
        order
    }

    #[test]
    fn the_bid_that_promises_least_goes_first() {
        let rich = ranked("rich", "bite", 40, 30);
        let poor = ranked("poor", "gram_prime_set", 40, 4);

        let order = giving_order(vec![&rich, &poor], &HashMap::new());

        assert_eq!(order[0].id, "poor");
    }

    #[test]
    fn among_equal_promises_the_one_the_market_moves_least_goes_first() {
        let brisk = ranked("brisk", "bite", 40, 5);
        let slow = ranked("slow", "gram_prime_set", 40, 5);
        let moved = HashMap::from([
            (("bite".to_owned(), String::new()), 90),
            (("gram_prime_set".to_owned(), String::new()), 2),
        ]);

        let order = giving_order(vec![&brisk, &slow], &moved);

        assert_eq!(order[0].id, "slow", "two copies a day is the slot to lose");
        assert_eq!(order[1].id, "brisk");
    }

    #[test]
    fn a_hidden_bid_gives_up_its_slot_before_any_showing_one() {
        let showing = ranked("showing", "bite", 40, 1);
        let mut hidden = ranked("hidden", "gram_prime_set", 40, 30);
        hidden.visible = false;

        let order = giving_order(vec![&showing, &hidden], &HashMap::new());

        assert_eq!(
            order[0].id, "hidden",
            "it promises nothing while it is hidden"
        );
    }

    #[test]
    fn a_hidden_bid_ties_up_no_platinum_but_still_asks_for_its_copies() {
        let mut bid = ours("bid", "gram_prime_set", 40, OURS);
        bid.quantity = 2;
        bid.visible = false;

        assert_eq!(commits(&bid), 0);
        assert_eq!(asks_for(&bid), 80);
    }

    #[test]
    fn a_bid_at_the_least_price_is_a_leftover() {
        assert!(leftover(&ours("bench", "gram_prime_set", 1, OURS)));
        assert!(!leftover(&ours("live", "gram_prime_set", 2, OURS)));
    }

    #[test]
    fn a_second_bid_on_a_pair_is_the_young_one_whatever_it_costs() {
        let (kept, young) = duplicates(vec![
            ours("young", "ballistica_prime_set", 112, "2026-08-27T21:39:17Z"),
            ours("old", "ballistica_prime_set", 111, "2026-08-27T21:39:14Z"),
        ]);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "old");
        assert_eq!(young.len(), 1);
        assert_eq!(young[0].id, "young");
    }

    #[test]
    fn one_bid_on_each_of_two_pairs_is_not_a_duplicate() {
        let (kept, young) = duplicates(vec![
            ours("a", "ballistica_prime_set", 112, "2026-08-27T21:39:17Z"),
            ours("b", "arcane_energize", 108, "2026-08-27T21:39:14Z"),
        ]);

        assert_eq!(kept.len(), 2);
        assert!(young.is_empty());
    }

    #[test]
    fn two_variants_of_one_item_are_two_pairs() {
        let mut bare = ours("bare", "primed_flow", 40, "2026-08-27T21:39:14Z");
        bare.variant = "mod_rank=0".into();
        let mut maxed = ours("maxed", "primed_flow", 90, "2026-08-27T21:39:17Z");
        maxed.variant = "mod_rank=10".into();

        let (kept, young) = duplicates(vec![bare, maxed]);

        assert_eq!(kept.len(), 2);
        assert!(young.is_empty());
    }

    #[test]
    fn nobody_bidding_leaves_no_top() {
        assert_eq!(best(&Book::from(Vec::new()), Side::Buy, "mine"), None);
    }
}
