use crate::fills::{Rates, depth};
use crate::market::{NEAR, THIN, Traded, credible_ask, fair, falling};
use crate::policy::{Chance, LEAST, Means, Permit, Policy, Reason, copies};
use crate::prospect::prospect;
use crate::time::{DAY, HOUR, Instant};

/// Market data the agent evaluates for one item-variant pair at one moment.
#[derive(Debug, Clone, Copy)]
pub struct Sight<'a> {
    pub item: &'a str,
    pub hours: &'a [Traded],
    pub days: &'a [Traded],
    /// Highest bid from a player who is in game.
    pub top_bid: Option<u32>,
    /// Price equal to the highest in-game bid.
    pub lead: u32,
    /// Lowest in-game ask that a listed copy would queue behind.
    pub held_ask: Option<u32>,
    pub sellers: Option<u32>,
    /// Copies an in-game seller is seen to sell in one trade.
    pub lot: u32,
    /// Market data of the form a copy is resold in, when that differs from the form bought, e.g.
    /// the mod after ranking it up.
    pub resale: Option<Resale<'a>>,
    pub now: Instant,
}

/// Closed trades and order book of the form a copy is resold in.
#[derive(Debug, Clone, Copy)]
pub struct Resale<'a> {
    pub hours: &'a [Traded],
    pub days: &'a [Traded],
    pub top_bid: Option<u32>,
    pub held_ask: Option<u32>,
    pub sellers: Option<u32>,
}

/// Bid the agent would place, with the policy permit to place it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    pub platinum: u32,
    pub copies: u32,
    /// Copies per trade.
    pub lot: u32,
    /// Expected value of the bid: platinum per trade or per day, depending on the strategy.
    pub promised: u32,
    pub permit: Permit,
}

/// Pricing rule that turns a `Sight` into a bid.
pub trait Strategy {
    fn quote(&self, policy: &Policy, means: &Means, sight: &Sight<'_>) -> Result<Quote, Reason>;
}

/// Expected value of one copy of the pair, and the market facts every bid on it uses.
#[derive(Debug, Clone, Copy)]
struct Reading {
    worth: u32,
    volume: u32,
    falling: bool,
}

impl Reading {
    /// Reading of `sight`; `Err` with the reason when there is nothing to bid on.
    fn of(policy: &Policy, sight: &Sight<'_>) -> Result<Self, Reason> {
        let sold = sight.resale.unwrap_or(Resale {
            hours: sight.hours,
            days: sight.days,
            top_bid: sight.top_bid,
            held_ask: sight.held_ask,
            sellers: sight.sellers,
        });
        let traded: Vec<Traded> = sold.hours.iter().chain(sold.days).copied().collect();
        let ours = fair(&traded, sight.now, THIN);
        let ingame = credible_ask(sold.held_ask, ours.as_ref());
        let near = |buckets: &[Traded]| -> Option<u64> {
            let within: Vec<&Traded> = buckets
                .iter()
                .filter(|bucket| bucket.starts >= sight.now - NEAR)
                .collect();
            (!within.is_empty()).then(|| within.iter().map(|bucket| bucket.volume).sum())
        };
        let volume = near(sold.hours)
            .or_else(|| near(sold.days))
            .ok_or(Reason::NoReading)?;
        ingame.ok_or(Reason::NoSeller)?;
        let worth = prospect(
            sold.hours,
            sold.days,
            sight.now,
            sold.top_bid,
            ingame,
            sold.sellers,
        )
        .worth(policy.sale_days)
        .ok_or(Reason::NoSeller)?;
        Ok(Self {
            worth,
            volume: u32::try_from(volume).unwrap_or(u32::MAX),
            falling: falling(sold.hours, sold.days, sight.now),
        })
    }

    /// Bid at `entry` with `lot` copies per trade, checked against the policy, with the expected
    /// value that `promised` computes.
    fn quote(
        &self,
        policy: &Policy,
        means: &Means,
        sight: &Sight<'_>,
        (entry, lot): (u32, u32),
        promised: impl FnOnce(&Chance) -> u32,
    ) -> Result<Quote, Reason> {
        let chance = Chance {
            item: sight.item.to_owned(),
            entry,
            worth: Some(self.worth),
            volume: Some(self.volume),
            per_trade: lot,
            copies: copies(policy, means, entry).max(lot),
            listed: false,
            falling: self.falling,
        };
        let permit = Permit::issue(policy, &chance, means)?;
        let carried = chance.carried(policy);
        Ok(Quote {
            platinum: entry,
            copies: chance.copies.clamp(1, policy.most_per_order.max(carried)),
            lot: carried,
            promised: promised(&chance),
            permit,
        })
    }

    /// Highest bid that still leaves the policy's margin on one copy.
    fn ceiling(&self, policy: &Policy, lot: u32) -> u32 {
        self.worth.saturating_sub(policy.least_per_copy(lot))
    }
}

/// Strategy that values a bid by its expected platinum per day, from the measured fill rates of
/// bids at the same depth.
#[derive(Debug, Clone, Copy)]
pub struct Measured<'a> {
    pub rates: &'a Rates,
}

impl Measured<'_> {
    /// Expected platinum per day of a bid at `entry` with `lot` copies per trade: the chance of a
    /// fill within a day times the margin on the lot; 0 when the depth is unknown.
    fn daily(&self, reading: &Reading, sight: &Sight<'_>, entry: u32, lot: u32) -> f64 {
        let Some(depth) = depth(entry, sight.hours, sight.now) else {
            return 0.0;
        };
        self.rates.chance(depth, DAY)
            * f64::from(reading.worth.saturating_sub(entry))
            * f64::from(lot)
    }

    /// Bid with `lot` copies per trade at the top bid price, or at the highest price the margin
    /// allows when the top is higher; `Err(BelowFloor)` when it is expected to bring less than 1
    /// platinum a day.
    fn best(
        &self,
        policy: &Policy,
        means: &Means,
        sight: &Sight<'_>,
        reading: &Reading,
        lot: u32,
    ) -> Result<Quote, Reason> {
        let entry = sight.lead.min(reading.ceiling(policy, lot));
        if entry <= LEAST {
            return Err(Reason::BelowFloor);
        }
        let daily = self.daily(reading, sight, entry, lot);
        if daily < 1.0 {
            return Err(Reason::BelowFloor);
        }
        reading.quote(policy, means, sight, (entry, lot), |_| daily.round() as u32)
    }
}

/// Seconds before the daily reset (midnight UTC) from which the minimum profit starts to decrease.
const CLOSING: Instant = 4 * HOUR;

/// Policy with the minimum profit lowered linearly over the last `CLOSING` seconds of the game
/// day, down to half at midnight UTC: trades left unused at the reset are lost, but before then
/// they are worth the full minimum.
fn closing(policy: &Policy, now: Instant) -> Policy {
    let left = u64::try_from((DAY - now.rem_euclid(DAY)).min(CLOSING)).unwrap_or(0);
    let full = u64::from(policy.least_profit);
    let floor = full.div_ceil(2) + (full - full.div_ceil(2)) * left / CLOSING as u64;
    Policy {
        least_profit: u32::try_from(floor).unwrap_or(u32::MAX),
        ..policy.clone()
    }
}

impl Strategy for Measured<'_> {
    fn quote(&self, policy: &Policy, means: &Means, sight: &Sight<'_>) -> Result<Quote, Reason> {
        let policy = &closing(policy, sight.now);
        let reading = Reading::of(policy, sight)?;
        // When the full lot cannot be bid on, e.g. the platinum does not cover it, bid on a
        // single copy.
        match self.best(policy, means, sight, &reading, sight.lot.max(1)) {
            Err(_) if sight.lot > 1 => self.best(policy, means, sight, &reading, 1),
            quoted => quoted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simplest strategy: bid equal to the highest in-game bid, capped below a copy's expected
    /// value. The agent uses `Measured`; this one tests the `Reading` both share.
    #[derive(Debug, Clone, Copy, Default)]
    struct Baseline;

    impl Strategy for Baseline {
        fn quote(
            &self,
            policy: &Policy,
            means: &Means,
            sight: &Sight<'_>,
        ) -> Result<Quote, Reason> {
            let reading = Reading::of(policy, sight)?;
            let entry = sight.lead.min(reading.ceiling(policy, 1));
            reading.quote(policy, means, sight, (entry, 1), |chance| {
                u32::try_from(chance.profit(policy).unwrap_or(0).max(0)).unwrap_or(0)
            })
        }
    }

    const NOW: Instant = 1_000 * DAY;

    fn bucket(ago: Instant, volume: u64, median: f64) -> Traded {
        Traded {
            starts: NOW - ago,
            volume,
            median,
            low: median,
        }
    }

    fn means(free: u32) -> Means {
        Means {
            held: Some(free),
            free: Some(free),
            tied: 0,
            exchanges: Some(20),
            orders: 0,
            hand: 0,
        }
    }

    fn policy() -> Policy {
        Policy {
            tied_from_purse: true,
            ..Policy::default()
        }
    }

    fn sight<'a>(hours: &'a [Traded], top_bid: u32, ask: u32) -> Sight<'a> {
        Sight {
            item: "/Lotus/Upgrades/Mods/Rifle/LeadedGas",
            hours,
            days: &[],
            top_bid: Some(top_bid),
            lead: top_bid,
            held_ask: Some(ask),
            sellers: Some(2),
            lot: 1,
            resale: None,
            now: NOW,
        }
    }

    #[test]
    fn the_baseline_stands_level_with_the_dearest_bid() {
        let hours: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 60.0)).collect();

        let quote = Baseline
            .quote(&policy(), &means(500), &sight(&hours, 30, 60))
            .expect("a bid");

        assert_eq!(quote.platinum, 30);
    }

    #[test]
    fn a_listing_over_the_trades_promises_nothing() {
        let hours: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 15.0)).collect();

        assert_eq!(
            Baseline.quote(&policy(), &means(500), &sight(&hours, 10, 40)),
            Err(Reason::BelowFloor)
        );
    }

    fn rates() -> Rates {
        let stood = |depth: u8, filled: bool| {
            (
                crate::fills::Depth::of(100 - u32::from(depth) * 10 - 5, 100.0),
                crate::fills::Exposure {
                    lasted: 10 * HOUR,
                    filled,
                },
            )
        };
        Rates::of(&crate::fills::fills(&[
            stood(2, true),
            stood(2, false),
            stood(5, false),
            stood(5, false),
        ]))
    }

    #[test]
    fn a_measured_bid_stands_level_with_the_top() {
        let hours: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 100.0)).collect();
        let rates = rates();

        // A top bid of 71 is at the depth where the measured bids were filled.
        let quote = Measured { rates: &rates }
            .quote(&policy(), &means(500), &sight(&hours, 71, 100))
            .expect("a bid");

        assert_eq!(quote.platinum, 71);
    }

    #[test]
    fn a_top_dearer_than_the_margin_allows_leaves_the_bid_at_the_ceiling() {
        let hours: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 100.0)).collect();
        let rates = rates();
        let dear = sight(&hours, 99, 100);
        let ceiling = Reading::of(&policy(), &dear)
            .expect("a reading")
            .ceiling(&policy(), 1);

        let quote = Measured { rates: &rates }
            .quote(&policy(), &means(500), &dear)
            .expect("a bid at the ceiling, not a refusal");

        assert!(ceiling < 99, "the top is over what the margin allows");
        assert_eq!(quote.platinum, ceiling);
    }

    #[test]
    fn a_depth_that_never_fills_is_no_bid() {
        let hours: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 100.0)).collect();
        let never = Rates::of(&crate::fills::fills(&[(
            crate::fills::Depth::of(20, 100.0),
            crate::fills::Exposure {
                lasted: 10 * HOUR,
                filled: false,
            },
        )]));

        assert_eq!(
            Measured { rates: &never }.quote(&policy(), &means(500), &sight(&hours, 30, 100)),
            Err(Reason::BelowFloor)
        );
    }

    #[test]
    fn a_bid_asks_for_the_lot_a_seller_hands_over_when_the_platinum_covers_it() {
        let hours: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 100.0)).collect();
        let rates = rates();
        let bulk = Sight {
            lot: 6,
            ..sight(&hours, 71, 100)
        };
        let roomy = Policy {
            most_per_position: 1_000,
            ..policy()
        };

        let quote = Measured { rates: &rates }
            .quote(&roomy, &means(1_000), &bulk)
            .expect("a bid");
        assert_eq!((quote.lot, quote.copies), (6, 6));

        let short = Measured { rates: &rates }
            .quote(&roomy, &means(100), &bulk)
            .expect("a single copy");
        assert_eq!((short.lot, short.copies), (1, 1));
    }

    #[test]
    fn a_copy_bought_to_rank_up_is_worth_what_the_ranked_copy_fetches() {
        let unranked: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 10.0)).collect();
        let ranked: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 55.0)).collect();
        let plain = sight(&unranked, 8, 10);

        assert_eq!(
            Baseline.quote(&policy(), &means(500), &plain),
            Err(Reason::BelowFloor)
        );
        let fused = Sight {
            resale: Some(Resale {
                hours: &ranked,
                days: &[],
                top_bid: Some(30),
                held_ask: Some(55),
                sellers: Some(4),
            }),
            ..plain
        };
        let quote = Baseline
            .quote(&policy(), &means(500), &fused)
            .expect("a bid on the ranked worth");
        assert_eq!(quote.platinum, 8);
    }

    #[test]
    fn a_flip_has_to_earn_less_as_the_day_runs_out() {
        let policy = Policy {
            least_profit: 12,
            ..policy()
        };

        assert_eq!(closing(&policy, NOW).least_profit, 12);
        assert_eq!(closing(&policy, NOW + DAY / 2).least_profit, 12);
        assert_eq!(closing(&policy, NOW + DAY - 2 * HOUR).least_profit, 9);
        assert_eq!(closing(&policy, NOW + DAY - 1).least_profit, 6);
    }

    #[test]
    fn nobody_selling_in_game_is_no_bid() {
        let hours: Vec<Traded> = (1..=48).map(|ago| bucket(ago * HOUR, 2, 15.0)).collect();
        let blind = Sight {
            held_ask: None,
            ..sight(&hours, 10, 40)
        };

        assert_eq!(
            Baseline.quote(&policy(), &means(500), &blind),
            Err(Reason::NoSeller)
        );
    }
}
