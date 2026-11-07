use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use median_core::{
    DAY, FRESH, Fair, Holder, Instant, Interest, LEAST, Need, Permit, Policy, Seller, held_ask,
    realized, seconds,
};
use median_ipc::SetRow;
use median_market::{Book, Order, Placing, Session, Side};
use tokio::sync::{Mutex, Notify};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::account::{self, Exchange, Position};
use crate::agent::gear::{Gear, best, overtaken_by, updated};
use crate::catalog;
use crate::market::{self, Priority, Token, Variant, lots_allowed};
use crate::state_sink::Shown;

const PAUSE: Duration = Duration::from_secs(90);

/// Places and reprices sell orders for the account's open positions while the agent runs: every
/// `PAUSE`, right after a buy, and when the account goes in or out of game.
pub async fn tend(
    gear: Arc<Gear>,
    running: Arc<Mutex<bool>>,
    bought: Arc<Notify>,
) -> Result<(), String> {
    let mut presence = gear.presence.watch();
    loop {
        if *running.lock().await {
            offer(&gear).await;
        }
        tokio::select! {
            () = sleep(PAUSE) => {}
            () = bought.notified() => {}
            Ok(()) = presence.changed() => {}
        }
    }
}

async fn offer(gear: &Gear) {
    let held = match gear.history.open_positions().await {
        Ok(held) => held,
        Err(why) => {
            warn!(error = %why, "holdings read failed");
            return;
        }
    };
    if held.is_empty() {
        return;
    }

    let standing = gear.standing_now().await;
    let policy = gear.policy();
    let now = seconds();
    let shown = gear.holdings.read();
    let moved = since_reading(gear).await;
    let held = match assemble(gear, &held, &shown, &moved, &policy, now).await {
        true => match gear.history.open_positions().await {
            Ok(held) => held,
            Err(why) => {
                warn!(error = %why, "holdings read failed");
                return;
            }
        },
        false => held,
    };

    // A set is held as its parts; the catalog lists the parts, as it does for the inventory.
    let sets: HashMap<String, SetRow> = match held.iter().any(|p| p.slug.ends_with(SET)) {
        true => gear
            .catalog
            .read(move |connection| catalog::sets(connection, "en"))
            .await
            .inspect_err(|why| warn!(error = %why, "set parts read failed"))
            .unwrap_or_default()
            .into_iter()
            .map(|set| (set.slug.clone(), set))
            .collect(),
        false => HashMap::new(),
    };

    // Positions for sale need fresh prices: a wrong price sells a copy below its value.
    gear.needs.declare(
        Holder::Offers,
        held.iter()
            .map(|position| Need {
                slug: position.slug.clone(),
                variant: position.variant.clone(),
                interest: Interest::Committed,
            })
            .collect(),
    );

    for position in held {
        let mine = standing.iter().find(|order| {
            order.selling
                && order.slug.as_deref() == Some(&position.slug)
                && order.variant == position.variant
        });

        if policy.rank_up
            && Variant::parse(&position.variant).rank == Some(0)
            && ripen(gear, &position, &shown, &policy, now).await != Ripening::AsIs
        {
            continue;
        }

        // A position records what was bought; the inventory dump records what the account
        // actually has. Copies equipped in a build belong to the user, and missing copies were
        // sold, ranked up or spent, so neither is offered, and a sell order already open for
        // them is withdrawn. Before the first dump is read, the position is left as it is.
        let stock = stock(&shown, &position, sets.get(&position.slug), &moved);
        if stock == Some(0) {
            if let Some(order) = mine
                && gear.drop_order(&order.id).await
            {
                info!(
                    slug = position.slug,
                    variant = position.variant,
                    "agent sell order withdrawn, no free copy"
                );
            }
            continue;
        }

        let variant = Variant::parse(&position.variant);
        let floor = floor(&position, &policy, now);

        // The full book, because the market cache's cheapest seller includes the agent's own
        // sell order, and pricing from it would match the agent's own price. The scheduler
        // fetches the book; when it is stale, the cached summary is used instead.
        let fresh = match gear.books.look(&position.slug) {
            // The book holds every variant of the item; the copy is priced against its own.
            Some((taken, kept)) if now - taken < FRESH => Some(Book::from(
                kept.of(&variant.selector()).cloned().collect::<Vec<_>>(),
            )),
            _ => None,
        };
        let asking = match &fresh {
            Some(book) => stand(book, mine.map(|order| order.id.as_str()), floor),
            None => {
                let market = match gear.market_cache.look(&position.slug, &variant).await {
                    Ok(Some(look)) => look.held_ask(),
                    Ok(None) => None,
                    Err(why) => {
                        warn!(error = %why, slug = position.slug, "market cache read failed");
                        None
                    }
                };
                asking(market, floor)
            }
        };
        let Some(asking) = asking else {
            continue;
        };
        let traded = market::sighted(&gear.market_cache, &position.slug, &variant, now)
            .await
            .ok()
            .and_then(|seen| realized(&seen.hours, &seen.days, now));
        let asking = bounded(
            asking,
            traded.as_ref(),
            is_fresh(&position, &policy, now),
            floor,
        );
        let Ok(permit) = Permit::to_sell(&policy, &position.item) else {
            continue;
        };

        let stock = stock.unwrap_or(position.quantity).min(position.quantity);
        match mine {
            None => match put(gear, &position, &permit, asking, stock).await {
                Ok(id) => info!(
                    slug = position.slug,
                    platinum = asking,
                    order_id = id,
                    "agent sell order placed"
                ),
                Err(why) => {
                    warn!(slug = position.slug, error = %why, "agent sell order placement failed")
                }
            },
            Some(order) => {
                if let Some(change) = amendment(order, stock, asking) {
                    match gear.amend(&order.id, change).await {
                        Ok(_) => info!(
                            slug = position.slug,
                            platinum = asking,
                            "agent sell order repriced"
                        ),
                        Err(why) => {
                            warn!(slug = position.slug, error = %why, "agent sell order reprice failed")
                        }
                    }
                } else if let Some(book) = &fresh
                    && best(book, Side::Sell, &order.id) == Some(order.platinum)
                    && let Some(beat) = overtaken_by(
                        book,
                        Side::Sell,
                        &order.id,
                        order.platinum,
                        &updated(order, book),
                    )
                {
                    // At the lowest ask, with a rival at that price listed above.
                    gear.touch(order, beat).await;
                }
            }
        }
    }
}

/// Sell orders in the book, except the order `mine`.
fn rivals<'a>(book: &'a Book, mine: Option<&'a str>) -> impl Iterator<Item = &'a Order> {
    book.rows()
        .iter()
        .filter(move |row| row.side == Side::Sell && Some(row.id.as_str()) != mine)
}

/// Asking price for a copy: the `held_ask` of reachable rivals (of all rivals when none is
/// reachable), never under `floor` or `LEAST`; `None` without rivals.
fn stand(book: &Book, mine: Option<&str>, floor: u32) -> Option<u32> {
    let sellers = |reachable_only: bool| -> Vec<Seller<'_>> {
        rivals(book, mine)
            .filter(|row| !reachable_only || row.user.status.reachable())
            .map(|row| Seller {
                platinum: row.platinum,
                quantity: row.quantity,
                name: &row.user.name,
            })
            .collect()
    };
    let least = held_ask(&sellers(true)).or_else(|| held_ask(&sellers(false)))?;
    Some(least.max(floor).max(LEAST))
}

/// State of a position bought to be ranked up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ripening {
    /// Sold at its current rank: not a mod to rank up, or still unranked after `sale_days`.
    AsIs,
    /// Waiting for the user to rank it up.
    Waiting,
    /// Ranked up and moved to the ranked position.
    Ranked,
}

/// Moves the copies of a rank-0 position that the user ranked up since the purchase to the
/// max-rank position. Ranked copies the account already had are excluded by the count recorded
/// at the first check (`ranked_before`).
async fn ripen(
    gear: &Gear,
    position: &Position,
    shown: &Shown,
    policy: &Policy,
    now: Instant,
) -> Ripening {
    let Ok(Some((_, top))) = gear.market_cache.fused(&position.slug).await else {
        return Ripening::AsIs;
    };
    let ranked = Variant {
        rank: Some(top),
        ..Variant::parse(&position.variant)
    }
    .key();
    let Some(held) = market::free(&shown.inventory, &position.item, &ranked) else {
        return Ripening::Waiting;
    };
    let Some(before) = position.ranked_before else {
        if let Err(why) = gear
            .history
            .mark_ranked(position.slug.clone(), position.variant.clone(), held)
            .await
        {
            warn!(error = %why, slug = position.slug, "ranked copy count not recorded");
        }
        return Ripening::Waiting;
    };
    let fused = held.saturating_sub(before).min(position.quantity);
    if fused == 0 {
        return match now - position.opened < Instant::from(policy.sale_days) * DAY {
            true => Ripening::Waiting,
            false => Ripening::AsIs,
        };
    }
    let conversion = account::Conversion {
        from: vec![(position.slug.clone(), position.variant.clone(), fused)],
        into_slug: position.slug.clone(),
        into_variant: ranked,
        into_item: position.item.clone(),
        into_quantity: fused,
    };
    if let Err(why) = gear.history.convert_positions(conversion).await {
        warn!(error = %why, slug = position.slug, "ranked copies not moved to ranked position");
        return Ripening::Waiting;
    }
    if let Err(why) = gear
        .history
        .mark_ranked(
            position.slug.clone(),
            position.variant.clone(),
            before + fused,
        )
        .await
    {
        warn!(error = %why, slug = position.slug, "ranked copy count not recorded");
    }
    info!(
        slug = position.slug,
        ranked = fused,
        "ranked copies put on sale"
    );
    Ripening::Ranked
}

/// Slug suffix of a full set on warframe.market.
const SET: &str = "_set";

/// Converts held parts into set positions when a full set sells for at least the parts' total
/// price less the profit floor of each extra trade that selling the parts one by one would
/// take; returns whether any set was assembled.
async fn assemble(
    gear: &Gear,
    held: &[Position],
    shown: &Shown,
    moved: &[Exchange],
    policy: &Policy,
    now: Instant,
) -> bool {
    let items: Vec<String> = held
        .iter()
        .filter(|position| position.variant.is_empty() && !position.slug.ends_with(SET))
        .map(|position| position.item.clone())
        .collect();
    let parts = match gear
        .catalog
        .read(move |connection| catalog::kit(connection, "en", &items))
        .await
    {
        Ok(parts) => parts,
        Err(why) => {
            warn!(error = %why, "sets of held parts read failed");
            return false;
        }
    };
    let mut sets: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
    for part in parts {
        sets.entry(part.slug)
            .or_default()
            .insert(part.item, part.per_set);
    }

    let mut assembled = false;
    for (set, needs) in sets {
        let Some(whole) = whole_sets(held, &needs, shown, moved) else {
            continue;
        };
        let price = |slug: String| async move {
            market::sighted(&gear.market_cache, &slug, &Variant::default(), now)
                .await
                .ok()
                .and_then(|seen| realized(&seen.hours, &seen.days, now))
                .map(|fair| fair.platinum)
        };
        let Some(set_price) = price(set.clone()).await else {
            continue;
        };
        let mut apart = 0.0;
        let mut exchanges = 0;
        let mut from = Vec::new();
        for (item, per_set) in &needs {
            let Some(position) = held.iter().find(|position| &position.item == item) else {
                continue;
            };
            apart += price(position.slug.clone()).await.unwrap_or(0.0) * f64::from(*per_set);
            exchanges += per_set;
            from.push((
                position.slug.clone(),
                position.variant.clone(),
                per_set * whole,
            ));
        }
        let saved = f64::from(policy.least_profit) * f64::from(exchanges.saturating_sub(1));
        if set_price + saved < apart {
            continue;
        }
        let conversion = account::Conversion {
            from,
            into_slug: set.clone(),
            into_variant: String::new(),
            into_item: set.clone(),
            into_quantity: whole,
        };
        match gear.history.convert_positions(conversion).await {
            Ok(()) => {
                assembled = true;
                info!(
                    set,
                    sets = whole,
                    set_platinum = set_price,
                    parts_platinum = apart,
                    "held parts combined into sets for sale"
                );
            }
            Err(why) => warn!(error = %why, set, "held parts not combined into set"),
        }
    }
    assembled
}

/// Number of full sets the free copies of the held parts make, limited by the scarcest part;
/// `None` when a part is not held or no full set results.
fn whole_sets(
    held: &[Position],
    needs: &BTreeMap<String, u32>,
    shown: &Shown,
    moved: &[Exchange],
) -> Option<u32> {
    needs
        .iter()
        .map(|(item, per_set)| {
            let position = held.iter().find(|position| &position.item == item)?;
            let free = stock(shown, position, None, moved)
                .unwrap_or(position.quantity)
                .min(position.quantity);
            Some(free / (*per_set).max(1))
        })
        .collect::<Option<Vec<u32>>>()?
        .into_iter()
        .min()
        .filter(|whole| *whole > 0)
}

/// Trades since the last inventory reading, which the dump does not include yet.
async fn since_reading(gear: &Gear) -> Vec<Exchange> {
    let Ok(Some(read)) = gear.history.last_reading().await else {
        return Vec::new();
    };
    gear.history
        .exchanges_since(read)
        .await
        .inspect_err(|why| warn!(error = %why, "trades since last inventory read failed"))
        .unwrap_or_default()
}

/// Free copies of the position: the count in the last dump plus buys and less sales since;
/// `None` before the first dump. A bought set is held as its parts, never as the built item, so
/// it counts as the number of full sets its scarcest part allows, by the same composition the
/// inventory uses for sets.
fn stock(
    shown: &Shown,
    position: &Position,
    set: Option<&SetRow>,
    moved: &[Exchange],
) -> Option<u32> {
    if shown.inventory.holdings.is_empty() {
        return None;
    }
    let free = match set {
        Some(set) => set
            .members
            .iter()
            .zip(&set.per_set)
            .map(|(part, per_set)| {
                market::free(&shown.inventory, part, "").map(|free| free / (*per_set).max(1))
            })
            .collect::<Option<Vec<u32>>>()?
            .into_iter()
            .min()?,
        // A set without a part list in the catalog cannot be counted, as with an unread dump.
        None if position.slug.ends_with(SET) => return None,
        None => market::free(&shown.inventory, &position.item, &position.variant)?,
    };
    let since: i64 = moved
        .iter()
        .filter(|exchange| exchange.slug == position.slug && exchange.variant == position.variant)
        .map(|exchange| match exchange.selling {
            true => -i64::from(exchange.quantity),
            false => i64::from(exchange.quantity),
        })
        .sum();
    Some(u32::try_from((i64::from(free) + since).max(0)).unwrap_or(u32::MAX))
}

/// Price and quantity change for an open sell order that no longer matches `asking` or
/// `stock`; `None` when it matches both.
fn amendment(order: &account::Standing, stock: u32, asking: u32) -> Option<serde_json::Value> {
    let mut change = serde_json::Map::new();
    if order.platinum != asking {
        change.insert("platinum".into(), asking.into());
    }
    if order.quantity != stock {
        change.insert("quantity".into(), stock.into());
    }
    match change.is_empty() {
        true => None,
        false => Some(serde_json::Value::Object(change)),
    }
}

/// Asking price for one copy: the market price, never under `floor`; `None` without a market
/// price.
fn asking(market: Option<u32>, floor: u32) -> Option<u32> {
    market.map(|market| market.max(floor))
}

/// Lowest price a copy may sell for: its cost, plus the profit floor while the position is
/// younger than `sale_days`.
fn floor(position: &Position, policy: &Policy, now: Instant) -> u32 {
    let basis = position.basis();
    match is_fresh(position, policy, now) {
        true => basis.saturating_add(policy.least_profit),
        false => basis,
    }
}

/// Whether the position is younger than the policy's `sale_days`.
fn is_fresh(position: &Position, policy: &Policy, now: Instant) -> bool {
    now - position.opened < Instant::from(policy.sale_days) * DAY
}

/// Asking price bounded by recent trades: never above the trade price, never more than the
/// usual spread below it while `fresh`, never under `floor`.
fn bounded(asking: u32, traded: Option<&Fair>, fresh: bool, floor: u32) -> u32 {
    let Some(traded) = traded else {
        return asking.max(floor);
    };
    let whole = |platinum: f64| platinum.round().clamp(0.0, f64::from(u32::MAX)) as u32;
    let held = match fresh {
        true => whole(traded.platinum - traded.spread),
        false => 0,
    };
    asking.min(whole(traded.platinum)).max(held).max(floor)
}

async fn put(
    gear: &Gear,
    position: &Position,
    permit: &Permit,
    asking: u32,
    stock: u32,
) -> Result<String, String> {
    let id = gear
        .market_cache
        .id_of(position.slug.clone())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the market does not list it".to_owned())?;
    let authorization = Token::read().ok_or_else(|| "no session".to_owned())?;
    let session = Session::resume(Arc::clone(gear.gateway.client()), authorization, None);
    let variant = Variant::parse(&position.variant);
    let _claim = gear
        .claim(&position.slug, &variant, true)
        .await
        .ok_or_else(|| "the copy is already offered".to_owned())?;
    // The market rejects the lot field on items traded one at a time and requires it on the
    // rest, so a copy whose rule cannot be read is not listed instead of listed wrongly.
    let lots = lots_allowed(
        &gear.gateway,
        &gear.market_cache,
        Priority::Commit,
        &position.slug,
    )
    .await?;

    let order = Placing {
        item: position.item.clone(),
        item_id: id,
        side: Side::Sell,
        platinum: asking,
        quantity: stock,
        // As many copies per trade as the policy allows, never more than the order's quantity.
        // One copy per trade would use up one in-game trade for every copy.
        per_trade: lots.then(|| gear.policy().most_per_trade.max(1).min(stock.max(1))),
        rank: variant.rank,
        subtype: variant.subtype.clone(),
        visible: true,
    };

    let placed = gear
        .gateway
        .call(Priority::Commit, |pass| session.place(pass, permit, &order))
        .await
        .map_err(|e| e.to_string())?;
    let id = placed.id.clone();
    let key = Variant::of_mine(&placed).key();
    gear.history
        .keep_placed_order((placed, Some(position.slug.clone()), key), None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(spent: u32, quantity: u32) -> Position {
        Position {
            item: "/Lotus/Powersuits/Hoplite/StyanaxPrime".into(),
            slug: "styanax_prime_set".into(),
            variant: String::new(),
            quantity,
            spent,
            opened: 0,
            ranked_before: None,
        }
    }

    /// A mod: held as itself, and a copy of it can be equipped in a build.
    const MOD: &str = "/Lotus/Mods/Bite";

    fn shown(copies: u32, built: bool) -> Shown {
        let item = MOD.to_owned();
        Shown {
            step: 1,
            inventory: Arc::new(median_core::Inventory {
                holdings: vec![median_core::Holding {
                    item: item.clone(),
                    variant: median_core::Variant::default(),
                    detail: None,
                    count: copies,
                }],
                equipped: match built {
                    true => vec![median_core::Equipped {
                        item,
                        variant: median_core::Variant::default(),
                    }],
                    false => Vec::new(),
                },
                ..median_core::Inventory::default()
            }),
        }
    }

    fn exchange(selling: bool) -> Exchange {
        Exchange {
            at: 0,
            item: "/Lotus/Powersuits/Hoplite/StyanaxPrime".into(),
            slug: "styanax_prime_set".into(),
            variant: String::new(),
            selling,
            platinum: 40,
            quantity: 1,
            origin: "stream".into(),
            agent: true,
        }
    }

    #[test]
    fn a_copy_bought_since_the_dump_is_held_though_the_dump_does_not_show_it() {
        // One copy is equipped in a build and belongs to the user; the bought copy came after
        // the dump.
        let bite = Position {
            item: MOD.into(),
            slug: "bite".into(),
            ..held(40, 1)
        };
        let moved = |selling| Exchange {
            item: MOD.into(),
            slug: "bite".into(),
            ..exchange(selling)
        };
        assert_eq!(stock(&shown(1, true), &bite, None, &[]), Some(0));
        assert_eq!(
            stock(&shown(1, true), &bite, None, &[moved(false)]),
            Some(1)
        );
        assert_eq!(stock(&shown(2, true), &bite, None, &[moved(true)]), Some(0));
    }

    #[test]
    fn a_bought_set_is_held_as_its_parts_and_counts_by_the_scarcest() {
        let parts = ["bp", "chassis", "neuroptics", "systems"];
        let set = SetRow {
            slug: "styanax_prime_set".into(),
            name: "Styanax Prime Set".into(),
            ducats: None,
            vaulted: None,
            built: Some("/Lotus/Powersuits/Hoplite/StyanaxPrime".into()),
            members: parts.iter().map(|part| (*part).to_owned()).collect(),
            per_set: vec![1, 1, 1, 1],
        };
        let holding = |item: &str, count: u32| median_core::Holding {
            item: item.into(),
            variant: median_core::Variant::default(),
            detail: None,
            count,
        };
        let parts_held = |missing: Option<&str>| Shown {
            step: 1,
            inventory: Arc::new(median_core::Inventory {
                holdings: parts
                    .iter()
                    .filter(|part| Some(**part) != missing)
                    .map(|part| holding(part, 1))
                    .collect(),
                ..median_core::Inventory::default()
            }),
        };

        // No built warframe in the inventory, but the parts make one full set for the position.
        assert_eq!(
            stock(&parts_held(None), &held(40, 1), Some(&set), &[]),
            Some(1)
        );
        assert_eq!(
            stock(&parts_held(Some("systems")), &held(40, 1), Some(&set), &[]),
            Some(0),
            "a part gone, the set is not whole"
        );
        assert_eq!(
            stock(
                &parts_held(Some("systems")),
                &held(40, 1),
                Some(&set),
                &[exchange(false)]
            ),
            Some(1),
            "bought since the dump, it is held all the same"
        );
        assert_eq!(
            stock(&parts_held(None), &held(40, 1), None, &[]),
            None,
            "a set the catalog does not break into parts is not counted by what they build"
        );
    }

    #[test]
    fn a_set_is_whole_only_as_often_as_its_scarcest_part_allows() {
        let part = |item: &str, quantity: u32| Position {
            item: item.into(),
            slug: item.into(),
            ..held(10 * quantity, quantity)
        };
        let held = [part("blade", 5), part("handle", 2)];
        let unread = Shown {
            step: 0,
            inventory: Arc::default(),
        };
        let needs = |blades: u32, handles: u32| {
            BTreeMap::from([("blade".to_owned(), blades), ("handle".to_owned(), handles)])
        };

        assert_eq!(whole_sets(&held, &needs(2, 2), &unread, &[]), Some(1));
        assert_eq!(whole_sets(&held, &needs(2, 3), &unread, &[]), None);
        assert_eq!(
            whole_sets(
                &held,
                &BTreeMap::from([("guard".to_owned(), 1)]),
                &unread,
                &[]
            ),
            None
        );
    }

    #[test]
    fn nothing_is_known_held_before_any_dump() {
        assert_eq!(
            stock(
                &Shown {
                    step: 0,
                    inventory: Arc::default()
                },
                &held(40, 1),
                None,
                &[]
            ),
            None
        );
    }

    #[test]
    fn the_market_sets_the_price_when_it_stands_above_the_floor() {
        assert_eq!(asking(Some(38), 30), Some(38));
    }

    #[test]
    fn the_floor_holds_the_price_up() {
        assert_eq!(asking(Some(20), 25), Some(25));
    }

    #[test]
    fn a_pair_the_market_cache_cannot_price_is_not_offered() {
        assert_eq!(asking(None, 30), None);
    }

    #[test]
    fn the_floor_is_what_a_copy_cost_not_what_the_lot_cost() {
        assert_eq!(floor(&held(120, 6), &policy(), DAY * 30), 20);
    }

    #[test]
    fn a_fresh_position_asks_the_profit_floor_on_top_of_the_cost() {
        let fresh = Position {
            opened: 0,
            ..held(70, 1)
        };

        assert_eq!(floor(&fresh, &policy(), DAY), 70 + policy().least_profit);
        assert_eq!(floor(&fresh, &policy(), DAY * 30), 70);
    }

    #[test]
    fn an_ask_over_what_copies_changed_hands_at_comes_down_to_it() {
        assert_eq!(bounded(120, Some(&traded(98.0, 0.0)), true, 80), 98);
        assert_eq!(bounded(120, Some(&traded(60.0, 0.0)), true, 80), 80);
        assert_eq!(bounded(90, None, true, 80), 90);
    }

    #[test]
    fn a_fresh_offer_does_not_chase_a_lone_cheap_seller_under_the_trades() {
        assert_eq!(bounded(65, Some(&traded(88.0, 4.0)), true, 64), 84);
        assert_eq!(bounded(86, Some(&traded(88.0, 4.0)), true, 64), 86);
        assert_eq!(bounded(65, Some(&traded(88.0, 4.0)), false, 51), 65);
    }

    fn traded(platinum: f64, spread: f64) -> Fair {
        Fair {
            platinum,
            volume: 40,
            window: median_core::NEAR,
            spread,
        }
    }

    fn policy() -> Policy {
        Policy::default()
    }

    fn standing(platinum: u32, quantity: u32) -> account::Standing {
        account::Standing {
            id: "x".into(),
            item_id: "y".into(),
            slug: Some("styanax_prime_set".into()),
            variant: String::new(),
            selling: true,
            platinum,
            quantity,
            per_trade: 1,
            visible: true,
            by_agent: true,
            promised: None,
            created_at: "2026-08-01T00:00:00Z".into(),
            updated_at: "2026-08-01T00:00:00Z".into(),
        }
    }

    fn sale(id: &str, platinum: u32, born: &str, status: &str) -> String {
        format!(
            r#"{{"id":"{id}","type":"sell","platinum":{platinum},"quantity":1,
                "createdAt":"{born}","updatedAt":"{born}",
                "user":{{"ingameName":"{id}","status":"{status}"}}}}"#
        )
    }

    fn book(rows: &[String]) -> median_market::Book {
        median_market::Book::from(
            serde_json::from_str::<Vec<median_market::Order>>(&format!("[{}]", rows.join(",")))
                .expect("orders parse"),
        )
    }

    const OURS: &str = "2026-08-01T00:00:00Z";
    const LATER: &str = "2026-08-20T00:00:00Z";
    const EARLIER: &str = "2026-07-01T00:00:00Z";

    #[test]
    fn the_offer_stands_level_with_the_cheapest_seller_it_can_be_reached_beside() {
        let held = book(&[
            sale("mine", 90, OURS, "ingame"),
            sale("other", 70, LATER, "ingame"),
        ]);

        assert_eq!(stand(&held, Some("mine"), 20), Some(70));
    }

    #[test]
    fn an_older_seller_level_with_us_is_matched_not_undercut() {
        let held = book(&[
            sale("mine", 90, OURS, "ingame"),
            sale("early", 70, EARLIER, "ingame"),
        ]);

        // A new or touched order is the most recently updated, and the market lists it first.
        assert_eq!(stand(&held, Some("mine"), 20), Some(70));
    }

    #[test]
    fn our_own_offer_is_not_a_rival_to_undercut() {
        let alone = book(&[sale("mine", 90, OURS, "ingame")]);

        assert_eq!(stand(&alone, Some("mine"), 20), None);
    }

    #[test]
    fn with_nobody_reachable_the_whole_book_sets_the_price() {
        let held = book(&[
            sale("mine", 90, OURS, "ingame"),
            sale("away", 55, LATER, "offline"),
        ]);

        assert_eq!(stand(&held, Some("mine"), 20), Some(55));
    }

    #[test]
    fn the_cost_of_the_copy_is_the_floor_under_the_market() {
        let held = book(&[
            sale("mine", 90, OURS, "ingame"),
            sale("cheap", 12, LATER, "ingame"),
        ]);

        assert_eq!(stand(&held, Some("mine"), 30), Some(30));
    }

    #[test]
    fn a_lone_give_away_far_under_the_rest_is_not_followed() {
        let held = book(&[
            sale("mine", 109, OURS, "ingame"),
            sale("stray", 20, LATER, "ingame"),
            sale("next", 98, LATER, "ingame"),
            sale("far", 110, LATER, "ingame"),
        ]);

        assert_eq!(stand(&held, Some("mine"), 67), Some(98));
    }

    #[test]
    fn an_offer_that_already_matches_is_left_alone() {
        assert_eq!(amendment(&standing(38, 1), 1, 38), None);
    }

    #[test]
    fn a_moved_market_moves_the_offer() {
        let change = amendment(&standing(38, 1), 1, 41).expect("there is a change");
        assert_eq!(change["platinum"], 41);
        assert!(change.get("quantity").is_none());
    }

    #[test]
    fn more_copies_raise_the_offer_rather_than_opening_a_second() {
        let change = amendment(&standing(38, 1), 3, 38).expect("there is a change");
        assert_eq!(change["quantity"], 3);
        assert!(change.get("platinum").is_none());
    }
}
