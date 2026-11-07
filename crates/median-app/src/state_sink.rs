use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use median_core::{
    Account as CoreAccount, Activity, Agent as CoreAgent, Alarm as CoreAlarm, Alert as CoreAlert,
    Capped as CoreCapped, Catalog as CoreCatalog, Changed, Collector as CoreCollector,
    Currency as CoreCurrency, Cycle as CoreCycle, CycleSide, Cycling as CoreCycling,
    Detail as CoreDetail, Domain, Era as CoreEra, Feeds as CoreFeeds,
    FissureKind as CoreFissureKind, Game as CoreGame, Holding, Instant as CoreInstant, Inventory,
    Membership as CoreMembership, Offered as CoreOffered, Part as CorePart, PickReason,
    Polarity as CorePolarity, Policy as CorePolicy, Reached as CoreReached,
    Recognition as CoreRecognition, Reset as CoreReset, Rewards as CoreRewards, Riven as CoreRiven,
    RivenPick as CoreRivenPick, RivenStat as CoreRivenStat, Shelf as CoreShelf, Standing, State,
    Status as CoreStatus, Swap as CoreSwap, SwapPart as CoreSwapPart, TaskReport,
    TaskState as CoreTaskState, Unread, UpdateChannel as CoreUpdateChannel, Vendor as CoreVendor,
    Watch as CoreWatch, Worldstate as CoreWorldstate, changes, seconds,
};
use median_engine::StateSink;
use median_ipc::{
    Account, AccountChanged, Agent, AgentChanged, Alarm, Alert, AppState, Capped, Catalog,
    CatalogActivity, CatalogChanged, CatalogPart, Collector, CollectorChanged, Currency, Cycle,
    CycleCard, CycleKey, CycleRoll, Cycling, Deadline, Detail, Era, Feeds, Fissure, FissureKind,
    Game, GameChanged, Holdings, Instant, InventoryChanged, Market, MarketChanged, Membership,
    MembershipChanged, Offer, Offered, Owned, Phase, Piece, Polarity, Policy, PolicyChanged, Price,
    QuarantineChanged, Reached, Recognition, ReleaseChannel, Reset, Rewards, Riven, RivenPick,
    RivenStat, SettingsChanged, Shelf, Swap, SwapPart, Task, TaskState, TasksChanged, Vendor,
    Visit, Watch, Whisper, WorldChanged, Worldstate,
};
use tauri::AppHandle;
use tauri_specta::Event;
use tokio::sync::watch;

/// Latest UI state snapshot, shared with query commands.
#[derive(Clone)]
pub struct SharedState(pub Arc<RwLock<AppState>>);

/// Whether warframe.market shows the account as in game, broadcast to subscribers on change.
#[derive(Clone)]
pub struct Presence(Arc<watch::Sender<bool>>);

impl Default for Presence {
    fn default() -> Self {
        Self(Arc::new(watch::channel(false).0))
    }
}

impl Presence {
    /// Subscribes to the presence; the receiver wakes on every change.
    pub fn watch(&self) -> watch::Receiver<bool> {
        self.0.subscribe()
    }

    /// Stores the current presence; subscribers wake only when the value changed.
    fn set(&self, in_game: bool) {
        self.0
            .send_if_modified(|was| std::mem::replace(was, in_game) != in_game);
    }
}

/// Holdings last sent to the window; `step` counts the updates.
#[derive(Clone)]
pub struct Shown {
    pub step: u32,
    pub inventory: Arc<Inventory>,
}

/// Latest holdings, shared with the command that returns the whole list to the window.
#[derive(Clone)]
pub struct SharedHoldings(Arc<RwLock<Shown>>);

impl SharedHoldings {
    /// Starts at step 0 with the inventory restored from the account store.
    pub fn new(inventory: Arc<Inventory>) -> Self {
        Self(Arc::new(RwLock::new(Shown { step: 0, inventory })))
    }

    /// Returns a copy of the current holdings and their step.
    pub fn read(&self) -> Shown {
        self.0.read().expect("holdings lock poisoned").clone()
    }

    /// Count of each item the account owns, summed over all its variants.
    pub fn counted(&self) -> HashMap<String, u32> {
        let mut counts: HashMap<String, u32> = HashMap::new();
        for holding in &self.read().inventory.holdings {
            *counts.entry(holding.item.clone()).or_default() += holding.count;
        }
        counts
    }

    /// Replaces the holdings with `inventory`; returns the new step and the holdings that changed.
    fn advance(&self, inventory: &Arc<Inventory>) -> (u32, Vec<Holding>) {
        let mut shown = self.0.write().expect("holdings lock poisoned");
        let moved = changes(&shown.inventory.holdings, &inventory.holdings);
        shown.step += 1;
        shown.inventory = Arc::clone(inventory);
        (shown.step, moved)
    }
}

/// Maps core state to the UI contract.
pub fn to_ipc(state: &State) -> AppState {
    AppState {
        locale: state.settings.locale.as_code().to_string(),
        accent: state.settings.accent.as_code().to_string(),
        worldstate: state.world.worldstate.as_ref().map(to_worldstate),
        shelves: state.world.shelves.iter().map(to_shelf).collect(),
        quarantined: state.quarantined,
        catalog: to_catalog(&state.catalog),
        collector: to_collector(&state.collector),
        tasks: state.tasks.reports().iter().map(to_task).collect(),
        account: to_account(&state.account, &state.inventory, &state.game),
        market: to_market(&state.market),
        membership: membership(&state.membership, state.settings.follow_game),
        policy: to_policy(&state.settings.policy),
        agent: to_agent(&state.agent),
        game: to_game(&state.game),
        alert: to_alert(&state.settings.alert),
        watches: state.settings.watches.iter().map(to_watch).collect(),
        update_channel: state.settings.update_channel.map(to_release_channel),
    }
}

/// Maps a release channel updates come from to the UI contract.
pub fn to_release_channel(channel: CoreUpdateChannel) -> ReleaseChannel {
    match channel {
        CoreUpdateChannel::Stable => ReleaseChannel::Stable,
        CoreUpdateChannel::Preview => ReleaseChannel::Preview,
    }
}

/// Maps a release channel from the UI contract; `None` for `Dev`.
pub fn from_release_channel(channel: ReleaseChannel) -> Option<CoreUpdateChannel> {
    match channel {
        ReleaseChannel::Stable => Some(CoreUpdateChannel::Stable),
        ReleaseChannel::Preview => Some(CoreUpdateChannel::Preview),
        ReleaseChannel::Dev => None,
    }
}

/// Maps one card of the relic reward screen to the UI contract.
fn to_offered(offered: &CoreOffered) -> Offered {
    Offered {
        item: offered.item.clone(),
        name: offered.name.clone(),
        image: offered.image.clone(),
        platinum: offered.platinum,
        ingame: offered.ingame,
        per_day_tenths: offered.per_day_tenths,
        ducats: offered.ducats,
        own: offered.own,
        favourite: offered.favourite,
        kit: offered
            .kit
            .iter()
            .map(|part| Owned {
                image: part.image.clone(),
                count: part.count,
                need: part.need,
                this: part.this,
            })
            .collect(),
        whole: offered.whole,
        reached: offered.reached.map(|reached| match reached {
            CoreReached::Built => Reached::Built,
            CoreReached::Mastered => Reached::Mastered,
        }),
    }
}

/// Maps the state read from the game log to the UI contract.
fn to_game(game: &CoreGame) -> Game {
    Game {
        listening: game.listening,
        shared: game.shared,
        whispers: game
            .whispers
            .iter()
            .map(|whisper| Whisper {
                player: whisper.player.clone(),
                at: whisper.at.max(0) as u32,
            })
            .collect(),
        swaps: game.swaps.iter().map(to_swap).collect(),
        rewards: to_rewards(&game.rewards),
        cycling: to_cycling(&game.cycling),
        node: game.node.clone(),
        running_since: game.running_since.map(instant),
        recognition: match game.recognition {
            CoreRecognition::Unchecked => Recognition::Unchecked,
            CoreRecognition::Ready => Recognition::Ready,
            CoreRecognition::Failed => Recognition::Failed,
        },
    }
}

/// Maps the relic reward screen to the UI contract.
fn to_rewards(rewards: &CoreRewards) -> Rewards {
    match rewards {
        CoreRewards::Gone => Rewards::Gone,
        CoreRewards::Reading => Rewards::Reading,
        CoreRewards::Read { cards, fresh } => Rewards::Read {
            cards: cards.iter().map(to_offered).collect(),
            fresh: *fresh,
        },
    }
}

/// Maps one in-game trade to the UI contract.
pub fn to_swap(swap: &CoreSwap) -> Swap {
    Swap {
        at: swap.at.max(0) as u32,
        partner: swap.partner.clone(),
        given: swap.given.iter().map(to_swap_part).collect(),
        taken: swap.taken.iter().map(to_swap_part).collect(),
        outcome: swap.outcome.as_code().to_owned(),
        platinum: swap.platinum_moved(),
        buying: swap.buying(),
    }
}

fn to_swap_part(part: &CoreSwapPart) -> SwapPart {
    SwapPart {
        name: part.name.clone(),
        count: part.count,
        kind: part.kind.as_code().to_owned(),
        rank: part.rank,
    }
}

/// Maps the sound alert settings to the UI contract.
pub fn to_alert(alert: &CoreAlert) -> Alert {
    Alert {
        whisper: alert.whisper,
        volume: alert.volume,
    }
}

/// Maps the sound alert settings from the UI contract, capping the volume at 100.
pub fn from_alert(alert: &Alert) -> CoreAlert {
    CoreAlert {
        whisper: alert.whisper,
        volume: alert.volume.min(100),
    }
}

/// Maps a watch to the UI contract.
pub fn to_watch(watch: &CoreWatch) -> Watch {
    match watch {
        CoreWatch::Phase { cycle, phase } => Watch::Phase {
            cycle: cycle_key(*cycle),
            phase: phase.clone(),
        },
        CoreWatch::Arrival { vendor: which } => Watch::Arrival {
            vendor: vendor(*which),
        },
        CoreWatch::Fissure { era, mission, kind } => Watch::Fissure {
            era: era.map(to_era),
            mission: mission.clone(),
            of: kind.map(|kind| match kind {
                CoreFissureKind::Plain => FissureKind::Plain,
                CoreFissureKind::Hard => FissureKind::Hard,
                CoreFissureKind::Storm => FissureKind::Storm,
            }),
        },
    }
}

/// Maps a watch from the UI contract.
pub fn from_watch(watch: &Watch) -> CoreWatch {
    match watch {
        Watch::Phase { cycle, phase } => CoreWatch::Phase {
            cycle: match cycle {
                CycleKey::Cetus => CoreCycle::Cetus,
                CycleKey::Earth => CoreCycle::Earth,
                CycleKey::Vallis => CoreCycle::Vallis,
                CycleKey::Cambion => CoreCycle::Cambion,
                CycleKey::Duviri => CoreCycle::Duviri,
            },
            phase: phase.clone(),
        },
        Watch::Arrival { vendor } => CoreWatch::Arrival {
            vendor: match vendor {
                Vendor::Baro => CoreVendor::Baro,
                Vendor::Varzia => CoreVendor::Varzia,
            },
        },
        Watch::Fissure { era, mission, of } => CoreWatch::Fissure {
            era: era.as_ref().map(|era| match era {
                Era::Lith => CoreEra::Lith,
                Era::Meso => CoreEra::Meso,
                Era::Neo => CoreEra::Neo,
                Era::Axi => CoreEra::Axi,
                Era::Requiem => CoreEra::Requiem,
                Era::Omnia => CoreEra::Omnia,
            }),
            mission: mission.clone(),
            kind: of.as_ref().map(|of| match of {
                FissureKind::Plain => CoreFissureKind::Plain,
                FissureKind::Hard => CoreFissureKind::Hard,
                FissureKind::Storm => CoreFissureKind::Storm,
            }),
        },
    }
}

/// Maps triggered watches to the UI contract.
pub fn to_alarms(alarms: &[CoreAlarm]) -> Vec<Alarm> {
    alarms
        .iter()
        .map(|alarm| Alarm {
            watch: to_watch(&alarm.watch),
            node: alarm.node.clone(),
        })
        .collect()
}

/// Maps the agent's run state and counters to the UI contract.
fn to_agent(agent: &CoreAgent) -> Agent {
    Agent {
        running: agent.running,
        seen: agent.seen.min(u64::from(u32::MAX)) as u32,
        taken: agent.taken.min(u64::from(u32::MAX)) as u32,
        heard: agent.heard.min(u64::from(u32::MAX)) as u32,
        unreachable: agent.unreachable.min(u64::from(u32::MAX)) as u32,
        placed: agent.placed.min(u64::from(u32::MAX)) as u32,
        moved: agent.moved.min(u64::from(u32::MAX)) as u32,
        warm: agent.warm,
        hot: agent.hot,
        flow: agent
            .flow
            .iter()
            .map(|seen| median_ipc::Seen {
                at: seen.at.max(0) as u32,
                item: seen.item.clone(),
                name: seen.name.clone(),
                image: seen.image.clone(),
                category: seen.category.clone(),
                entry: seen.entry,
                outbid: seen.outbid,
                worth: seen.worth,
                passed: seen.passed.map(|reason| reason.as_code().to_owned()),
            })
            .collect(),
        picks: agent.picks.iter().map(to_pick).collect(),
    }
}

fn to_pick(pick: &CoreRivenPick) -> RivenPick {
    RivenPick {
        id: pick.id.clone(),
        at: pick.at.max(0) as u32,
        name: pick.name.clone(),
        riven: to_riven(&pick.riven),
        rank: u32::from(pick.rank),
        seller: pick.seller.clone(),
        status: pick.status.clone(),
        reason: match pick.reason {
            PickReason::Cheap => "cheap",
            PickReason::Unrolled => "unrolled",
        }
        .to_owned(),
        buyout: pick.buyout,
        worth: pick.worth,
        offer: pick.offer,
        whisper: pick.whisper.clone(),
    }
}

/// Maps the selection policy to the UI contract.
fn to_policy(policy: &CorePolicy) -> Policy {
    Policy {
        least_profit: policy.least_profit,
        tied_from_purse: policy.tied_from_purse,
        purse_share: policy.purse_share,
        least_exchanges: policy.least_exchanges,
        least_per_day: policy.least_per_day,
        most_per_position: policy.most_per_position,
        most_tied: policy.most_tied,
        most_per_trade: policy.most_per_trade,
        most_per_order: policy.most_per_order,
        sale_days: policy.sale_days,
        blacklist: policy.blacklist.clone(),
        whitelist: policy
            .whitelist
            .iter()
            .map(|line| Capped {
                item: line.item.clone(),
                most: line.most,
            })
            .collect(),
        rivens: policy.rivens,
        rank_up: policy.rank_up,
        riven_capital: policy.riven_capital,
        riven_share: policy.riven_share,
        riven_margin: policy.riven_margin,
        riven_least_profit: policy.riven_least_profit,
    }
}

/// Maps the selection policy from the UI contract, for the command that sets it.
pub fn from_policy(policy: &Policy) -> CorePolicy {
    CorePolicy {
        least_profit: policy.least_profit,
        tied_from_purse: policy.tied_from_purse,
        purse_share: policy.purse_share,
        least_exchanges: policy.least_exchanges,
        least_per_day: policy.least_per_day,
        most_per_position: policy.most_per_position,
        most_tied: policy.most_tied,
        most_per_trade: policy.most_per_trade,
        most_per_order: policy.most_per_order,
        sale_days: policy.sale_days,
        blacklist: policy.blacklist.clone(),
        whitelist: policy
            .whitelist
            .iter()
            .map(|line| CoreCapped {
                item: line.item.clone(),
                most: line.most,
            })
            .collect(),
        rivens: policy.rivens,
        rank_up: policy.rank_up,
        riven_capital: policy.riven_capital,
        riven_share: policy.riven_share,
        riven_margin: policy.riven_margin,
        riven_least_profit: policy.riven_least_profit,
    }
}

/// Maps the market membership to the UI contract.
fn membership(held: &CoreMembership, follow: bool) -> Membership {
    let (standing, name) = match &held.standing {
        Standing::Out => ("out", None),
        Standing::In { name } => ("in", name.clone()),
        Standing::Expired => ("expired", None),
    };
    Membership {
        standing: standing.to_owned(),
        name,
        status: held.status.as_code().to_owned(),
        auto: follow,
    }
}

/// Maps the market cache state (own-order reads, failure start) to the UI contract.
fn to_market(market: &median_core::Market) -> Market {
    Market {
        own: market.own,
        failing_since: market.failing_since.map(|since| since.max(0) as u32),
    }
}

fn to_account(account: &CoreAccount, inventory: &Inventory, game: &CoreGame) -> Account {
    Account {
        profile: account.profile.as_ref().map(|profile| profile.name.clone()),
        positions: account.positions,
        read: account.read,
        distinct: account.distinct,
        unreadable: account.unreadable,
        unidentified: account.unidentified,
        moved: account
            .moved
            .map(|at| seconds().saturating_sub(at).max(0) as u32),
        exchanges: median_core::exchanges_left(inventory.purse.trades_left, game),
        platinum: median_core::platinum_left(inventory.purse.platinum, game),
        credits: inventory
            .purse
            .credits
            .map(|credits| credits.min(u64::from(u32::MAX)) as u32),
    }
}

/// Every holding of the account with its equipped flag, plus the step and the digest of the
/// inventory the list was built from.
pub fn to_holdings(shown: &Shown) -> Holdings {
    let carrying = installed(&shown.inventory);
    Holdings {
        step: shown.step,
        digest: shown.inventory.digest().hex(),
        pieces: shown
            .inventory
            .holdings
            .iter()
            .map(|held| to_piece(held, carried(&carrying, held)))
            .collect(),
    }
}

/// Maps one holding to the UI contract.
pub fn to_piece(held: &Holding, equipped: bool) -> Piece {
    Piece {
        item: held.item.clone(),
        variant: held.variant.0.clone(),
        count: held.count,
        detail: held.detail.as_ref().map(to_detail),
        equipped,
    }
}

/// Equipped copies as `(item, variant)` keys, the form a holding is looked up by.
pub fn installed(inventory: &Inventory) -> HashSet<(&str, &str)> {
    inventory
        .equipped
        .iter()
        .map(|copy| (copy.item.as_str(), copy.variant.0.as_str()))
        .collect()
}

/// Whether `held` is one of the equipped copies.
pub fn carried(installed: &HashSet<(&str, &str)>, held: &Holding) -> bool {
    installed.contains(&(held.item.as_str(), held.variant.0.as_str()))
}

fn to_detail(detail: &CoreDetail) -> Detail {
    match detail {
        CoreDetail::Rank(rank) => Detail::Rank {
            rank: u32::from(*rank),
        },
        CoreDetail::Stars(stars) => Detail::Stars {
            stars: u32::from(*stars),
        },
        CoreDetail::Riven { rank, revealed } => Detail::Riven {
            rank: u32::from(*rank),
            revealed: revealed.as_ref().map(to_riven),
        },
    }
}

fn to_cycling(cycling: &CoreCycling) -> Cycling {
    match cycling {
        CoreCycling::Gone => Cycling::Gone,
        CoreCycling::Open {
            cards,
            rolls,
            reading,
            unread,
        } => Cycling::Open {
            cards: cards
                .iter()
                .map(|card| CycleCard {
                    side: match card.side {
                        CycleSide::Chosen => "chosen",
                        CycleSide::Left => "left",
                        CycleSide::Right => "right",
                    }
                    .to_owned(),
                    riven: to_riven(&card.riven),
                    rank: u32::from(card.rank),
                    name: card.name.clone(),
                    fresh: card.fresh,
                })
                .collect(),
            rolls: rolls
                .iter()
                .map(|roll| CycleRoll {
                    kuva: roll.kuva,
                    name: roll.name.clone(),
                })
                .collect(),
            reading: *reading,
            unread: unread.map(|unread| {
                match unread {
                    Unread::NoReader => "noReader",
                    Unread::NotHeld => "notHeld",
                    Unread::Unsupported => "unsupported",
                    Unread::Stats => "stats",
                }
                .to_owned()
            }),
        },
    }
}

fn to_riven(riven: &CoreRiven) -> Riven {
    Riven {
        weapon: riven.weapon.clone(),
        mastery: u32::from(riven.mastery),
        rerolls: riven.rerolls,
        polarity: riven.polarity.map(|polarity| match polarity {
            CorePolarity::Madurai => Polarity::Madurai,
            CorePolarity::Vazarin => Polarity::Vazarin,
            CorePolarity::Naramon => Polarity::Naramon,
        }),
        buffs: riven.buffs.iter().map(to_riven_stat).collect(),
        curses: riven.curses.iter().map(to_riven_stat).collect(),
    }
}

fn to_riven_stat(stat: &CoreRivenStat) -> RivenStat {
    RivenStat {
        tag: stat.tag.clone(),
        roll: stat.roll,
    }
}

fn to_task(report: &TaskReport) -> Task {
    Task {
        name: report.name.clone(),
        state: match &report.state {
            CoreTaskState::Running => TaskState::Running,
            CoreTaskState::Restarting { reason } => TaskState::Restarting {
                reason: reason.clone(),
            },
            CoreTaskState::Stopped { reason } => TaskState::Stopped {
                reason: reason.clone(),
            },
        },
    }
}

fn to_collector(collector: &CoreCollector) -> Collector {
    match collector {
        CoreCollector::Offline => Collector::Offline,
        CoreCollector::Live { heard_at, feeds } => Collector::Live {
            heard: instant(*heard_at),
            feeds: match feeds {
                CoreFeeds::Waiting => Feeds::Waiting,
                CoreFeeds::Up => Feeds::Up,
                CoreFeeds::Refused { reason } => Feeds::Refused {
                    reason: reason.clone(),
                },
            },
        },
        CoreCollector::Outdated { speaks } => Collector::Outdated { speaks: *speaks },
    }
}

fn to_worldstate(world: &CoreWorldstate) -> Worldstate {
    Worldstate {
        at: instant(world.at),
        cycles: world
            .cycles
            .iter()
            .map(|state| Cycle {
                key: cycle_key(state.cycle),
                phases: state
                    .cycle
                    .phases()
                    .iter()
                    .map(|phase| Phase {
                        key: phase.key.to_string(),
                        seconds: phase.seconds as u32,
                    })
                    .collect(),
                phase: state.phase as u32,
                ends: instant(state.ends),
            })
            .collect(),
        fissures: world
            .fissures
            .iter()
            .map(|fissure| Fissure {
                node: fissure.node.clone(),
                era: to_era(fissure.era),
                hard: fissure.hard,
                storm: fissure.storm,
                mission: fissure.mission.clone(),
                expires: instant(fissure.expires),
            })
            .collect(),
        deadlines: world
            .deadlines
            .iter()
            .map(|deadline| Deadline {
                reset: to_reset(deadline.reset),
                at: instant(deadline.at),
            })
            .collect(),
        visits: world
            .visits
            .iter()
            .map(|visit| Visit {
                vendor: vendor(visit.vendor),
                node: visit.node.clone(),
                arrives: instant(visit.arrives),
                departs: instant(visit.departs),
            })
            .collect(),
    }
}

fn cycle_key(cycle: CoreCycle) -> CycleKey {
    match cycle {
        CoreCycle::Cetus => CycleKey::Cetus,
        CoreCycle::Earth => CycleKey::Earth,
        CoreCycle::Vallis => CycleKey::Vallis,
        CoreCycle::Cambion => CycleKey::Cambion,
        CoreCycle::Duviri => CycleKey::Duviri,
    }
}

fn to_era(era: CoreEra) -> Era {
    match era {
        CoreEra::Lith => Era::Lith,
        CoreEra::Meso => Era::Meso,
        CoreEra::Neo => Era::Neo,
        CoreEra::Axi => Era::Axi,
        CoreEra::Requiem => Era::Requiem,
        CoreEra::Omnia => Era::Omnia,
    }
}

fn to_reset(reset: CoreReset) -> Reset {
    match reset {
        CoreReset::Daily => Reset::Daily,
        CoreReset::Sortie => Reset::Sortie,
        CoreReset::Weekly => Reset::Weekly,
    }
}

fn vendor(vendor: CoreVendor) -> Vendor {
    match vendor {
        CoreVendor::Baro => Vendor::Baro,
        CoreVendor::Varzia => Vendor::Varzia,
    }
}

fn to_shelf(shelf: &CoreShelf) -> Shelf {
    Shelf {
        vendor: vendor(shelf.vendor),
        offers: shelf
            .offers
            .iter()
            .map(|offer| Offer {
                item: offer.item.clone(),
                prices: offer
                    .prices
                    .iter()
                    .map(|price| Price {
                        currency: match price.currency {
                            CoreCurrency::Ducats => Currency::Ducats,
                            CoreCurrency::Credits => Currency::Credits,
                            CoreCurrency::Aya => Currency::Aya,
                            CoreCurrency::RegalAya => Currency::RegalAya,
                        },
                        amount: price.amount,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Narrows a timestamp to the IPC `Instant`; 0 when it does not fit.
fn instant(at: CoreInstant) -> Instant {
    Instant::try_from(at).unwrap_or(0)
}

fn to_catalog(catalog: &CoreCatalog) -> Catalog {
    let activity = match &catalog.activity {
        Activity::Idle => CatalogActivity::Idle,
        Activity::Checking => CatalogActivity::Checking,
        Activity::Fetching {
            part, done, total, ..
        } => CatalogActivity::Fetching {
            part: match part {
                CorePart::Database => CatalogPart::Database,
                CorePart::Pictures => CatalogPart::Pictures,
            },
            done: narrow(*done),
            total: narrow(*total),
        },
        Activity::Opening { .. } => CatalogActivity::Opening,
        Activity::Unreachable { reason } => CatalogActivity::Unreachable {
            reason: reason.to_string(),
        },
        Activity::Unsupported { schema } => CatalogActivity::Unsupported { schema: *schema },
        Activity::Stale { schema } => CatalogActivity::Stale { schema: *schema },
    };
    Catalog {
        ready: !catalog.is_bare(),
        installed: catalog.installed.map(|version| version.to_string()),
        pack: catalog.pack.map(|version| version.to_string()),
        pending: catalog
            .activity
            .release()
            .map(|release| release.version.to_string()),
        activity,
    }
}

/// Narrows a byte count to `u32`, saturating at `u32::MAX`.
fn narrow(bytes: u64) -> u32 {
    u32::try_from(bytes).unwrap_or(u32::MAX)
}

/// Copies each published state into the shared snapshot and emits one event per changed domain.
pub struct TauriStateSink {
    app: AppHandle,
    shared: SharedState,
    holdings: SharedHoldings,
    presence: Presence,
    /// Unix seconds of the last `AgentChanged`; agent counters change faster than the window
    /// can repaint, so they are emitted at most once a second. Atomic: `publish` has `&self`.
    told: AtomicI64,
    /// Running flag and placed-order count at the last `AgentChanged`. A change in either is
    /// the user's own action, so it is emitted at once.
    ran: Mutex<(bool, u32)>,
}

impl TauriStateSink {
    /// Creates a sink that emits over `app` and updates `shared`, `holdings` and `presence`.
    pub fn new(
        app: AppHandle,
        shared: SharedState,
        holdings: SharedHoldings,
        presence: Presence,
    ) -> Self {
        Self {
            app,
            shared,
            holdings,
            presence,
            told: AtomicI64::new(0),
            ran: Mutex::new((false, 0)),
        }
    }
}

impl StateSink for TauriStateSink {
    fn publish(&self, state: &State, changed: Changed) {
        let snapshot = to_ipc(state);
        *self.shared.0.write().expect("state lock poisoned") = snapshot.clone();
        self.presence
            .set(state.membership.status == CoreStatus::Ingame);

        if changed.has(Domain::Settings) {
            let _ = SettingsChanged {
                locale: snapshot.locale.clone(),
                accent: snapshot.accent.clone(),
                alert: snapshot.alert.clone(),
                watches: snapshot.watches.clone(),
                update_channel: snapshot.update_channel,
            }
            .emit(&self.app);
        }
        if changed.has(Domain::World) {
            let _ = WorldChanged {
                worldstate: snapshot.worldstate,
                shelves: snapshot.shelves,
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Quarantine) {
            let _ = QuarantineChanged {
                quarantined: snapshot.quarantined,
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Catalog) {
            if snapshot.catalog.ready {
                crate::window::data_ready(&self.app);
            }
            let _ = CatalogChanged {
                catalog: snapshot.catalog,
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Collector) {
            let _ = CollectorChanged {
                collector: snapshot.collector,
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Tasks) {
            let _ = TasksChanged {
                tasks: snapshot.tasks,
            }
            .emit(&self.app);
        }
        // The trades left today are part of the account view, and an in-game trade read from
        // the game log changes that count before the next inventory dump does.
        if changed.has(Domain::Account) || changed.has(Domain::Game) {
            let _ = AccountChanged {
                account: snapshot.account,
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Market) {
            let _ = MarketChanged {
                market: snapshot.market,
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Agent) {
            let now_at = (
                state.agent.running,
                state.agent.placed.min(u64::from(u32::MAX)) as u32,
            );
            let moved = {
                let mut ran = self.ran.lock().expect("the run lock is poisoned");
                let moved = *ran != now_at;
                *ran = now_at;
                moved
            };

            let now = seconds();
            if moved || now - self.told.load(Ordering::Relaxed) >= 1 {
                self.told.store(now, Ordering::Relaxed);
                let _ = AgentChanged {
                    agent: to_agent(&state.agent),
                }
                .emit(&self.app);
            }
        }
        if changed.has(Domain::Game) {
            let _ = GameChanged {
                game: to_game(&state.game),
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Policy) {
            let _ = PolicyChanged {
                policy: to_policy(&state.settings.policy),
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Membership) {
            let _ = MembershipChanged {
                membership: membership(&state.membership, state.settings.follow_game),
            }
            .emit(&self.app);
        }
        if changed.has(Domain::Inventory) {
            let (step, moved) = self.holdings.advance(&state.inventory);
            let _ = InventoryChanged {
                step,
                digest: state.inventory.digest().hex(),
                moved: {
                    let carrying = installed(&state.inventory);
                    moved
                        .iter()
                        .map(|held| to_piece(held, carried(&carrying, held)))
                        .collect()
                },
            }
            .emit(&self.app);
        }
    }
}
