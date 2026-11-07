mod lines;
mod nodes;
mod relics;
mod reroll;

#[cfg(windows)]
mod dbwin;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use median_core::{
    Cycling, Event, GameEvent, Instant, MarketEvent, Origin, Outcome, Recognition, Rewards, Swap,
    SwapKind, SwapPart, Whisper, seconds, split, words,
};
use median_engine::Input;
use tokio::sync::mpsc::{Sender, UnboundedSender};
use tracing::{debug, error, info, warn};

use crate::account;
use crate::catalog::{self, Catalog};
use crate::market::{self, Variant};
use crate::reward;
use crate::state_sink::{SharedHoldings, SharedState};
use lines::{LogEvent, Proposal, Reading};

/// Delay before the task opens the game log again after reading stopped.
const REOPEN: Duration = Duration::from_secs(10);

/// Seconds a new private chat tab waits for the user's outgoing message before it counts as
/// another player writing to the user. Measured 28.08.2026: the outgoing message is logged a
/// millisecond after the tab appears.
const GRACE: Instant = 2;

/// Timeout of the wait for a log event, so pending chat tabs reach `GRACE` while the game log is
/// quiet.
const TICK: Duration = Duration::from_millis(500);

/// Seconds between market cache reads for the reward cards, until the market cache has fetched
/// prices for every card since the screen opened.
const REPRICE: Instant = 2;

/// Delay before the reward cards are read again while some card is unrecognised: the game logs
/// them as shown before they finish sliding in, which takes longer on a slow connection.
const REREAD: Duration = Duration::from_millis(700);

/// Maximum number of reads of one reward screen.
const MOST_READS: u32 = 10;

/// Applies a trade the game confirmed to the account's own orders and positions.
///
/// Item names come in the game's language, so an order is matched by its catalog name in that
/// language. An item that matches no own order is not recorded.
async fn settle(tx: &Sender<Input>, context: &GameLogContext, lang: &str, swap: &Swap) {
    let standing = match context.history.standing_orders().await {
        Ok(standing) => standing,
        Err(why) => {
            warn!(error = %why, "open orders read failed");
            return;
        }
    };

    let items: Vec<&SwapPart> = swap
        .items()
        .iter()
        .filter(|part| part.kind == SwapKind::Item)
        .collect();
    let counts: Vec<u32> = items.iter().map(|part| part.count).collect();
    let selling = swap.selling();
    let mut lines: Vec<Line> = items
        .iter()
        .copied()
        .zip(split(swap.platinum_moved(), &counts))
        .map(|(part, platinum)| Line {
            part,
            count: part.count,
            platinum,
        })
        .collect();
    let mut closed = false;

    for (order, item, copies, platinum) in kits(context, &standing, lang, &mut lines, selling).await
    {
        closed |= write_down(context, &order, item, copies, platinum, selling).await;
    }

    for line in lines.iter().filter(|line| line.count > 0) {
        let Some((order, item)) = ours(context, &standing, lang, line.part, selling).await else {
            info!(name = line.part.name, "traded item matches no own order");
            continue;
        };
        closed |= write_down(context, &order, item, line.count, line.platinum, selling).await;
    }

    // The window reads its orders again, so it no longer shows the quantity the trade used.
    if closed {
        let _ = tx
            .send(Input::Command(Event::Market(MarketEvent::OrdersRead)))
            .await;
    }
}

/// One item of the trade: the count not yet matched to an order, and its share of the platinum.
struct Line<'a> {
    part: &'a SwapPart,
    count: u32,
    platinum: u32,
}

/// Records the trade against an order and closes that many copies of the order on
/// warframe.market; `true` when the order is closed or the trade was already recorded.
async fn write_down(
    context: &GameLogContext,
    order: &account::Standing,
    item: String,
    copies: u32,
    platinum: u32,
    selling: bool,
) -> bool {
    let each = platinum.div_ceil(copies.max(1));
    let made = account::Made {
        item,
        slug: order.slug.clone().unwrap_or_default(),
        variant: order.variant.clone(),
        selling,
        platinum: each,
        quantity: copies,
        origin: Origin::Stream.word().to_owned(),
        own: Some(order.id.clone()),
    };
    match context.history.recorded_lately(made.clone()).await {
        Ok(true) => {
            info!(order_id = order.id, "trade already recorded");
            return true;
        }
        Ok(false) => {}
        Err(why) => warn!(error = %why, "trade lookup failed"),
    }
    if let Err(why) = context.history.record_exchange(made).await {
        warn!(error = %why, "trade not saved");
        return false;
    }

    // Closing the order adds the trade to the user's transactions on warframe.market.
    match market::close_own_order(
        &context.gateway,
        market::Priority::Commit,
        &context.history,
        order,
        copies,
        each,
    )
    .await
    {
        Ok(()) => {
            info!(
                order_id = order.id,
                slug = order.slug,
                copies,
                selling,
                "order closed by trade"
            );
            if !selling {
                context.bought.notify_one();
            }
            true
        }
        Err(why) => {
            warn!(error = %why, order_id = order.id, "order close failed");
            false
        }
    }
}

/// Own set orders for which the trade contains every part: each with the set's item, the number
/// of complete sets and their platinum. The matched parts are removed from `lines`.
async fn kits(
    context: &GameLogContext,
    standing: &[account::Standing],
    lang: &str,
    lines: &mut [Line<'_>],
    selling: bool,
) -> Vec<(account::Standing, String, u32, u32)> {
    let mut found = Vec::new();
    for order in standing.iter().filter(|order| order.selling == selling) {
        let Some(slug) = order.slug.as_deref() else {
            continue;
        };
        let Some(parts) = parts_named(context, lang, slug).await else {
            continue;
        };
        let Ok(Some(known)) = market::named_by_slug(&context.catalog, lang, slug).await else {
            continue;
        };
        if let Some((copies, platinum)) = take_kit(lines, &parts, order.quantity) {
            found.push((order.clone(), known.item, copies, platinum));
        }
    }
    found
}

/// Removes from `lines` as many complete sets as all parts allow, up to `most`; returns the
/// number of sets and the platinum of their parts, `None` when no complete set is present.
fn take_kit(lines: &mut [Line<'_>], parts: &[(Vec<String>, u32)], most: u32) -> Option<(u32, u32)> {
    let at: Vec<(usize, u32)> = parts
        .iter()
        .map(|(name, need)| {
            lines
                .iter()
                .position(|line| line.count > 0 && words(&line.part.name) == *name)
                .map(|index| (index, (*need).max(1)))
        })
        .collect::<Option<_>>()?;
    let copies = at
        .iter()
        .map(|&(index, need)| lines[index].count / need)
        .min()?
        .min(most);
    if copies == 0 {
        return None;
    }

    let mut platinum = 0;
    for (index, need) in at {
        let line = &mut lines[index];
        let used = copies * need;
        let share = (u64::from(line.platinum) * u64::from(used) / u64::from(line.count)) as u32;
        line.platinum -= share;
        line.count -= used;
        platinum += share;
    }
    Some((copies, platinum))
}

/// Parts of the set with warframe.market slug `slug`, as the words of their names in the game's
/// language, each with its count per set; `None` when the slug is not a set.
async fn parts_named(
    context: &GameLogContext,
    lang: &str,
    slug: &str,
) -> Option<Vec<(Vec<String>, u32)>> {
    let wanted = slug.to_owned();
    let set = context
        .catalog
        .read(move |connection| catalog::sets(connection, "en"))
        .await
        .ok()?
        .into_iter()
        .find(|set| set.slug == wanted)?;
    let need: HashMap<String, u32> = set
        .members
        .iter()
        .cloned()
        .zip(set.per_set.iter().copied())
        .collect();
    let members = set.members;
    let count = members.len();
    let slugs = context
        .catalog
        .read(move |connection| catalog::slugs(connection, &members))
        .await
        .ok()?;

    let mut names = Vec::with_capacity(count);
    for (item, part) in slugs {
        let known = market::named_by_slug(&context.catalog, lang, &part)
            .await
            .ok()??;
        names.push((words(&known.name), need.get(&item).copied().unwrap_or(1)));
    }
    (count > 0 && names.len() == count).then_some(names)
}

/// Own order that this trade item matches by side, name and rank, with the item's unique name;
/// `None` when no order matches.
async fn ours(
    context: &GameLogContext,
    standing: &[account::Standing],
    lang: &str,
    part: &SwapPart,
    selling: bool,
) -> Option<(account::Standing, String)> {
    let name = words(&part.name);
    for order in standing.iter().filter(|order| order.selling == selling) {
        let Some(slug) = order.slug.as_deref() else {
            continue;
        };
        if !ranked_alike(part, &order.variant) {
            continue;
        }
        let Ok(Some(known)) = market::named_by_slug(&context.catalog, lang, slug).await else {
            continue;
        };
        if words(&known.name) == name {
            return Some((order.clone(), known.item));
        }
    }
    None
}

/// Whether the traded copy has the rank the order is for. A rank 10 mod is a different item
/// from the same mod at rank 0, at a different price, so a buy order for one is not filled by
/// the other. The game shows the rank for mods but not for arcanes and parts, and a copy without
/// a shown rank matches any order.
fn ranked_alike(part: &SwapPart, variant: &str) -> bool {
    match part.rank {
        None => true,
        Some(rank) => Variant::parse(variant).rank.unwrap_or(0) == rank,
    }
}

/// Removes and returns the chat tabs open for at least `GRACE` seconds, which count as another
/// player writing to the user.
fn ripe(opening: &mut Vec<(String, Instant)>, now: Instant) -> Vec<(String, Instant)> {
    let (ripe, waiting) = std::mem::take(opening)
        .into_iter()
        .partition(|(_, at)| now - *at >= GRACE);
    *opening = waiting;
    ripe
}

/// Stops the game log reader thread when the task that started it is dropped.
struct Reader(Arc<AtomicBool>);

impl Drop for Reader {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// Dependencies of the game log reader for recording trades and pricing the reward screen.
pub struct GameLogContext {
    pub history: Arc<account::Store>,
    pub catalog: Arc<Catalog>,
    pub gateway: market::Gateway,
    /// Notified when a buy is recorded, so the copy is listed for sale at once.
    pub bought: Arc<tokio::sync::Notify>,
    pub market_cache: Arc<market::Store>,
    pub holdings: SharedHoldings,
    pub needs: market::Needs,
}

/// Reward cards of the screen in the form sent to the overlay.
fn read(screen: &reward::Screen, found: &[reward::Found], fresh: bool) -> Rewards {
    let ours = screen.ours();
    Rewards::Read {
        cards: found
            .iter()
            .map(|card| card.offered(card.item() == ours))
            .collect(),
        fresh,
    }
}

/// Clears the reward screen's price requests to the market cache and the shown cards, once the
/// screen closes.
fn forget(needs: &market::Needs, shown: &mut Vec<reward::Found>, repricing: &mut Option<Instant>) {
    needs.declare(median_core::Holder::Rewards, Vec::new());
    shown.clear();
    *repricing = None;
}

/// Favourite items of the profile whose holdings are loaded; empty when the account store read
/// fails.
async fn marked(
    history: &account::Store,
    holdings: &SharedHoldings,
) -> std::collections::HashSet<String> {
    match history
        .favourites(holdings.read().inventory.profile.clone())
        .await
    {
        Ok(items) => items.into_iter().collect(),
        Err(why) => {
            warn!(error = %why, "favourites read failed");
            std::collections::HashSet::new()
        }
    }
}

/// Reads the game log for as long as the application runs, reopening it `REOPEN` after it stops.
pub async fn tend(
    tx: Sender<Input>,
    shared: SharedState,
    context: GameLogContext,
) -> Result<(), String> {
    let (reader, recognition) = match tokio::task::block_in_place(reward::reader) {
        Ok(reader) => (Some(reader), Recognition::Ready),
        Err(state) => (None, state),
    };
    say(&tx, GameEvent::Recognition(recognition)).await;
    let mut screen = reward::Screen::new(reader.clone());
    let mut cycling = crate::cycling::Screen::new(reader);

    loop {
        listen(&tx, &shared, &context, &mut screen, &mut cycling).await;
        say(
            &tx,
            GameEvent::Listening {
                listening: false,
                shared: false,
            },
        )
        .await;
        tokio::time::sleep(REOPEN).await;
    }
}

/// Reads the reward cards and shows them, keeping the previous read when it recognised more
/// cards; returns when to read again while some card is unrecognised and reads remain.
#[cfg(windows)]
async fn read_cards(
    tx: &Sender<Input>,
    shared: &SharedState,
    context: &GameLogContext,
    screen: &reward::Screen,
    shown: &mut Vec<reward::Found>,
    repricing: &mut Option<Instant>,
    tries: u32,
) -> Option<(std::time::Instant, u32)> {
    let again = (tries + 1 < MOST_READS).then(|| (std::time::Instant::now() + REREAD, tries + 1));
    let lang = spoken(shared);
    let favourites = marked(&context.history, &context.holdings).await;
    let found = match screen
        .cards(
            &context.catalog,
            &context.market_cache,
            &context.holdings,
            &favourites,
            &lang,
        )
        .await
    {
        Ok(found) => found,
        Err(why) => {
            if again.is_none() {
                warn!(error = %why, attempts = tries, "reward capture failed");
                if shown.is_empty() {
                    say(tx, GameEvent::Rewards(Rewards::Gone)).await;
                }
            }
            return again;
        }
    };

    let named = |cards: &[reward::Found]| cards.iter().filter(|card| card.item().is_some()).count();
    if !shown.is_empty() && named(&found) < named(shown) {
        return again;
    }
    let settled = screen.fresh(&found);
    say(tx, GameEvent::Rewards(read(screen, &found, settled))).await;
    context.needs.declare(
        median_core::Holder::Rewards,
        found
            .iter()
            .filter_map(|card| card.pair.clone())
            .map(|(slug, variant)| median_core::Need {
                slug,
                variant,
                interest: median_core::Interest::Waiting,
            })
            .collect(),
    );
    *repricing = (!settled).then(|| seconds() + REPRICE);
    let is_whole = !found.is_empty() && named(&found) == found.len();
    if is_whole || again.is_none() {
        info!(
            attempts = tries,
            picked = screen.ours().unwrap_or("none"),
            matches_game_log = screen.agrees(&found),
            lowest_confidence = found
                .iter()
                .map(|card| card.confidence)
                .fold(f32::MAX, f32::min),
            cards = found
                .iter()
                .map(|card| match card.item() {
                    Some(item) => item.to_owned(),
                    None => format!("?{}", card.text),
                })
                .collect::<Vec<_>>()
                .join(" | "),
            "reward screen read"
        );
    }
    *shown = found;
    if is_whole { None } else { again }
}

#[cfg(windows)]
/// Passes the event a game log line tells, if any, to the loop that handles it, without waiting.
fn hand_on(reading: &mut Reading, lines: &UnboundedSender<(LogEvent, Instant)>, line: &str) {
    if let Some(said) = reading.line(line) {
        let _ = lines.send((said, seconds()));
    }
}

#[cfg(not(windows))]
async fn listen(
    _: &Sender<Input>,
    _: &SharedState,
    _: &GameLogContext,
    _: &mut reward::Screen,
    _: &mut crate::cycling::Screen,
) {
    std::future::pending::<()>().await;
}

#[cfg(windows)]
async fn listen(
    tx: &Sender<Input>,
    shared: &SharedState,
    context: &GameLogContext,
    screen: &mut reward::Screen,
    cycling: &mut crate::cycling::Screen,
) {
    let (lines, mut events) = tokio::sync::mpsc::unbounded_channel::<(LogEvent, Instant)>();
    let (opened, mut taken) = tokio::sync::mpsc::channel::<bool>(1);
    let running = Arc::new(AtomicBool::new(true));
    let stop = Reader(Arc::clone(&running));

    let reading = std::thread::spawn(move || {
        let mut reading = Reading::new();
        dbwin::listen(
            &running,
            |shared| {
                let _ = opened.blocking_send(shared);
            },
            |line| hand_on(&mut reading, &lines, line),
        )
    });

    if let Some(shared_with) = taken.recv().await {
        info!(
            shared_with_other_reader = shared_with,
            "game log reader started"
        );
        say(
            tx,
            GameEvent::Listening {
                listening: true,
                shared: shared_with,
            },
        )
        .await;
    }

    let mut proposed: Option<Proposal> = None;
    // Private chat tabs opened but not yet matched to an outgoing message from the user.
    let mut opening: Vec<(String, Instant)> = Vec::new();
    // Reward cards last shown, and when their prices are read from the market cache again.
    let mut shown = Vec::new();
    let mut repricing: Option<Instant> = None;
    // Next read of the cards while some card is unrecognised, and the number of reads so far.
    let mut rereading: Option<(std::time::Instant, u32)> = None;
    // The node the host announced, until a level opens; then the node the game is at.
    let mut announced: Option<String> = None;
    let mut entered: Option<String> = None;

    loop {
        let next = tokio::time::timeout(TICK, events.recv()).await;
        if screen.overstayed() {
            warn!("reward screen past its countdown, overlay hidden");
            screen.gone();
            rereading = None;
            forget(&context.needs, &mut shown, &mut repricing);
            say(tx, GameEvent::Rewards(Rewards::Gone)).await;
        }
        if let Some((at, tries)) = rereading
            && std::time::Instant::now() >= at
        {
            rereading = read_cards(
                tx,
                shared,
                context,
                screen,
                &mut shown,
                &mut repricing,
                tries,
            )
            .await;
        }
        // The market data the cards were first priced from can be hours old; fresh prices were
        // requested from the market cache when the cards appeared, and here they reach the overlay.
        if repricing.is_some_and(|at| seconds() >= at) && !shown.is_empty() {
            let lang = spoken(shared);
            let favourites = marked(&context.history, &context.holdings).await;
            let fresh = screen
                .repriced(
                    &context.catalog,
                    &context.market_cache,
                    &context.holdings,
                    &favourites,
                    &lang,
                    &shown,
                )
                .await;
            let settled = screen.fresh(&fresh);
            repricing = (!settled).then(|| seconds() + REPRICE);
            say(tx, GameEvent::Rewards(read(screen, &fresh, settled))).await;
            shown = fresh;
        }
        if cycling.reload_overdue(seconds()) {
            warn!("riven cycling screen not back after keep, overlay hidden");
            cycling.gone();
            say(tx, GameEvent::Cycling(Cycling::Gone)).await;
        }
        if !cycling.is_open(seconds()) && cycling.shown() != Cycling::Gone {
            warn!("riven cycling screen past its time limit, overlay hidden");
            cycling.gone();
            say(tx, GameEvent::Cycling(Cycling::Gone)).await;
        }
        if cycling.is_open(seconds()) {
            match tokio::task::block_in_place(|| cycling.look()) {
                Ok(Some(look)) => {
                    info!(chosen = look.layout.chosen, beside = ?look.layout.beside, "riven cycling screen cards changed");
                    say(tx, GameEvent::Cycling(cycling.shown())).await;
                    let lang = spoken(shared);
                    match cycling
                        .rivens(&context.catalog, &context.holdings, &lang, &look)
                        .await
                    {
                        Ok(read) => {
                            for card in read {
                                info!(
                                    riven = card.name,
                                    name_edits = card.apart,
                                    buffs = card.riven.buffs.len(),
                                    curses = card.riven.curses.len(),
                                    "riven read from cycling screen"
                                );
                            }
                        }
                        Err(why) => warn!(error = %why, "cycling screen rivens read failed"),
                    }
                    say(tx, GameEvent::Cycling(cycling.shown())).await;
                }
                Ok(None) => {}
                Err(why) => debug!(error = %why, "cycling screen capture failed"),
            }
        }
        for (player, at) in ripe(&mut opening, seconds()) {
            alert(shared);
            say(tx, GameEvent::Whispered(Whisper { player, at })).await;
        }
        let Ok(next) = next else { continue };
        let Some((said, at)) = next else { break };

        match said {
            // The game logs the same line whoever opened a private tab. A tab the user opened
            // is followed a moment later by the user's outgoing message.
            LogEvent::Opened { player } => opening.push((player, at)),
            LogEvent::Wrote { player } => opening.retain(|(held, _)| *held != player),
            LogEvent::Proposed(proposal) => proposed = Some(proposal),
            LogEvent::RewardScreen => {
                screen.opened();
                if screen.reads() {
                    say(tx, GameEvent::Rewards(Rewards::Reading)).await;
                }
            }
            LogEvent::OwnReward { item } => screen.took(item),
            LogEvent::RewardsCountdown { seconds } => screen.stands_for(seconds),
            LogEvent::RewardsGone => {
                screen.gone();
                rereading = None;
                forget(&context.needs, &mut shown, &mut repricing);
                say(tx, GameEvent::Rewards(Rewards::Gone)).await;
            }
            LogEvent::RewardsShown => {
                shown.clear();
                rereading =
                    read_cards(tx, shared, context, screen, &mut shown, &mut repricing, 0).await;
            }
            LogEvent::Announced { node } => announced = Some(node),
            LogEvent::LevelOpened => enter(tx, &mut entered, announced.take()).await,
            LogEvent::Located { node } => enter(tx, &mut entered, Some(node)).await,
            LogEvent::CycleScreen => {
                info!("riven cycling screen opened");
                cycling.opened(at);
                say(tx, GameEvent::Cycling(cycling.shown())).await;
            }
            LogEvent::Cycled(cycle) => {
                info!(
                    weapon = cycle.weapon,
                    riven = cycle.name,
                    kuva = cycle.kuva,
                    "riven cycled"
                );
                cycling.cycled(cycle.name, cycle.kuva);
                say(tx, GameEvent::Cycling(cycling.shown())).await;
            }
            LogEvent::CycleKept => info!("cycled riven kept"),
            LogEvent::CycleScreenGone => {
                info!("riven cycling screen closed");
                cycling.gone();
                say(tx, GameEvent::Cycling(cycling.shown())).await;
            }
            LogEvent::CycleScreenReloading => {
                info!("riven cycling screen reloading");
                cycling.reloading(at);
            }
            LogEvent::Settled(outcome) => {
                let Some(proposal) = proposed.take() else {
                    continue;
                };
                let swap = proposal.settled(at, outcome);
                info!(
                    partner = swap.partner,
                    platinum = swap.platinum_moved(),
                    outcome = outcome.as_code(),
                    "trade settled"
                );
                if outcome == Outcome::Done {
                    if let Err(why) = context.history.keep_swap(swap.clone()).await {
                        warn!(error = %why, "trade not recorded");
                    }
                    let lang = spoken(shared);
                    settle(tx, context, &lang, &swap).await;
                    say(tx, GameEvent::Traded(Box::new(swap))).await;
                }
            }
        }
    }

    drop(stop);
    match reading.join() {
        Ok(Err(why)) => warn!(error = why, "game log reader stopped"),
        Err(_) => error!("game log reader thread panicked"),
        Ok(Ok(())) => {}
    }
}

/// Sends `GameEvent::Entered` with the node the game is at, unless that node was already sent.
async fn enter(tx: &Sender<Input>, entered: &mut Option<String>, node: Option<String>) {
    if *entered != node {
        entered.clone_from(&node);
        say(tx, GameEvent::Entered(node)).await;
    }
}

/// Display language of the window.
fn spoken(shared: &SharedState) -> String {
    shared
        .0
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .locale
        .clone()
}

/// Plays the whisper chime when it is enabled in the settings.
fn alert(shared: &SharedState) {
    let asked = {
        let held = shared
            .0
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.alert.clone()
    };
    if !asked.whisper {
        return;
    }
    crate::sound::chime(asked.volume);
}

async fn say(tx: &Sender<Input>, event: GameEvent) {
    let _ = tx.send(Input::Command(Event::Game(event))).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn the_log_reader_hands_on_events_while_nobody_takes_them() {
        let (lines, events) = tokio::sync::mpsc::unbounded_channel();
        let mut reading = Reading::new();
        for _ in 0..1_000 {
            hand_on(
                &mut reading,
                &lines,
                "800.001 Sys [Info]: Created /Lotus/Interface/OmegaRerollSelection.swf",
            );
        }
        assert_eq!(events.len(), 1_000);
    }

    fn handed(rank: Option<u8>) -> SwapPart {
        SwapPart {
            name: "Инициирование Прайм".to_owned(),
            count: 1,
            kind: SwapKind::Item,
            rank,
        }
    }

    #[test]
    fn a_mod_bought_unranked_does_not_fill_a_bid_for_a_ranked_one() {
        assert!(!ranked_alike(&handed(Some(0)), "mod_rank=10"));
    }

    #[test]
    fn a_copy_at_the_rank_the_bid_asks_for_fills_it() {
        assert!(ranked_alike(&handed(Some(10)), "mod_rank=10"));
    }

    /// Arcanes and parts have no rank in brackets, and orders for them have no rank.
    #[test]
    fn a_rank_the_game_did_not_spell_tells_nothing_apart() {
        assert!(ranked_alike(&handed(None), "mod_rank=5"));
        assert!(ranked_alike(&handed(Some(0)), ""));
    }

    fn piece(name: &str, count: u32) -> SwapPart {
        SwapPart {
            name: name.to_owned(),
            count,
            kind: SwapKind::Item,
            rank: None,
        }
    }

    fn lato() -> Vec<(Vec<String>, u32)> {
        [
            "Лато Вандал: Чертёж",
            "Лато Вандал: Ствол",
            "Лато Вандал: Приёмник",
        ]
        .map(|name| (words(name), 1))
        .to_vec()
    }

    #[test]
    fn a_set_that_takes_two_of_a_part_fills_the_order_only_with_both() {
        let handed = [
            piece("ЧЕРТЁЖ: Лато Вандал", 1),
            piece("Лато Вандал: Ствол", 3),
            piece("Лато Вандал: Приёмник", 1),
        ];
        let mut lines: Vec<Line> = handed
            .iter()
            .map(|part| Line {
                part,
                count: part.count,
                platinum: 30,
            })
            .collect();
        let mut doubled = lato();
        doubled[1].1 = 2;

        assert_eq!(take_kit(&mut lines, &doubled, 5), Some((1, 80)));
        assert_eq!(lines[1].count, 1);
        assert_eq!(lines[1].platinum, 10);
    }

    #[test]
    fn every_part_of_a_set_handed_over_fills_the_set_order() {
        let handed = [
            piece("ЧЕРТЁЖ: Лато Вандал", 2),
            piece("Лато Вандал: Ствол", 2),
            piece("Лато Вандал: Приёмник", 2),
        ];
        let mut lines: Vec<Line> = handed
            .iter()
            .map(|part| Line {
                part,
                count: part.count,
                platinum: 30,
            })
            .collect();

        assert_eq!(take_kit(&mut lines, &lato(), 5), Some((2, 90)));
        assert!(
            lines
                .iter()
                .all(|line| line.count == 0 && line.platinum == 0)
        );
    }

    #[test]
    fn a_set_short_of_a_part_is_left_to_the_parts() {
        let handed = [
            piece("Лато Вандал: Ствол", 1),
            piece("Лато Вандал: Приёмник", 1),
        ];
        let mut lines: Vec<Line> = handed
            .iter()
            .map(|part| Line {
                part,
                count: part.count,
                platinum: 10,
            })
            .collect();

        assert_eq!(take_kit(&mut lines, &lato(), 1), None);
        assert!(lines.iter().all(|line| line.count == 1));
    }

    #[test]
    fn a_set_order_takes_no_more_sets_than_it_asks_for() {
        let handed = [
            piece("ЧЕРТЁЖ: Лато Вандал", 3),
            piece("Лато Вандал: Ствол", 3),
            piece("Лато Вандал: Приёмник", 3),
        ];
        let mut lines: Vec<Line> = handed
            .iter()
            .map(|part| Line {
                part,
                count: part.count,
                platinum: 30,
            })
            .collect();

        assert_eq!(take_kit(&mut lines, &lato(), 1), Some((1, 30)));
        assert!(
            lines
                .iter()
                .all(|line| line.count == 2 && line.platinum == 20)
        );
    }
}
