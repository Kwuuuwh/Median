use crate::lot_pick::PickReason;
use crate::policy::Reason;
use crate::riven::Riven;
use crate::time::Instant;

/// Riven auction lot the agent selected for purchase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RivenPick {
    /// warframe.market auction id.
    pub id: String,
    pub at: Instant,
    /// Riven name as warframe.market spells it, e.g. `crita-visican`.
    pub name: String,
    pub riven: Riven,
    pub rank: u8,
    pub seller: String,
    /// Seller status: `ingame`, `online` or `offline`.
    pub status: String,
    pub reason: PickReason,
    pub buyout: u32,
    /// Expected resale price, in platinum.
    pub worth: u32,
    /// First price offered to the seller, in platinum.
    pub offer: u32,
    /// Message to the seller, ready to paste.
    pub whisper: String,
}

/// One order from the order stream, and the agent's decision on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub at: Instant,
    /// Game item key; empty when the order maps to no catalog item.
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// Catalog category of the item; empty when the catalog has none.
    pub category: String,
    /// Price of one copy for the account: the seller's ask, or 1 platinum above the highest open
    /// bid when the opportunity is to outbid the buyers.
    pub entry: u32,
    /// Whether the opportunity came from a buy order rather than a sell order.
    pub outbid: bool,
    /// Value of one copy according to the market cache.
    pub worth: Option<u32>,
    /// `None` when the agent accepted the order; otherwise the reason it passed.
    pub passed: Option<Reason>,
}

/// Number of entries of `flow` and `picks` kept for the window.
pub const SHOWN: usize = 40;

/// Agent counters over the order stream, and its latest decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Agent {
    /// Whether the agent is running. A running agent places orders; there is no read-only state.
    pub running: bool,
    /// Orders received from the order stream since the application started.
    pub heard: u64,
    /// Of those, orders whose counterparty cannot be traded with right now.
    pub unreachable: u64,
    /// Of those, orders evaluated through to a decision.
    pub seen: u64,
    /// Of those, orders the policy accepted.
    pub taken: u64,
    /// Of those, orders that resulted in a placed order.
    pub placed: u64,
    /// Times one of the account's open orders was repriced or removed.
    pub moved: u64,
    /// Watched items whose market data has been fetched since the application started.
    pub warm: u32,
    /// Items the market heat map watches; 0 until the heat map is built.
    pub hot: u32,
    /// Latest orders from the order stream, newest first; at most `SHOWN`, older ones are dropped.
    pub flow: Vec<Seen>,
    /// Riven lots selected for purchase, newest first; at most `SHOWN`.
    pub picks: Vec<RivenPick>,
}

/// Agent event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    RunningSet(bool),
    Looked(Box<Seen>),
    /// An order was placed.
    Placed,
    /// One of the account's open orders was repriced or removed.
    Moved,
    /// Orders received from the order stream so far, and how many of them have a counterparty
    /// who cannot be traded with.
    StreamCounted {
        carried: u64,
        unreachable: u64,
    },
    /// Watched items whose market data has been fetched (`warm`), out of all watched items (`hot`).
    Warmed {
        warm: u32,
        hot: u32,
    },
    /// A riven lot was selected for purchase.
    Picked(Box<RivenPick>),
    /// A selected lot was bought or dismissed; carries the auction id.
    PickDropped(String),
}

/// Agent state after `event`.
pub fn update(agent: &Agent, event: AgentEvent) -> Agent {
    let mut next = agent.clone();
    match event {
        AgentEvent::RunningSet(running) => next.running = running,
        AgentEvent::Looked(seen) => {
            next.seen += 1;
            if seen.passed.is_none() {
                next.taken += 1;
            }
            next.flow.insert(0, *seen);
            next.flow.truncate(SHOWN);
        }
        AgentEvent::Placed => next.placed += 1,
        AgentEvent::Moved => next.moved += 1,
        AgentEvent::StreamCounted {
            carried,
            unreachable,
        } => {
            next.heard = carried;
            next.unreachable = unreachable;
        }
        AgentEvent::Warmed { warm, hot } => {
            next.warm = warm;
            next.hot = hot;
        }
        AgentEvent::Picked(pick) => {
            next.picks.retain(|held| held.id != pick.id);
            next.picks.insert(0, *pick);
            next.picks.truncate(SHOWN);
        }
        AgentEvent::PickDropped(id) => next.picks.retain(|held| held.id != id),
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(passed: Option<Reason>) -> AgentEvent {
        AgentEvent::Looked(Box::new(Seen {
            at: 1_787_400_000,
            item: "/Lotus/Types/Items/MiscItems/Rubedo".into(),
            name: "Рубедо".into(),
            image: None,
            category: "Ресурсы".into(),
            entry: 18,
            outbid: false,
            worth: Some(24),
            passed,
        }))
    }

    #[test]
    fn a_stopped_agent_is_simply_not_running() {
        assert!(!Agent::default().running);

        let agent = update(&Agent::default(), AgentEvent::RunningSet(true));

        assert!(agent.running);
    }

    #[test]
    fn what_it_took_is_counted_apart_from_what_it_saw() {
        let agent = update(&Agent::default(), seen(Some(Reason::BelowFloor)));
        let agent = update(&agent, seen(None));

        assert_eq!(agent.seen, 2);
        assert_eq!(agent.taken, 1);
    }

    #[test]
    fn what_was_written_is_counted_apart_from_what_passed_the_policy() {
        let agent = update(&Agent::default(), seen(None));
        let agent = update(&agent, AgentEvent::Placed);

        assert_eq!(agent.taken, 1);
        assert_eq!(agent.placed, 1);
    }

    #[test]
    fn the_flow_says_how_much_of_it_had_nobody_to_trade_with() {
        let agent = update(
            &Agent::default(),
            AgentEvent::StreamCounted {
                carried: 1_000,
                unreachable: 480,
            },
        );

        assert_eq!(agent.heard, 1_000);
        assert_eq!(agent.unreachable, 480);
    }

    #[test]
    fn the_flow_keeps_the_newest_and_drops_the_tail() {
        let mut agent = Agent::default();
        for _ in 0..(SHOWN + 5) {
            agent = update(&agent, seen(None));
        }

        assert_eq!(agent.flow.len(), SHOWN);
        assert_eq!(agent.seen, (SHOWN + 5) as u64);
    }

    #[test]
    fn the_warming_is_told_as_it_stands() {
        let agent = update(&Agent::default(), AgentEvent::Warmed { warm: 12, hot: 313 });

        assert_eq!((agent.warm, agent.hot), (12, 313));
    }

    fn picked(id: &str) -> AgentEvent {
        AgentEvent::Picked(Box::new(RivenPick {
            id: id.into(),
            at: 1_787_400_000,
            name: "crita-visican".into(),
            riven: Riven {
                weapon: "/Lotus/Weapons/ClanTech/Bio/BioWeapon".into(),
                mastery: 8,
                rerolls: 0,
                polarity: None,
                buffs: Vec::new(),
                curses: Vec::new(),
            },
            rank: 0,
            seller: "Kaelith".into(),
            status: "ingame".into(),
            reason: PickReason::Cheap,
            buyout: 300,
            worth: 500,
            offer: 250,
            whisper: String::new(),
        }))
    }

    #[test]
    fn a_pick_goes_first_and_leaves_once_dropped() {
        let agent = update(&Agent::default(), picked("a"));
        let agent = update(&agent, picked("b"));
        assert_eq!(agent.picks[0].id, "b");

        let agent = update(&agent, AgentEvent::PickDropped("b".into()));

        assert_eq!(agent.picks.len(), 1);
        assert_eq!(agent.picks[0].id, "a");
    }
}
