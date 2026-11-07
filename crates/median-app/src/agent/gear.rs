use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicU64, Ordering};

use median_core::{
    AgentEvent, Chance, Event, HOUR, Holder, Instant, Interest, Means, Measured, Need, Permit,
    Policy, Rates, Reason, Seen, Sight, Strategy, seconds,
};
use median_engine::Input;
use median_market::{Book, MarketError, Mine, Order, Placing, Session, Side, moment};
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, Notify};
use tracing::{info, warn};

use crate::account;
use crate::agent::rank_up;
use crate::catalog::Catalog;
use crate::market::{
    Books, Gateway, Heat, Needs, Priority, Reading, Store, Token, Variant, lots_allowed,
    named_by_slug, priced_at, sighted,
};
use crate::state_sink::{Presence, SharedHoldings, SharedState, from_policy};

/// Seconds the cached open orders of the account are reused before the account store is read
/// again.
const BOOKED: Instant = 5;

/// Percent of the watched pairs that must have a fresh book before the agent places a bid.
const WARM_PERCENT: u64 = 90;

/// Cache of the account's open orders. The order-stream handler and the recent-orders pass
/// check against this one copy, so neither places an order the other has just placed.
pub struct Booked {
    at: Instant,
    orders: Vec<account::Standing>,
}

impl Booked {
    fn new() -> Self {
        Self {
            at: Instant::MIN,
            orders: Vec::new(),
        }
    }

    /// Expires the cache, so the next read goes to the account store.
    fn stale(&mut self) {
        self.at = Instant::MIN;
    }
}

/// Pairs and sides, as (slug, variant, selling), with an order write in progress.
#[derive(Default)]
struct Claims(std::sync::Mutex<HashSet<(String, String, bool)>>);

/// Exclusive right of one task to place an order on one pair and side; released on drop.
pub(crate) struct Claim {
    claims: Arc<Claims>,
    key: (String, String, bool),
}

impl Drop for Claim {
    fn drop(&mut self) {
        self.claims
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.key);
    }
}

/// Most order-stream pairs kept in `Glimpses`; past this the oldest is dropped.
const GLIMPSED: usize = 1_000;

/// Pairs that appeared in the order stream without a fresh book in the market cache. The agent
/// sends no request itself: it declares the pairs as needs, and the scheduler fetches their
/// books, newest pair first.
struct Glimpses {
    order: VecDeque<String>,
    held: HashSet<String>,
    /// The latest order-stream order on each pair, evaluated again when the pair's book arrives.
    cues: HashMap<String, Cue>,
}

impl Glimpses {
    fn new() -> Self {
        Self {
            order: VecDeque::new(),
            held: HashSet::new(),
            cues: HashMap::new(),
        }
    }

    /// Adds a slug with the order that brought it; returns whether the list changed.
    fn glimpsed(&mut self, slug: &str, cue: Cue) -> bool {
        self.cues.insert(slug.to_owned(), cue);
        if !self.held.insert(slug.to_owned()) {
            return false;
        }
        self.order.push_back(slug.to_owned());
        if self.order.len() > GLIMPSED
            && let Some(dropped) = self.order.pop_front()
        {
            self.held.remove(&dropped);
            self.cues.remove(&dropped);
        }
        true
    }

    /// Removes a slug; returns whether the list changed.
    fn answered(&mut self, slug: &str) -> bool {
        self.cues.remove(slug);
        if !self.held.remove(slug) {
            return false;
        }
        self.order.retain(|held| held != slug);
        true
    }

    fn needs(&self) -> Vec<Need> {
        self.order
            .iter()
            .map(|slug| Need {
                slug: slug.clone(),
                variant: String::new(),
                interest: Interest::Glimpsed,
            })
            .collect()
    }
}

/// What triggered the evaluation of a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    /// An order from the order stream, evaluated on arrival.
    Flow,
    /// A fetched book, or an order-stream order that waited for that book.
    Book,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Source::Flow => "flow",
            Source::Book => "book",
        }
    }
}

/// One pair to evaluate, with the price and side of the order that triggered it.
#[derive(Debug, Clone)]
struct Cue {
    item_id: String,
    slug: String,
    of: Variant,
    side: Side,
    platinum: u32,
    /// Copies per in-game trade.
    lot: u32,
    from: Source,
}

/// Interval between the agent's decision summaries in the log.
const TALLIED: Instant = HOUR;

/// Counts of the agent's decisions since the last summary, by source and outcome.
struct Funnel {
    since: Instant,
    counts: BTreeMap<(Source, &'static str), u64>,
}

impl Funnel {
    fn new() -> Self {
        Self {
            since: seconds(),
            counts: BTreeMap::new(),
        }
    }

    /// Counts one outcome and logs the summary once `TALLIED` has passed since the last one.
    fn count(&mut self, from: Source, outcome: &'static str, now: Instant) {
        *self.counts.entry((from, outcome)).or_default() += 1;
        if now - self.since < TALLIED {
            return;
        }
        let said: Vec<String> = self
            .counts
            .iter()
            .map(|((from, outcome), count)| format!("{}:{outcome}={count}", from.as_str()))
            .collect();
        info!(
            minutes = (now - self.since) / 60,
            outcomes = said.join(" "),
            "agent hourly summary"
        );
        self.counts.clear();
        self.since = now;
    }
}

/// How far back the observation log is read for the recent cheapest ask on a pair.
const LATELY: Instant = 6 * HOUR;

/// Least observed orders on a pair before its recent cheapest ask is used.
const ENOUGH: u32 = 3;

/// Most evaluated orders `Judged` remembers.
const REMEMBERED: usize = 4_096;

/// Orders already evaluated, oldest first. The socket and the recent-orders pass deliver the
/// same orders, and the market's recent list overlaps between passes; this keeps one order from
/// being evaluated again and again.
struct Judged {
    order: VecDeque<String>,
    held: HashSet<String>,
}

impl Judged {
    fn new() -> Self {
        Self {
            order: VecDeque::new(),
            held: HashSet::new(),
        }
    }

    /// Whether this order is new at its current price, quantity and visibility; records it if so.
    fn fresh(&mut self, order: &Order) -> bool {
        let key = format!(
            "{}:{}:{}:{}",
            order.id,
            order.platinum,
            order.quantity,
            order.visible.unwrap_or(true)
        );
        if !self.held.insert(key.clone()) {
            return false;
        }
        self.order.push_back(key);
        if self.order.len() > REMEMBERED
            && let Some(dropped) = self.order.pop_front()
        {
            self.held.remove(&dropped);
        }
        true
    }
}

/// Result of touching an order: re-saving it at its own price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Touched {
    /// The market set a new update time, so the order is listed first among equal prices again.
    Moved,
    /// The market kept the old update time, so touching cannot restore first place.
    Still,
    /// Not tried: an earlier touch showed that the market keeps the update time.
    Pointless,
    /// The market refused the change.
    Refused,
}

impl Touched {
    /// Outcome code counted in the summary of a repricing pass.
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Touched::Moved => "touched",
            Touched::Still => "untouched",
            Touched::Pointless => "touch-pointless",
            Touched::Refused => "touch-refused",
        }
    }
}

/// Shared state and handles the agent uses to evaluate orders and to place, amend and withdraw
/// its own.
pub struct Gear {
    pub tx: Sender<Input>,
    pub shared: SharedState,
    /// Whether warframe.market shows the account as in game; a change wakes the bid and sell
    /// order loops.
    pub(crate) presence: Presence,
    pub market_cache: Arc<Store>,
    pub catalog: Arc<Catalog>,
    pub history: Arc<account::Store>,
    pub gateway: Gateway,
    /// Account holdings from the latest inventory dump.
    pub holdings: SharedHoldings,
    glimpses: Mutex<Glimpses>,
    /// Latest full book of each pair the agent trades, as fetched by the scheduler.
    pub(crate) books: Books,
    /// Pairs whose books the agent needs the scheduler to keep fresh.
    pub(crate) needs: Needs,
    booked: Mutex<Booked>,
    judged: Mutex<Judged>,
    funnel: std::sync::Mutex<Funnel>,
    heard: AtomicU64,
    unreachable: AtomicU64,
    said: Mutex<Instant>,
    /// Notified when a book or an order affects one of the agent's bids, so the repricing pass
    /// runs now instead of at its next scheduled time.
    pub(crate) nudge: Arc<Notify>,
    /// Pairs with an order write in progress.
    claims: Arc<Claims>,
    /// Held while a claim is taken, so two tasks cannot both find the same pair free.
    claiming: Mutex<()>,
    /// Fill rate of the agent's bids at each depth.
    rates: std::sync::RwLock<Rates>,
    /// (watched pairs with a fresh book, watched pairs).
    warmth: std::sync::Mutex<(u32, u32)>,
    /// Whether touching an order changes its update time on the market; `None` until the first
    /// touch returns.
    touch_moves: std::sync::Mutex<Option<bool>>,
}

impl Gear {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tx: Sender<Input>,
        shared: SharedState,
        presence: Presence,
        market_cache: Arc<Store>,
        catalog: Arc<Catalog>,
        history: Arc<account::Store>,
        gateway: Gateway,
        holdings: SharedHoldings,
        needs: Needs,
        books: Books,
    ) -> Self {
        Self {
            tx,
            shared,
            presence,
            market_cache,
            catalog,
            history,
            gateway,
            holdings,
            books,
            needs,
            glimpses: Mutex::new(Glimpses::new()),
            booked: Mutex::new(Booked::new()),
            judged: Mutex::new(Judged::new()),
            funnel: std::sync::Mutex::new(Funnel::new()),
            heard: AtomicU64::new(0),
            unreachable: AtomicU64::new(0),
            said: Mutex::new(Instant::MIN),
            nudge: Arc::new(Notify::new()),
            claims: Arc::new(Claims::default()),
            claiming: Mutex::new(()),
            rates: std::sync::RwLock::new(Rates::default()),
            warmth: std::sync::Mutex::new((0, 0)),
            touch_moves: std::sync::Mutex::new(None),
        }
    }

    /// Whether at least `WARM_PERCENT` of the watched pairs have a fresh book.
    fn is_warm(&self) -> bool {
        let (warm, hot) = *self.warmth.lock().unwrap_or_else(PoisonError::into_inner);
        hot > 0 && u64::from(warm) * 100 >= u64::from(hot) * WARM_PERCENT
    }

    /// Latest measured fill rate of the agent's bids at each depth.
    pub(crate) fn rates(&self) -> Rates {
        self.rates
            .read()
            .map(|rates| rates.clone())
            .unwrap_or_default()
    }

    /// Replaces the fill rates with a new measurement.
    pub(crate) fn learn(&self, rates: Rates) {
        if let Ok(mut held) = self.rates.write() {
            *held = rates;
        }
    }

    /// Declares an order-stream pair without a fresh book as a `Glimpsed` need, so the scheduler
    /// fetches its book ahead of the watched and sampled pairs.
    async fn glimpsed(&self, cue: &Cue) {
        let mut glimpses = self.glimpses.lock().await;
        if glimpses.glimpsed(&cue.slug, cue.clone()) {
            self.needs.declare(Holder::Flow, glimpses.needs());
        }
    }

    /// The account's open orders, from the cache or from the account store once the cache is
    /// `BOOKED` seconds old.
    async fn standing(&self, now: Instant) -> Vec<account::Standing> {
        let mut booked = self.booked.lock().await;
        if now.saturating_sub(booked.at) >= BOOKED {
            booked.orders = self.history.standing_orders().await.unwrap_or_default();
            booked.at = now;
        }
        booked.orders.clone()
    }

    /// The agent policy currently set in the settings.
    pub fn policy(&self) -> Policy {
        let held = self.shared.0.read().unwrap_or_else(PoisonError::into_inner);
        from_policy(&held.policy)
    }

    /// The account's open orders, cached for up to `BOOKED` seconds.
    pub async fn standing_now(&self) -> Vec<account::Standing> {
        self.standing(seconds()).await
    }

    /// Claims one pair and side for an order write; `None` when another task holds the claim,
    /// an order is already open there, or the open orders cannot be read. Reads the open orders
    /// from the account store, not from the cache.
    pub(crate) async fn claim(&self, slug: &str, of: &Variant, selling: bool) -> Option<Claim> {
        let key = (slug.to_owned(), of.key(), selling);
        let _taking = self.claiming.lock().await;
        if self
            .claims
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&key)
        {
            return None;
        }

        let standing = match self.history.standing_orders().await {
            Ok(standing) => standing,
            Err(why) => {
                warn!(error = %why, slug, "open orders read failed");
                return None;
            }
        };
        {
            let mut booked = self.booked.lock().await;
            booked.orders = standing.clone();
            booked.at = seconds();
        }

        // A sell order and a buy order may both be open on one pair, so each side is checked
        // on its own.
        let held = match selling {
            true => standing.iter().any(|order| {
                order.selling && order.slug.as_deref() == Some(slug) && order.variant == key.1
            }),
            false => listed(&standing, slug, of),
        };
        if held {
            return None;
        }

        self.claims
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.clone());
        Some(Claim {
            claims: Arc::clone(&self.claims),
            key,
        })
    }

    /// Evaluates one order from the market, places a bid when the policy allows one, and sends
    /// the decision to the window. An order already evaluated is skipped.
    pub async fn look(&self, order: &Order) {
        if !self.judged.lock().await.fresh(order) {
            return;
        }

        let now = seconds();
        let Some(id) = order.item_id.as_deref() else {
            return;
        };
        let Ok(Some(slug)) = self.market_cache.slug_of(id.to_owned()).await else {
            return;
        };
        let of = Variant::of_order(order);
        self.count_heard(now, order.user.status.reachable()).await;
        let standing = self.standing(now).await;

        // A bid at or above the agent's price wakes the repricing pass.
        if order.side == Side::Buy
            && standing.iter().any(|ours| {
                ours.by_agent
                    && !ours.selling
                    && ours.slug.as_deref() == Some(slug.as_str())
                    && ours.variant == of.key()
                    && ours.platinum <= order.platinum
            })
        {
            self.nudge.notify_one();
        }

        if !order.user.status.reachable() {
            return;
        }

        let lot = match order.side {
            Side::Sell => order.per_trade.unwrap_or(1),
            // One copy: a buy order has no lot size, and a guessed one would tie up
            // platinum the user did not choose to commit.
            Side::Buy => 1,
        };
        self.act(Cue {
            item_id: id.to_owned(),
            slug,
            of,
            side: order.side,
            platinum: order.platinum,
            lot,
            from: Source::Flow,
        })
        .await;
    }

    /// Evaluates a fetched book: first the order-stream order that waited for it, then each
    /// watched variant.
    pub async fn answered_with(&self, slug: &str, looked: &[Variant], heat: &Heat) {
        // A fresh book of a pair the agent bids on may show its bid was outbid, so the repricing
        // pass is woken.
        if self
            .standing(seconds())
            .await
            .iter()
            .any(|ours| ours.by_agent && !ours.selling && ours.slug.as_deref() == Some(slug))
        {
            self.nudge.notify_one();
        }

        let waiting = {
            let mut glimpses = self.glimpses.lock().await;
            let cue = glimpses.cues.get(slug).cloned();
            if glimpses.answered(slug) {
                self.needs.declare(Holder::Flow, glimpses.needs());
            }
            cue
        };
        if let Some(cue) = waiting {
            self.act(Cue {
                from: Source::Book,
                ..cue
            })
            .await;
        }

        for of in looked.iter().filter(|of| heat.holds(slug, &of.key())) {
            let now = seconds();
            let bid = match priced_at(&self.market_cache, slug, of, now).await {
                Ok(reading) => reading.bid,
                Err(_) => continue,
            };
            // No reachable bid: there is no price to outbid, and setting the first one would be
            // a guess.
            let Some(bid) = bid else {
                self.tally(Source::Book, "no-bid", now);
                continue;
            };
            let Ok(Some(item_id)) = self.market_cache.id_of(slug.to_owned()).await else {
                continue;
            };
            self.act(Cue {
                item_id,
                slug: slug.to_owned(),
                of: of.clone(),
                side: Side::Buy,
                platinum: bid,
                lot: 1,
                from: Source::Book,
            })
            .await;
        }
    }

    /// Stores how many watched pairs have a fresh book, of how many, and sends it to the window.
    pub async fn warmed(&self, (warm, hot): (u32, u32)) {
        *self.warmth.lock().unwrap_or_else(PoisonError::into_inner) = (warm, hot);
        let _ = self
            .tx
            .send(Input::Command(Event::Agent(AgentEvent::Warmed {
                warm,
                hot,
            })))
            .await;
    }

    fn tally(&self, from: Source, outcome: &'static str, now: Instant) {
        self.funnel
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .count(from, outcome, now);
    }

    /// Evaluates one cue and places the bid the policy allows.
    async fn act(&self, cue: Cue) {
        let now = seconds();
        let standing = self.standing(now).await;
        let from = cue.from;

        let (policy, means, lang) = {
            let held = self.shared.0.read().unwrap_or_else(PoisonError::into_inner);
            (
                from_policy(&held.policy),
                means_of(&held.account, &standing),
                held.locale.clone(),
            )
        };

        let Some((mut seen, taking)) = self
            .judge(&cue, &policy, &means, &standing, &lang, now)
            .await
        else {
            self.tally(from, "unpriced", now);
            return;
        };

        // Until most watched pairs have a fresh book, the first opportunity found is not
        // necessarily the best one.
        let taking = match taking {
            Some(_) if !self.is_warm() => {
                seen.passed = Some(Reason::Warming);
                None
            }
            taking => taking,
        };
        if let Some(taking) = taking {
            // Another task placed a bid here during this evaluation; the repricing pass raises
            // that bid instead.
            let claim = self.claim(&taking.slug, &taking.of, false).await;
            let Some(_claim) = claim else {
                self.nudge.notify_one();
                seen.passed = Some(Reason::AlreadyListed);
                self.tally(from, Reason::AlreadyListed.as_code(), now);
                let _ = self
                    .tx
                    .send(Input::Command(Event::Agent(AgentEvent::Looked(Box::new(
                        seen,
                    )))))
                    .await;
                return;
            };
            if taking.crowded && !self.make_room(&standing, taking.promised, now).await {
                self.tally(from, "crowded", now);
                return;
            }
            match self
                .place(&taking.permit, &taking.order, &taking.slug, taking.promised)
                .await
            {
                Ok(id) => {
                    info!(
                        order_id = id,
                        item = taking.permit.item(),
                        slug = taking.slug,
                        platinum = taking.order.platinum,
                        quantity = taking.order.quantity,
                        expected_value = taking.promised,
                        "agent bid placed"
                    );
                    self.booked.lock().await.stale();
                    let _ = self
                        .tx
                        .send(Input::Command(Event::Agent(AgentEvent::Placed)))
                        .await;
                }
                // A failed write is not retried, since its outcome is unknown; the orders page
                // reads the market again. The verdict records the refusal, not the permit.
                Err(why) => {
                    warn!(
                        item = taking.permit.item(),
                        error = why,
                        "agent bid placement failed"
                    );
                    seen.passed = Some(Reason::Refused);
                }
            }
        }

        self.tally(
            from,
            seen.passed.map_or("placed", |reason| reason.as_code()),
            now,
        );
        let _ = self
            .tx
            .send(Input::Command(Event::Agent(AgentEvent::Looked(Box::new(
                seen,
            )))))
            .await;
    }

    /// Evaluates one cue: the verdict for the window and, when the policy allows, the bid to
    /// place; `None` when the market cache cannot price or describe the pair.
    async fn judge(
        &self,
        cue: &Cue,
        policy: &Policy,
        means: &Means,
        standing: &[account::Standing],
        lang: &str,
        now: Instant,
    ) -> Option<(Seen, Option<Taking>)> {
        let (id, slug, of) = (cue.item_id.as_str(), cue.slug.clone(), cue.of.clone());

        let reading = priced_at(&self.market_cache, &slug, &of, now).await.ok()?;
        let known = named_by_slug(&self.catalog, lang, &slug)
            .await
            .ok()
            .flatten();
        let (name, item, image, category) = match known {
            Some(known) => (known.name, known.item, known.image, known.category),
            None => (slug.clone(), String::new(), None, String::new()),
        };
        let listed = listed(standing, &slug, &of);
        let lot = cue.lot;

        // The entry price depends on the side. Buying from a sell order costs its asking price.
        // Outbidding a buy order costs one platinum over the highest open bid, which is not
        // necessarily the order from the stream.
        let entry = |bid: Option<u32>| match cue.side {
            Side::Sell => cue.platinum,
            Side::Buy => bid
                .unwrap_or(cue.platinum)
                .max(cue.platinum)
                .saturating_add(1),
        };

        // Every order that reaches this point gets a verdict, so a rejected order can be told
        // apart from one that was never evaluated.
        let told = |seen: &Reading, passed: Reason| Seen {
            at: now,
            item: item.clone(),
            name: name.clone(),
            image: image.clone(),
            category: category.clone(),
            entry: entry(seen.bid),
            outbid: cue.side == Side::Buy,
            worth: seen.ingame,
            passed: Some(passed),
        };

        let stale = reading
            .looked
            .is_none_or(|looked| now - looked >= median_core::FRESH);
        if stale && cue.from == Source::Flow {
            self.glimpsed(cue).await;
        }

        if reading.traded.is_none() {
            return Some((told(&reading, Reason::NoReading), None));
        }

        // A mod bought to be ranked up is worth the price of the ranked copy, so the cache
        // check on the unranked copy does not apply to it.
        let ranked = rank_up::ranked_resale(self, &slug, &of, now).await;

        // The market cache estimate is checked first, and its verdict is the one the window
        // shows: rejected for volume, rejected below the profit floor, or evaluated on the book.
        let estimate = Chance {
            item: item.clone(),
            entry: entry(reading.bid),
            worth: goes_for(&reading, self.lately_asked(&slug, &of, now).await),
            volume: reading.traded,
            per_trade: lot,
            copies: median_core::copies(policy, means, entry(reading.bid)),
            listed,
            falling: reading.falling,
        };
        if ranked.is_none()
            && let Some(passed) = passed(&estimate, policy)
        {
            return Some((told(&reading, passed), None));
        }
        // The agent already has a bid open here, so its price is raised instead of a new order
        // added. The repricing pass does that, since it reads the pair's book.
        if listed {
            self.nudge.notify_one();
        }

        if stale {
            return Some((told(&reading, Reason::NoReading), None));
        }
        // A copy resells at the cheapest reachable ask; with no reachable seller there is no
        // resale price.
        if reading.ingame.is_none() {
            return Some((told(&reading, Reason::NoSeller), None));
        }
        let Some(worth) = reading.worth(policy) else {
            return Some((told(&reading, Reason::NoSeller), None));
        };

        if listed {
            return Some((told(&reading, Reason::AlreadyListed), None));
        }

        if let Some((_, endo)) = &ranked
            && rank_up::endo_left(self, standing)
                .await
                .is_none_or(|left| left < *endo)
        {
            return Some((told(&reading, Reason::NoEndo), None));
        }
        let sighted = sighted(&self.market_cache, &slug, &of, now).await.ok()?;
        let sight = Sight {
            resale: ranked.as_ref().map(|(seen, _)| seen.resale()),
            ..sighted.sight(&item, now)
        };
        let rates = self.rates();
        let strategy = Measured { rates: &rates };
        // Full order slots or no free capital is not final until the worst open bid is compared
        // with this one: withdrawing that bid frees both.
        let (quoted, crowded) = match strategy.quote(policy, means, &sight) {
            Err(Reason::TooManyOrders | Reason::NoCapital) => {
                let freed = worst_settled(standing, now).map_or(0, commits);
                let room = Means {
                    orders: means.orders.saturating_sub(1),
                    tied: means.tied.saturating_sub(freed),
                    free: means.free.map(|free| free.saturating_add(freed)),
                    ..*means
                };
                (strategy.quote(policy, &room, &sight), true)
            }
            asked => (asked, false),
        };
        // The market rejects the lot field on items traded one at a time and requires it on the
        // rest, so the rule is read from the market instead of guessed.
        let lots = match quoted.is_ok() {
            true => lots_allowed(&self.gateway, &self.market_cache, Priority::Commit, &slug)
                .await
                .inspect_err(|why| warn!(error = %why, slug, "market lot rule read failed"))
                .ok(),
            false => None,
        };
        let shown = quoted
            .as_ref()
            .map_or_else(|_| entry(reading.bid), |quote| quote.platinum);
        let taking = quoted.as_ref().ok().map(|quote| Taking {
            permit: quote.permit.clone(),
            slug: slug.clone(),
            of: of.clone(),
            promised: quote.promised,
            crowded,
            order: Placing {
                item: item.clone(),
                item_id: id.to_owned(),
                side: Side::Buy,
                platinum: quote.platinum,
                quantity: quote.copies,
                per_trade: lots.and_then(|allowed| allowed.then_some(quote.lot)),
                rank: of.rank,
                subtype: of.subtype.clone(),
                visible: true,
            },
        });

        let seen = Seen {
            at: now,
            item,
            name,
            image,
            category,
            entry: shown,
            outbid: cue.side == Side::Buy,
            worth: Some(worth),
            passed: quoted.err(),
        };
        Some((seen, taking))
    }

    /// Cheapest ask observed on this pair in the last `LATELY` seconds; `None` with fewer than
    /// `ENOUGH` observations. The observation log holds orders as they were posted, so this is
    /// what sellers asked, not the cheapest order open now.
    async fn lately_asked(&self, slug: &str, of: &Variant, now: Instant) -> Option<u32> {
        let (asked, heard) = self
            .market_cache
            .asked_lately(slug, &of.key(), now - LATELY)
            .await
            .ok()?;
        (heard >= ENOUGH).then_some(asked).flatten()
    }

    /// Counts one order received by the agent, whatever the policy decides about it.
    async fn count_heard(&self, now: Instant, reachable: bool) {
        // The stream delivers a few hundred orders a minute, so the running total goes to the
        // window at most once a second, not once per order.
        let total = self.heard.fetch_add(1, Ordering::Relaxed) + 1;
        let away = match reachable {
            true => self.unreachable.load(Ordering::Relaxed),
            false => self.unreachable.fetch_add(1, Ordering::Relaxed) + 1,
        };
        let mut said = self.said.lock().await;
        if now > *said {
            *said = now;
            let _ = self
                .tx
                .send(Input::Command(Event::Agent(AgentEvent::StreamCounted {
                    carried: total,
                    unreachable: away,
                })))
                .await;
        }
    }

    /// Withdraws the agent's bid with the lowest expected value so a better opportunity can take
    /// its slot and its platinum; returns whether one was withdrawn. Sell orders are never
    /// withdrawn: each is for a copy already paid for. A hidden bid has zero expected value
    /// while hidden, so any opportunity beats it.
    async fn make_room(&self, standing: &[account::Standing], promised: u32, now: Instant) -> bool {
        let Some(worst) = worst_settled(standing, now) else {
            return false;
        };
        let was = match worst.visible {
            true => worst.promised.unwrap_or(0),
            false => 0,
        };
        if was >= promised {
            return false;
        }

        let id = worst.id.clone();
        if !self.drop_order(&id).await {
            return false;
        }
        info!(
            order_id = id,
            expected_value_was = was,
            expected_value = promised,
            "agent bid withdrawn for a better one"
        );
        true
    }

    /// Withdraws one of the account's orders on the market and in the account store; returns
    /// whether the order is off the market.
    pub(crate) async fn drop_order(&self, id: &str) -> bool {
        let Some(authorization) = Token::read() else {
            warn!(order_id = id, "order withdraw skipped, not signed in");
            return false;
        };
        let session = Session::resume(Arc::clone(self.gateway.client()), authorization, None);
        let withdrawn = self
            .gateway
            .call(Priority::Commit, |pass| session.withdraw(pass, id))
            .await;
        match withdrawn {
            Ok(()) => {}
            // The market does not list it: filled, or withdrawn by hand or by the site. The
            // account store follows the market.
            Err(MarketError::NotListed(_)) => {
                info!(order_id = id, "order already gone from market");
            }
            Err(why) => {
                warn!(order_id = id, error = %why, "order withdraw failed");
                return false;
            }
        }
        if let Err(why) = self.history.withdraw_order(id.to_owned()).await {
            warn!(order_id = id, error = %why, "withdrawn order not removed from account store");
        }
        self.moved().await;
        true
    }

    /// Amends one of the account's open orders; returns the order as the market now stores it.
    pub(crate) async fn amend(&self, id: &str, change: serde_json::Value) -> Result<Mine, String> {
        let authorization = Token::read().ok_or_else(|| "no session".to_owned())?;
        let session = Session::resume(Arc::clone(self.gateway.client()), authorization, None);
        let amended = self
            .gateway
            .call(Priority::Commit, |pass| session.amend(pass, id, change))
            .await
            .map_err(|e| e.to_string())?;

        let key = Variant::of_mine(&amended).key();
        self.history
            .keep_one_order((amended.clone(), None, key))
            .await
            .map_err(|e| e.to_string())?;
        self.moved().await;
        Ok(amended)
    }

    /// Re-saves one of the account's orders at its own price, so its update time is later than
    /// `beat`, the update time of the rival listed above it.
    pub(crate) async fn touch(&self, order: &account::Standing, beat: &str) -> Touched {
        let moves = *self
            .touch_moves
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if moves == Some(false) {
            return Touched::Pointless;
        }
        let change = serde_json::json!({ "platinum": order.platinum });
        match self.amend(&order.id, change).await {
            Ok(amended) => {
                let moved = moment(&amended.updated_at) >= moment(beat);
                *self
                    .touch_moves
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = Some(moved);
                info!(
                    order_id = order.id,
                    rival_updated_at = beat,
                    updated_at = amended.updated_at,
                    moved,
                    "order touched to stand first"
                );
                match moved {
                    true => Touched::Moved,
                    false => Touched::Still,
                }
            }
            Err(why) => {
                warn!(order_id = order.id, error = why, "order touch refused");
                Touched::Refused
            }
        }
    }

    /// Expires the open-orders cache and notifies the window that the account's orders changed.
    async fn moved(&self) {
        self.booked.lock().await.stale();
        let _ = self
            .tx
            .send(Input::Command(Event::Agent(AgentEvent::Moved)))
            .await;
    }

    /// The account's platinum, commitments and order counts right now, as the policy uses them.
    pub(crate) fn means(&self, standing: &[account::Standing]) -> Means {
        let held = self.shared.0.read().unwrap_or_else(PoisonError::into_inner);
        means_of(&held.account, standing)
    }

    /// Places one order and records it in the account store as placed by the agent; returns the
    /// order id.
    pub(crate) async fn place(
        &self,
        permit: &Permit,
        order: &Placing,
        slug: &str,
        promised: u32,
    ) -> Result<String, String> {
        let authorization = Token::read().ok_or_else(|| "no session".to_owned())?;
        let session = Session::resume(Arc::clone(self.gateway.client()), authorization, None);
        let placed = self
            .gateway
            .call(Priority::Commit, |pass| session.place(pass, permit, order))
            .await
            .map_err(|e| e.to_string())?;

        let id = placed.id.clone();
        let variant = Variant::of_mine(&placed).key();
        self.history
            .keep_placed_order((placed, Some(slug.to_owned()), variant), Some(promised))
            .await
            .map_err(|e| e.to_string())?;
        self.booked.lock().await.stale();
        Ok(id)
    }
}

/// A bid the policy allows, ready to place.
struct Taking {
    permit: Permit,
    order: Placing,
    slug: String,
    /// The bid's variant, for the claim taken before placing it.
    of: Variant,
    /// Expected value of the bid, in platinum.
    promised: u32,
    /// Whether it fits only after one of the agent's open bids is withdrawn.
    crowded: bool,
}

/// Estimated price of a copy from the market cache alone: the higher of the closed-trade price
/// and the recent cheapest ask; `None` when neither is known.
///
/// The higher one on purpose. This estimate only decides whether the pair is worth one market
/// request, a request costs a fraction of the budget, and a missed opportunity is gone. The
/// book check that follows is the strict one.
fn goes_for(reading: &Reading, asked: Option<u32>) -> Option<u32> {
    let settled = reading
        .ours
        .map(|fair| fair.platinum.round().clamp(0.0, f64::from(u32::MAX)) as u32);
    match (settled, asked) {
        (Some(settled), Some(asked)) => Some(settled.max(asked)),
        (settled, asked) => settled.or(asked),
    }
}

/// Why the market cache estimate rejects an opportunity (`ThinVolume` or `BelowFloor`); `None`
/// when it is worth one market request. The thresholds are the policy's own, applied to the
/// cache estimate instead of to a book.
fn passed(chance: &Chance, policy: &Policy) -> Option<Reason> {
    if chance
        .volume
        .is_none_or(|volume| volume < policy.least_volume())
    {
        return Some(Reason::ThinVolume);
    }
    chance
        .profit(policy)
        .is_none_or(|profit| profit < i64::from(policy.least_profit))
        .then_some(Reason::BelowFloor)
}

/// The agent's bid with the lowest expected value, the first to withdraw.
pub(crate) fn worst_buy(standing: &[account::Standing]) -> Option<&account::Standing> {
    standing
        .iter()
        .filter(|order| order.by_agent && !order.selling)
        .min_by_key(|order| order.promised.unwrap_or(0))
}

/// Least age of an agent bid, in seconds, before a better opportunity may replace it.
const SETTLING: Instant = 15 * 60;

/// Whether a bid is at least `SETTLING` old, or of unknown age, and so may be replaced.
pub(crate) fn settled(order: &account::Standing, now: Instant) -> bool {
    median_market::moment(&order.created_at).is_none_or(|born| now - born >= SETTLING)
}

/// The agent's bid to withdraw first: a hidden one, whose expected value is zero while hidden,
/// else the one with the lowest expected value among those at least `SETTLING` old.
fn worst_settled(standing: &[account::Standing], now: Instant) -> Option<&account::Standing> {
    standing
        .iter()
        .filter(|order| order.by_agent && !order.selling)
        .filter(|order| !order.visible || settled(order, now))
        .min_by_key(|order| (order.visible, order.promised.unwrap_or(0)))
}

/// Whether a buy order, or a sell order the user placed, is already open on this item and
/// variant; the agent's own sell order does not block buying another copy.
fn listed(standing: &[account::Standing], slug: &str, of: &Variant) -> bool {
    let key = of.key();
    standing.iter().any(|order| {
        order.slug.as_deref() == Some(slug)
            && order.variant == key
            && !(order.selling && order.by_agent)
    })
}

/// Orders on `side` from reachable users, except the order `mine`.
fn reachable<'a>(book: &'a Book, side: Side, mine: &'a str) -> impl Iterator<Item = &'a Order> {
    book.rows()
        .iter()
        .filter(move |row| row.side == side && row.user.status.reachable() && row.id != mine)
}

/// Best price among reachable orders on `side` other than `mine`: the highest bid or the lowest
/// ask; `None` when there are none.
pub(crate) fn best(book: &Book, side: Side, mine: &str) -> Option<u32> {
    let prices = reachable(book, side, mine).map(|row| row.platinum);
    match side {
        Side::Buy => prices.max(),
        Side::Sell => prices.min(),
    }
}

/// Update time of the newest reachable order on `side` at `platinum` updated after `ours`;
/// `None` when there is none.
pub(crate) fn overtaken_by<'a>(
    book: &'a Book,
    side: Side,
    mine: &'a str,
    platinum: u32,
    ours: &str,
) -> Option<&'a str> {
    let ours = moment(ours)?;
    reachable(book, side, mine)
        .filter(|row| row.platinum == platinum)
        .filter_map(|row| Some((moment(&row.updated_at)?, row.updated_at.as_str())))
        .filter(|(at, _)| *at > ours)
        .max_by_key(|(at, _)| *at)
        .map(|(_, text)| text)
}

/// Last update time of the account's order: the later of the account store's and the book's.
pub(crate) fn updated(order: &account::Standing, book: &Book) -> String {
    match book.rows().iter().find(|row| row.id == order.id) {
        Some(row) if moment(&row.updated_at) > moment(&order.updated_at) => row.updated_at.clone(),
        _ => order.updated_at.clone(),
    }
}

/// Platinum one buy order would pay in full: price times quantity, quantity at least 1.
pub(crate) fn asks_for(order: &account::Standing) -> u32 {
    order.platinum.saturating_mul(order.quantity.max(1))
}

/// Platinum one buy order commits: `asks_for` while visible, 0 while hidden.
pub(crate) fn commits(order: &account::Standing) -> u32 {
    match order.visible {
        true => asks_for(order),
        false => 0,
    }
}

/// The account's means: the last account snapshot less the platinum open buy orders commit.
fn means_of(account: &median_ipc::Account, standing: &[account::Standing]) -> Means {
    let tied = standing
        .iter()
        .filter(|order| !order.selling)
        .map(commits)
        .fold(0u32, u32::saturating_add);

    Means {
        held: account.platinum,
        free: account.platinum.map(|held| held.saturating_sub(tied)),
        tied,
        exchanges: account.exchanges,
        // Counted separately: the agent's slots are the 100 market slots less the user's open
        // orders, so counting the user's orders here as well would subtract them twice.
        // `give_way` keeps the total under 100.
        orders: standing.iter().filter(|order| order.by_agent).count() as u32,
        hand: standing.iter().filter(|order| !order.by_agent).count() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing(slug: &str, variant: &str, selling: bool, platinum: u32) -> account::Standing {
        account::Standing {
            id: "x".into(),
            item_id: "y".into(),
            slug: Some(slug.into()),
            variant: variant.into(),
            selling,
            platinum,
            quantity: 1,
            per_trade: 1,
            visible: true,
            by_agent: true,
            promised: None,
            created_at: "2026-08-01T00:00:00Z".into(),
            updated_at: "2026-08-01T00:00:00Z".into(),
        }
    }

    fn purse(platinum: u32) -> median_ipc::Account {
        median_ipc::Account {
            profile: None,
            positions: 0,
            read: 0,
            distinct: 0,
            unreadable: 0,
            unidentified: 0,
            exchanges: Some(14),
            platinum: Some(platinum),
            credits: Some(11_737_725),
            moved: None,
        }
    }

    #[test]
    fn what_the_buys_hold_is_not_free_platinum_any_more() {
        let means = means_of(&purse(1_000), &[standing("gram_prime_set", "", false, 100)]);

        assert_eq!(means.tied, 100);
        assert_eq!(means.free, Some(900));
        assert_eq!(means.orders, 1);
    }

    #[test]
    fn an_order_put_up_by_hand_does_not_come_out_of_the_agents_share() {
        let mut theirs = standing("gram_prime_set", "", true, 40);
        theirs.by_agent = false;
        let ours = standing("bite", "", false, 100);

        let means = means_of(&purse(1_000), &[theirs, ours]);

        // The user's orders come out of the reserve; counting them here as well would block
        // the agent while the market still has free order slots.
        assert_eq!(means.orders, 1);
    }

    #[test]
    fn what_a_bid_ties_up_is_every_copy_it_asks_for() {
        let mut bid = standing("gram_prime_set", "", false, 100);
        bid.quantity = 7;
        bid.per_trade = 1;

        assert_eq!(means_of(&purse(1_000), &[bid]).tied, 700);
    }

    #[test]
    fn a_sale_of_ours_ties_up_no_platinum() {
        let means = means_of(&purse(480), &[standing("gram_prime_set", "", true, 100)]);

        assert_eq!(means.tied, 0);
        assert_eq!(means.free, Some(480));
        assert_eq!(means.orders, 1);
    }

    #[test]
    fn a_purse_no_dump_has_stated_stays_unknown() {
        let unread = median_ipc::Account {
            exchanges: None,
            platinum: None,
            credits: None,
            ..purse(0)
        };

        let means = means_of(&unread, &[standing("gram_prime_set", "", false, 100)]);

        assert_eq!(means.tied, 100);
        assert_eq!(means.free, None);
        assert_eq!(means.exchanges, None);
        assert_eq!(means.orders, 1);
    }

    fn order(id: &str, platinum: u32) -> Order {
        let json = format!(
            r#"{{"id":"{id}","type":"sell","platinum":{platinum},"quantity":1,
                "createdAt":"2026-08-24T00:00:00Z","updatedAt":"2026-08-24T00:00:00Z",
                "user":{{"ingameName":"Tenno","status":"ingame"}}}}"#
        );
        serde_json::from_str(&json).expect("order parses")
    }

    #[test]
    fn the_same_order_is_judged_once() {
        let mut judged = Judged::new();

        assert!(judged.fresh(&order("a", 20)));
        assert!(!judged.fresh(&order("a", 20)));
    }

    #[test]
    fn an_order_that_moved_its_price_is_a_new_chance() {
        let mut judged = Judged::new();
        judged.fresh(&order("a", 20));

        assert!(judged.fresh(&order("a", 14)));
    }

    #[test]
    fn what_falls_off_the_end_stops_being_remembered() {
        let mut judged = Judged::new();
        judged.fresh(&order("first", 20));
        for step in 0..REMEMBERED {
            judged.fresh(&order(&format!("filler{step}"), 20));
        }

        assert_eq!(judged.held.len(), REMEMBERED);
        assert!(judged.fresh(&order("first", 20)));
    }

    fn reading(settled: Option<f64>, traded: Option<u32>) -> Reading {
        Reading {
            traded,
            ours: settled.map(|platinum| median_core::Fair {
                platinum,
                volume: 40,
                window: median_core::NEAR,
                spread: 0.0,
            }),
            ingame: None,
            bid: None,
            looked: None,
            prospect: median_core::Prospect::default(),
            falling: false,
        }
    }

    fn chance(entry: u32, worth: Option<u32>, volume: Option<u32>) -> Chance {
        Chance {
            item: "/Lotus/Gram".into(),
            entry,
            worth,
            volume,
            per_trade: 1,
            copies: 1,
            listed: false,
            falling: false,
        }
    }

    #[test]
    fn a_copy_goes_for_the_dearer_of_what_it_settled_at_and_what_is_asked() {
        assert_eq!(
            goes_for(&reading(Some(120.0), Some(30)), Some(90)),
            Some(120)
        );
        assert_eq!(
            goes_for(&reading(Some(120.0), Some(30)), Some(140)),
            Some(140)
        );
    }

    #[test]
    fn one_of_the_two_is_enough_to_price_a_copy() {
        assert_eq!(goes_for(&reading(Some(120.4), Some(30)), None), Some(120));
        assert_eq!(goes_for(&reading(None, Some(30)), Some(90)), Some(90));
        assert_eq!(goes_for(&reading(None, None), None), None);
    }

    #[test]
    fn a_bargain_on_a_pair_that_trades_is_not_turned_down() {
        assert_eq!(
            passed(&chance(60, Some(100), Some(48)), &Policy::default()),
            None
        );
    }

    #[test]
    fn a_pair_nobody_trades_is_turned_down_on_its_volume() {
        let policy = Policy::default();

        assert_eq!(
            passed(&chance(60, Some(100), Some(4)), &policy),
            Some(Reason::ThinVolume)
        );
        assert_eq!(
            passed(&chance(60, Some(100), None), &policy),
            Some(Reason::ThinVolume)
        );
    }

    #[test]
    fn an_estimate_that_promises_too_little_is_turned_down_on_the_floor() {
        let policy = Policy::default();

        assert_eq!(
            passed(&chance(95, Some(100), Some(48)), &policy),
            Some(Reason::BelowFloor)
        );
        // A small margin on an expensive copy still passes the profit floor.
        assert_eq!(passed(&chance(900, Some(1_000), Some(48)), &policy), None);
    }

    #[test]
    fn a_copy_of_unknown_worth_is_turned_down_on_the_floor() {
        assert_eq!(
            passed(&chance(60, None, Some(48)), &Policy::default()),
            Some(Reason::BelowFloor)
        );
    }

    #[test]
    fn one_variant_standing_does_not_cover_another() {
        let mine = [standing("primed_flow", "mod_rank=0", false, 20)];
        let bare = Variant {
            rank: Some(0),
            ..Variant::default()
        };
        let maxed = Variant {
            rank: Some(10),
            ..Variant::default()
        };

        assert!(listed(&mine, "primed_flow", &bare));
        assert!(!listed(&mine, "primed_flow", &maxed));
        assert!(!listed(&mine, "primed_continuity", &bare));
    }

    #[test]
    fn the_agents_own_sale_leaves_room_for_a_bid_and_the_persons_does_not() {
        let maxed = Variant {
            rank: Some(10),
            ..Variant::default()
        };
        let agents = [standing("archon_flow", "mod_rank=10", true, 88)];
        let persons = [account::Standing {
            by_agent: false,
            ..standing("archon_stretch", "mod_rank=10", true, 99)
        }];

        assert!(!listed(&agents, "archon_flow", &maxed));
        assert!(listed(&persons, "archon_stretch", &maxed));
    }

    #[test]
    fn a_pair_the_scheduler_answered_is_no_longer_asked_for_by_name() {
        let mut glimpses = Glimpses::new();

        assert!(glimpses.glimpsed("hespar_set", cue("hespar_set", 40)));
        assert!(!glimpses.glimpsed("hespar_set", cue("hespar_set", 35)));
        assert_eq!(glimpses.needs().len(), 1);
        // The latest order is the one evaluated once the book arrives.
        assert_eq!(glimpses.cues["hespar_set"].platinum, 35);

        assert!(glimpses.answered("hespar_set"));
        assert!(glimpses.needs().is_empty());
        assert!(glimpses.cues.is_empty());
        assert!(!glimpses.answered("hespar_set"));
    }

    fn cue(slug: &str, platinum: u32) -> Cue {
        Cue {
            item_id: "id".into(),
            slug: slug.into(),
            of: Variant::default(),
            side: Side::Sell,
            platinum,
            lot: 1,
            from: Source::Flow,
        }
    }

    #[test]
    fn the_hour_is_written_down_once_it_is_up() {
        let mut funnel = Funnel::new();
        let start = funnel.since;

        funnel.count(Source::Flow, "placed", start + 10);
        funnel.count(Source::Book, "thin-volume", start + 20);
        assert_eq!(funnel.counts.len(), 2);

        funnel.count(Source::Book, "placed", start + TALLIED);
        assert!(funnel.counts.is_empty());
        assert_eq!(funnel.since, start + TALLIED);
    }
}
