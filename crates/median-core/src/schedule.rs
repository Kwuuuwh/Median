use std::collections::BTreeMap;

use crate::market::FRESH;
use crate::time::{DAY, Instant};

/// Minimum seconds between two market requests for the same item.
pub const FLOOR: Instant = 20;

/// Refresh interval of an item's closed trades, in seconds.
pub const TRADES_KEEP: Instant = DAY;

/// Age assumed for an item that was never requested, in seconds.
const NEVER: f64 = 1.0e9;

/// Part of the app that requests market data. Each holder has one set of needs; removing the
/// holder removes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Holder {
    /// Inventory: every item the account holds.
    Inventory,
    /// Orders page: every item the account has an order for.
    Orders,
    /// Agent: the pairs it has open bids on.
    Agent,
    /// Agent offers: the items the agent holds for resale.
    Offers,
    /// Window: the items it currently shows.
    Screen,
    /// Order stream: pairs with an opportunity that the market cache cannot evaluate yet.
    Flow,
    /// Roadmap: the parts to buy on the rows the window shows.
    Plan,
    /// Reward screen: the reward cards shown while the player picks.
    Rewards,
    /// Syndicates page: the items vendors sell for syndicate standing.
    Syndicates,
    /// Market heat map: every pair traded often and expensively enough for the agent to trade.
    Heat,
}

/// Reason a pair needs market data; sets the maximum age of that data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interest {
    /// The user is waiting for it: the reward screen, an open item card.
    Waiting,
    /// Shown in a list in the window.
    Shown,
    /// The account has an open order on it, or holds a copy bought for resale.
    Committed,
    /// The order stream received an order on it that the agent wants to evaluate.
    Glimpsed,
    /// Traded often and expensively enough for the agent to trade.
    Watched,
    /// Traded often enough, but recently with no margin left for the agent; checked occasionally.
    Sampled,
    /// The account holds it, and it is not shown.
    Held,
    /// A vendor sells it for syndicate standing the account has.
    Offered,
}

impl Interest {
    /// Maximum age of the order book data for this interest, in seconds; while the agent is on,
    /// `Watched` uses `FRESH`.
    pub fn keeps(self, agent_on: bool) -> Instant {
        match self {
            Interest::Waiting => 0,
            Interest::Shown => 300,
            Interest::Committed => 120,
            Interest::Glimpsed => 30,
            Interest::Watched if agent_on => FRESH,
            Interest::Watched => 900,
            Interest::Held | Interest::Sampled => 1_800,
            Interest::Offered => 3_600,
        }
    }

    /// Whether the interest needs closed trades as well as the order book.
    fn trades(self) -> bool {
        matches!(
            self,
            Interest::Committed | Interest::Glimpsed | Interest::Watched | Interest::Sampled
        )
    }
}

/// Market data one holder needs for one pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Need {
    pub slug: String,
    /// Variant key as the market cache spells it; empty for the item as a whole.
    pub variant: String,
    pub interest: Interest,
}

/// Request state of an item's data in the market cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Kept {
    /// When its order book was last requested, whether or not a response arrived.
    pub book: Option<Instant>,
    /// When its closed trades were last requested.
    pub trades: Option<Instant>,
    /// Whether the order stream received an order for the item since its order book was
    /// requested.
    pub stirred: bool,
}

/// Which market data of an item to request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fetch {
    /// Its open orders (the order book).
    Book,
    /// Its closed trades.
    Trades,
}

/// One market request to make; `overdue` is the data age divided by its maximum age, due at 1.0
/// or more.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub slug: String,
    pub fetch: Fetch,
    /// Maximum order book age of the strictest interest behind it, in seconds.
    pub keeps: Instant,
    pub overdue: f64,
    /// Whether the user is waiting for it.
    pub waiting: bool,
}

impl Ask {
    /// Whether the data is at or past its maximum age.
    pub fn due(&self) -> bool {
        self.overdue >= 1.0
    }
}

/// Needs of every holder for one item, merged.
#[derive(Debug, Clone, Copy)]
struct Wanted {
    keeps: Instant,
    waiting: bool,
    trades: bool,
}

/// Age of data requested at `asked` divided by `keeps`; `None` within `FLOOR` seconds of the last
/// request. Data never requested counts as `NEVER` seconds old; `stirred` data counts as at least
/// `keeps` old.
fn overdue(asked: Option<Instant>, keeps: Instant, stirred: bool, now: Instant) -> Option<f64> {
    let Some(asked) = asked else {
        return Some(NEVER / (keeps.max(1) as f64));
    };
    let age = now.saturating_sub(asked);
    if age < FLOOR {
        return None;
    }
    let age = match stirred {
        true => age.max(keeps),
        false => age,
    };
    Some(age as f64 / keeps.max(1) as f64)
}

/// Every request that can be made now, in priority order: due requests the user is waiting for,
/// then due requests by strictness with trades before order books, then the most overdue.
pub fn queue(
    needs: &[Need],
    kept: &dyn Fn(&str) -> Kept,
    agent_on: bool,
    now: Instant,
) -> Vec<Ask> {
    let mut items: BTreeMap<&str, Wanted> = BTreeMap::new();
    for need in needs {
        let keeps = need.interest.keeps(agent_on);
        let waiting = need.interest == Interest::Waiting;
        let trades = need.interest.trades();
        items
            .entry(need.slug.as_str())
            .and_modify(|wanted| {
                wanted.keeps = wanted.keeps.min(keeps);
                wanted.waiting |= waiting;
                wanted.trades |= trades;
            })
            .or_insert(Wanted {
                keeps,
                waiting,
                trades,
            });
    }

    let mut asks: Vec<Ask> = Vec::new();
    for (slug, wanted) in items {
        let known = kept(slug);
        if let Some(overdue) = overdue(known.book, wanted.keeps, known.stirred, now) {
            asks.push(Ask {
                slug: slug.to_owned(),
                fetch: Fetch::Book,
                keeps: wanted.keeps,
                overdue,
                waiting: wanted.waiting,
            });
        }
        if wanted.trades
            && let Some(overdue) = overdue(known.trades, TRADES_KEEP, false, now)
        {
            asks.push(Ask {
                slug: slug.to_owned(),
                fetch: Fetch::Trades,
                keeps: wanted.keeps,
                overdue,
                waiting: false,
            });
        }
    }

    asks.sort_by(|one, other| {
        let pressing = |ask: &Ask| ask.waiting && ask.due();
        let strictness = |ask: &Ask| match ask.due() {
            true => ask.keeps,
            false => Instant::MAX,
        };
        let trades = |ask: &Ask| ask.due() && ask.fetch == Fetch::Trades;
        pressing(other)
            .cmp(&pressing(one))
            .then(strictness(one).cmp(&strictness(other)))
            .then(trades(other).cmp(&trades(one)))
            .then(other.overdue.total_cmp(&one.overdue))
            .then(one.slug.cmp(&other.slug))
    });
    asks
}

/// Highest-priority request from `queue`; `None` when no item can be requested yet.
pub fn next(
    needs: &[Need],
    kept: &dyn Fn(&str) -> Kept,
    agent_on: bool,
    now: Instant,
) -> Option<Ask> {
    queue(needs, kept, agent_on, now).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn need(slug: &str, interest: Interest) -> Need {
        Need {
            slug: slug.into(),
            variant: String::new(),
            interest,
        }
    }

    fn asked(at: Instant) -> impl Fn(&str) -> Kept {
        move |_: &str| Kept {
            book: Some(at),
            trades: Some(at),
            stirred: false,
        }
    }

    fn slugs(asks: &[Ask]) -> Vec<&str> {
        asks.iter().map(|ask| ask.slug.as_str()).collect()
    }

    #[test]
    fn an_item_asked_a_moment_ago_is_left_alone_whoever_wants_it() {
        let asks = queue(
            &[need("primed_flow", Interest::Waiting)],
            &asked(1_000),
            true,
            1_010,
        );

        assert!(asks.is_empty());
    }

    #[test]
    fn what_the_person_waits_on_goes_before_anything_more_overdue() {
        let kept = |slug: &str| Kept {
            book: Some(match slug {
                "card" => 9_970,
                _ => 0,
            }),
            ..Kept::default()
        };
        let asks = queue(
            &[
                need("held", Interest::Held),
                need("card", Interest::Waiting),
            ],
            &kept,
            true,
            10_000,
        );

        assert_eq!(slugs(&asks), vec!["card", "held"]);
    }

    #[test]
    fn due_items_go_by_how_strict_their_interest_is() {
        let asks = queue(
            &[
                need("held", Interest::Held),
                need("watched", Interest::Watched),
                need("committed", Interest::Committed),
            ],
            &|slug: &str| Kept {
                book: (slug == "committed").then_some(9_000),
                trades: Some(10_000),
                stirred: false,
            },
            true,
            10_000,
        );

        assert_eq!(slugs(&asks), vec!["committed", "watched", "held"]);
    }

    #[test]
    fn overdue_is_the_age_over_how_old_the_price_may_grow() {
        let asks = queue(
            &[need("shown", Interest::Shown), need("held", Interest::Held)],
            &asked(10_000 - 600),
            false,
            10_000,
        );

        let book = |slug: &str| {
            asks.iter()
                .find(|ask| ask.slug == slug && ask.fetch == Fetch::Book)
                .map(|ask| ask.overdue)
        };
        assert_eq!(book("shown"), Some(2.0));
        assert_eq!(book("held"), Some(600.0 / 1_800.0));
    }

    #[test]
    fn a_strict_interest_past_due_goes_before_a_loose_one_far_past_due() {
        let kept = |slug: &str| Kept {
            book: Some(match slug {
                "watched" => 10_000 - 3_600,
                _ => 10_000 - 2 * 86_400,
            }),
            trades: Some(10_000),
            stirred: false,
        };
        let asks = queue(
            &[
                need("held", Interest::Held),
                need("watched", Interest::Watched),
            ],
            &kept,
            false,
            10_000,
        );

        assert_eq!(slugs(&asks), vec!["watched", "held"]);
    }

    #[test]
    fn due_trades_are_not_starved_by_books_always_past_due() {
        let kept = |slug: &str| Kept {
            book: Some(10_000 - 3_600),
            trades: Some(match slug {
                "stale" => 10_000 - TRADES_KEEP - 60,
                _ => 10_000 - 3_600,
            }),
            stirred: false,
        };
        let asks = queue(
            &[
                need("fresh", Interest::Watched),
                need("stale", Interest::Watched),
                need("held", Interest::Held),
            ],
            &kept,
            true,
            10_000,
        );

        let first = &asks[0];
        assert_eq!((first.slug.as_str(), first.fetch), ("stale", Fetch::Trades));
        assert!(
            asks.iter()
                .position(|ask| ask.slug == "held" && ask.fetch == Fetch::Book)
                < asks
                    .iter()
                    .position(|ask| ask.fetch == Fetch::Trades && ask.slug == "fresh")
        );
    }

    #[test]
    fn something_is_always_asked_about_while_anything_is_wanted() {
        let asks = queue(
            &[need("held", Interest::Held)],
            &asked(10_000 - 60),
            false,
            10_000,
        );

        assert_eq!(slugs(&asks), vec!["held"]);
        assert!(asks[0].overdue < 1.0);
    }

    #[test]
    fn needs_on_the_variants_of_one_item_ask_once_by_the_strictest() {
        let mut maxed = need("primed_flow", Interest::Committed);
        maxed.variant = "mod_rank=10".into();
        let asks = queue(
            &[need("primed_flow", Interest::Held), maxed],
            &asked(10_000 - 240),
            false,
            10_000,
        );

        let books: Vec<&Ask> = asks.iter().filter(|ask| ask.fetch == Fetch::Book).collect();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].overdue, 2.0);
    }

    #[test]
    fn the_flow_carrying_an_order_makes_the_book_due() {
        let asks = queue(
            &[need("primed_flow", Interest::Held)],
            &|_: &str| Kept {
                book: Some(10_000 - 60),
                trades: Some(10_000),
                stirred: true,
            },
            false,
            10_000,
        );

        assert_eq!(asks[0].overdue, 1.0);
    }

    #[test]
    fn a_watched_pair_keeps_as_long_as_a_decision_trusts_a_look_only_while_the_agent_runs() {
        assert_eq!(Interest::Watched.keeps(true), FRESH);
        assert!(Interest::Watched.keeps(false) > FRESH);
    }

    #[test]
    fn closed_trades_are_asked_for_only_where_a_decision_reads_them() {
        let asks = queue(
            &[
                need("held", Interest::Held),
                need("watched", Interest::Watched),
            ],
            &|_: &str| Kept {
                book: Some(10_000),
                trades: None,
                stirred: false,
            },
            true,
            10_000,
        );

        assert_eq!(
            asks.iter()
                .filter(|ask| ask.fetch == Fetch::Trades)
                .map(|ask| ask.slug.as_str())
                .collect::<Vec<_>>(),
            vec!["watched"]
        );
    }
}
