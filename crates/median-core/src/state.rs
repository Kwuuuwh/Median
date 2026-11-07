use std::sync::Arc;

use crate::{
    Catalog,
    account::{Account, Inventory},
    agent::Agent,
    collector::Collector,
    game::Game,
    market::Market,
    membership::Membership,
    settings::Settings,
    tasks::Tasks,
    trade::Shelf,
    worldstate::Worldstate,
};

/// Latest fetched world state and the offers of visiting vendors.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct World {
    /// Latest world state; `None` until the first fetch.
    pub worldstate: Option<Worldstate>,
    /// Offers of the rotating vendors currently visiting.
    pub shelves: Vec<Shelf>,
}

/// Application state, one slice per domain; slices holding collections are wrapped in `Arc`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// User settings, saved to disk.
    pub settings: Settings,
    /// Latest fetched world state and vendor offers.
    pub world: Arc<World>,
    /// Number of inputs rejected at the boundary (see `Event::InputQuarantined`).
    pub quarantined: u32,
    /// Installed game data versions and any check or install in progress.
    pub catalog: Arc<Catalog>,
    /// Connection state of the collector.
    pub collector: Collector,
    /// Latest state reported by each background task.
    pub tasks: Arc<Tasks>,
    /// State of the account the collector reports on.
    pub account: Account,
    /// Inventory from the latest inventory dump.
    pub inventory: Arc<Inventory>,
    /// State of the market cache.
    pub market: Market,
    /// Whether the account is signed in to warframe.market, and its status shown there.
    pub membership: Membership,
    /// Whether the agent runs, and counters of the order stream it evaluated.
    pub agent: Agent,
    /// State read from the game log.
    pub game: Game,
}
