use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::dto::{
    Account, Agent, Alarm, Alert, Catalog, Collector, Game, Market, Membership, Piece, Policy,
    Priced, ReleaseChannel, Shelf, Task, Watch, Worldstate,
};

/// Emitted when a setting changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct SettingsChanged {
    pub locale: String,
    pub accent: String,
    pub alert: Alert,
    pub watches: Vec<Watch>,
    pub update_channel: Option<ReleaseChannel>,
}

/// Emitted when one or more world watches fire.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct Rang {
    pub alarms: Vec<Alarm>,
}

/// Emitted when the world state or the visiting traders' stock changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct WorldChanged {
    pub worldstate: Option<Worldstate>,
    pub shelves: Vec<Shelf>,
}

/// Emitted when an input is rejected at the boundary.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct QuarantineChanged {
    pub quarantined: u32,
}

/// Emitted when the game data installation state or its current activity changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct CatalogChanged {
    pub catalog: Catalog,
}

/// Emitted when the collector connection state changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct CollectorChanged {
    pub collector: Collector,
}

/// Emitted when a background task reports a new state.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TasksChanged {
    pub tasks: Vec<Task>,
}

/// Emitted when the account status changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct AccountChanged {
    pub account: Account,
}

/// Emitted when the account's inventory changes; carries only the changed entries.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct InventoryChanged {
    /// Number of inventory changes since the application started.
    pub step: u32,
    /// Digest of the inventory after these entries are applied.
    pub digest: String,
    /// Entries that differ from the previous inventory; a count of zero means the entry was
    /// removed.
    pub moved: Vec<Piece>,
}

/// Emitted when the market cache status changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct MarketChanged {
    pub market: Market,
}

/// Emitted when a new market response repriced inventory entries or sets; carries only the
/// repriced rows.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct PricesChanged {
    /// Inventory entries the response repriced.
    pub held: Vec<Priced>,
    /// Sets the response repriced.
    pub sets: Vec<Priced>,
}

/// Emitted when a fresh order book arrives for a pair the account has an open order on.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct BookMoved {
    pub slug: String,
}

/// Emitted when the agent state changes: at most once a second, except that a start, a stop or
/// a newly placed order is emitted at once.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct AgentChanged {
    pub agent: Agent,
}

/// Emitted when the game state read from the game log or the screen changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct GameChanged {
    pub game: Game,
}

/// Emitted when the agents' trading limits change.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct PolicyChanged {
    pub policy: Policy,
}

/// Emitted while an update downloads, at each whole percent.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct UpdateProgressed {
    /// Bytes received so far.
    pub downloaded: u32,
    /// Size of the download in bytes; `None` when the server does not give it.
    pub total: Option<u32>,
}

/// Emitted when the account's warframe.market sign-in state or status changes.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct MembershipChanged {
    pub membership: Membership,
}
