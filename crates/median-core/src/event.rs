use crate::account::AccountEvent;
use crate::agent::AgentEvent;
use crate::catalog::CatalogEvent;
use crate::collector::CollectorEvent;
use crate::domain::Domain;
use crate::game::GameEvent;
use crate::locale::Locale;
use crate::market::MarketEvent;
use crate::membership::MembershipEvent;
use crate::policy::Policy;
use crate::settings::Accent;
use crate::settings::Alert;
use crate::settings::UpdateChannel;
use crate::tasks::TaskReport;
use crate::trade::Shelf;
use crate::watch::Watch;
use crate::worldstate::Worldstate;

/// Download progress of one subsystem, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Subsystem doing the download.
    pub domain: Domain,
    /// Bytes received so far.
    pub done: u64,
    /// Bytes expected in total.
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The interface language was set.
    LocaleSet(Locale),
    /// The accent colour was set.
    AccentSet(Accent),
    /// World state and the offers of visiting vendors were fetched.
    WorldReceived {
        worldstate: Worldstate,
        shelves: Vec<Shelf>,
    },
    /// An input was rejected at the boundary.
    InputQuarantined,
    /// A subsystem reported download progress.
    Progressed(Progress),
    /// Game data (catalog) event.
    Catalog(CatalogEvent),
    /// Collector connection event.
    Collector(CollectorEvent),
    /// A background task reported its state.
    TaskReported(TaskReport),
    /// An inventory dump was received.
    Account(AccountEvent),
    /// Market cache event.
    Market(MarketEvent),
    /// warframe.market account event.
    Membership(MembershipEvent),
    /// The limits the trading agents act within were set.
    PolicySet(Policy),
    /// The sound alert settings were set.
    AlertSet(Alert),
    /// The release channel updates come from was chosen.
    UpdateChannelSet(UpdateChannel),
    /// A world state watch was turned on (`on: true`) or off.
    WatchSet { watch: Watch, on: bool },
    /// The market status was set to follow the game (`true`) or to be set by the user (`false`).
    FollowGameSet(bool),
    /// Agent event.
    Agent(AgentEvent),
    /// Game log event.
    Game(GameEvent),
}
