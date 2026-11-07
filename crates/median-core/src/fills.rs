use std::collections::BTreeMap;

use crate::market::{NEAR, THIN, Traded, fair};
use crate::time::{DAY, HOUR, Instant};

/// Deepest depth distinguished, in tenths: 5 covers bids at half the median trade price or lower.
const DEEPEST: u8 = 5;

/// How far a bid is below the median trade price of its pair, in tenths of that median, from 0
/// to `DEEPEST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Depth(u8);

impl Depth {
    /// Depth of `bid` below `middle`; a bid at or above `middle` has depth 0.
    pub fn of(bid: u32, middle: f64) -> Self {
        let under = 1.0 - f64::from(bid) / middle;
        Self((under * 10.0).floor().clamp(0.0, f64::from(DEEPEST)) as u8)
    }

    /// Tenths of the median the bid is below; `DEEPEST` also covers every deeper bid.
    pub fn tenths(self) -> u8 {
        self.0
    }
}

/// Depth of `bid` at `at` against the trades of the two days (`NEAR`) before it; `None` when too
/// little traded for a fair price.
pub fn depth(bid: u32, traded: &[Traded], at: Instant) -> Option<Depth> {
    let before: Vec<Traded> = traded
        .iter()
        .filter(|bucket| bucket.starts < at && bucket.starts >= at - NEAR)
        .copied()
        .collect();
    let middle = fair(&before, at, THIN)?.platinum;
    Some(Depth::of(bid, middle))
}

/// Lifetime of one bid as recorded in the account store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidLife {
    /// Each price the bid was set to, with the time it was set, oldest first.
    pub prices: Vec<(Instant, u32)>,
    /// When the bid was closed; `None` while it is open.
    pub ended: Option<Instant>,
    /// Times a seller filled the bid.
    pub fills: Vec<Instant>,
}

/// One interval during which a bid stayed at one price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stretch {
    pub at: Instant,
    pub platinum: u32,
    pub exposure: Exposure,
}

/// Seconds a bid stayed at one price until it was filled or changed, and whether it was filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exposure {
    pub lasted: Instant,
    pub filled: bool,
}

/// Intervals the bid spent at each of its prices; the last interval of an open bid ends at `now`.
pub fn stretches(life: &BidLife, now: Instant) -> Vec<Stretch> {
    let end = life.ended.unwrap_or(now);
    life.prices
        .iter()
        .enumerate()
        .filter_map(|(at, &(from, platinum))| {
            let to = life.prices.get(at + 1).map_or(end, |next| next.0).min(end);
            if to < from {
                return None;
            }
            let fill = life
                .fills
                .iter()
                .copied()
                .filter(|fill| *fill >= from && *fill <= to)
                .min();
            Some(Stretch {
                at: from,
                platinum,
                exposure: Exposure {
                    lasted: fill.unwrap_or(to) - from,
                    filled: fill.is_some(),
                },
            })
        })
        .collect()
}

/// Kaplan-Meier curve of the share of bids still unfilled after each wait. A bid removed unfilled
/// counts as unknown from then on (censored).
#[derive(Debug, Clone, PartialEq)]
pub struct Survival {
    steps: Vec<(Instant, f64)>,
}

impl Survival {
    /// Survival curve of `exposures`.
    pub fn of(exposures: &[Exposure]) -> Self {
        let mut sorted = exposures.to_vec();
        sorted.sort_by_key(|exposure| exposure.lasted);

        let mut standing = sorted.len();
        let mut left = 1.0;
        let mut steps = Vec::new();
        let mut at = 0;
        while at < sorted.len() {
            let lasted = sorted[at].lasted;
            let same = sorted[at..]
                .iter()
                .take_while(|exposure| exposure.lasted == lasted)
                .count();
            let filled = sorted[at..at + same]
                .iter()
                .filter(|exposure| exposure.filled)
                .count();
            if filled > 0 {
                left *= 1.0 - filled as f64 / standing as f64;
                steps.push((lasted, left));
            }
            standing -= same;
            at += same;
        }
        Self { steps }
    }

    /// Probability that a bid is filled within `wait` seconds.
    pub fn filled_within(&self, wait: Instant) -> f64 {
        let left = self
            .steps
            .iter()
            .take_while(|(lasted, _)| *lasted <= wait)
            .last()
            .map_or(1.0, |(_, left)| *left);
        1.0 - left
    }
}

/// Fill statistics of the bids at one depth.
#[derive(Debug, Clone, PartialEq)]
pub struct Fills {
    pub depth: Depth,
    pub stretches: usize,
    pub filled: usize,
    /// Probability that a bid at this depth is filled within a day.
    pub within_day: f64,
    /// Fills per hour that bids at this depth were open.
    pub per_hour: f64,
    /// Total hours bids at this depth were open.
    pub hours: f64,
}

/// Fill statistics per depth, shallowest first.
pub fn fills(exposures: &[(Depth, Exposure)]) -> Vec<Fills> {
    let mut by_depth: BTreeMap<Depth, Vec<Exposure>> = BTreeMap::new();
    for (depth, exposure) in exposures {
        by_depth.entry(*depth).or_default().push(*exposure);
    }
    by_depth
        .into_iter()
        .map(|(depth, of)| {
            let filled = of.iter().filter(|exposure| exposure.filled).count();
            let hours: f64 = of
                .iter()
                .map(|exposure| exposure.lasted as f64 / HOUR as f64)
                .sum();
            Fills {
                depth,
                stretches: of.len(),
                filled,
                within_day: Survival::of(&of).filled_within(DAY),
                per_hour: match hours > 0.0 {
                    true => filled as f64 / hours,
                    false => 0.0,
                },
                hours,
            }
        })
        .collect()
}

/// Fill rate per hour for each depth, measured from past bids at that depth.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rates {
    per_hour: BTreeMap<Depth, f64>,
}

impl Rates {
    /// Measured fill rates, with adjacent depths pooled so that no depth has a lower rate than a
    /// deeper one.
    pub fn of(fills: &[Fills]) -> Self {
        let mut pools: Vec<Pool> = Vec::new();
        for fills in fills.iter().filter(|fills| fills.hours > 0.0) {
            pools.push(Pool {
                depths: vec![fills.depth],
                filled: fills.filled as f64,
                hours: fills.hours,
            });
            while let [.., shallower, deeper] = pools.as_slice()
                && shallower.rate() < deeper.rate()
                && let Some(deeper) = pools.pop()
                && let Some(shallower) = pools.last_mut()
            {
                shallower.absorb(deeper);
            }
        }
        Self {
            per_hour: pools
                .iter()
                .flat_map(|pool| pool.depths.iter().map(|depth| (*depth, pool.rate())))
                .collect(),
        }
    }

    /// Probability that a bid at `depth` is filled within `wait` seconds, from the rate of the
    /// nearest measured depth at or below `depth`; 0 when every measured depth is shallower.
    pub fn chance(&self, depth: Depth, wait: Instant) -> f64 {
        let rate = self
            .per_hour
            .range(depth..)
            .next()
            .map_or(0.0, |(_, rate)| *rate);
        1.0 - (-rate * wait as f64 / HOUR as f64).exp()
    }
}

/// Adjacent depths whose fills are pooled into one rate.
struct Pool {
    depths: Vec<Depth>,
    filled: f64,
    hours: f64,
}

impl Pool {
    fn rate(&self) -> f64 {
        self.filled / self.hours
    }

    fn absorb(&mut self, deeper: Pool) {
        self.depths.extend(deeper.depths);
        self.filled += deeper.filled;
        self.hours += deeper.hours;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: Instant = 1_000 * DAY;

    fn bucket(ago: Instant, volume: u64, median: f64) -> Traded {
        Traded {
            starts: NOW - ago,
            volume,
            median,
            low: median,
        }
    }

    fn stood(lasted: Instant, filled: bool) -> Exposure {
        Exposure { lasted, filled }
    }

    #[test]
    fn a_bid_is_measured_in_tenths_under_the_middle() {
        assert_eq!(Depth::of(100, 95.0).tenths(), 0);
        assert_eq!(Depth::of(67, 95.0).tenths(), 2);
        assert_eq!(Depth::of(11, 40.0).tenths(), 5);
    }

    #[test]
    fn a_bid_is_placed_against_the_trades_before_it() {
        let traded = [bucket(2 * HOUR, 12, 95.0), bucket(-HOUR, 50, 40.0)];

        assert_eq!(depth(67, &traded, NOW), Some(Depth(2)));
    }

    #[test]
    fn nothing_traded_places_nothing() {
        assert_eq!(depth(20, &[bucket(3 * DAY, 12, 15.0)], NOW), None);
    }

    #[test]
    fn each_price_a_bid_stood_at_is_its_own_stretch() {
        let life = BidLife {
            prices: vec![(0, 11), (HOUR, 14), (3 * HOUR, 15)],
            ended: Some(5 * HOUR),
            fills: vec![4 * HOUR],
        };

        assert_eq!(
            stretches(&life, NOW)
                .iter()
                .map(|stretch| (stretch.platinum, stretch.exposure))
                .collect::<Vec<_>>(),
            [
                (11, stood(HOUR, false)),
                (14, stood(2 * HOUR, false)),
                (15, stood(HOUR, true)),
            ]
        );
    }

    #[test]
    fn a_bid_still_standing_runs_up_to_now() {
        let life = BidLife {
            prices: vec![(NOW - HOUR, 20)],
            ended: None,
            fills: Vec::new(),
        };

        assert_eq!(stretches(&life, NOW)[0].exposure, stood(HOUR, false));
    }

    #[test]
    fn a_bid_taken_down_unfilled_is_not_counted_as_never_filling() {
        let curve = Survival::of(&[stood(10, true), stood(20, false), stood(30, true)]);

        assert!((curve.filled_within(10) - 1.0 / 3.0).abs() < 1e-9);
        assert!((curve.filled_within(30) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn nothing_filled_means_no_chance() {
        let curve = Survival::of(&[stood(10, false), stood(20, false)]);

        assert_eq!(curve.filled_within(DAY), 0.0);
    }

    #[test]
    fn each_depth_is_counted_apart() {
        let counted = fills(&[
            (Depth(5), stood(DAY, false)),
            (Depth(2), stood(HOUR, true)),
            (Depth(2), stood(3 * HOUR, false)),
        ]);

        assert_eq!(
            counted
                .iter()
                .map(|fills| (fills.depth, fills.stretches, fills.filled))
                .collect::<Vec<_>>(),
            [(Depth(2), 2, 1), (Depth(5), 1, 0)]
        );
        assert!((counted[0].within_day - 0.5).abs() < 1e-9);
        assert!((counted[0].per_hour - 0.25).abs() < 1e-9);
    }

    #[test]
    fn a_depth_never_measured_never_fills() {
        let rates = Rates::of(&fills(&[(Depth(2), stood(HOUR, true))]));

        assert!(rates.chance(Depth(2), HOUR) > 0.6);
        assert_eq!(rates.chance(Depth(5), DAY), 0.0);
    }

    #[test]
    fn a_shallow_depth_never_measured_fills_like_the_nearest_deeper_one() {
        let rates = Rates::of(&fills(&[(Depth(2), stood(HOUR, true))]));

        assert_eq!(rates.chance(Depth(0), HOUR), rates.chance(Depth(2), HOUR));
    }

    #[test]
    fn a_shallow_depth_that_has_not_filled_yet_is_pooled_with_the_deeper_one_that_has() {
        let rates = Rates::of(&fills(&[
            (Depth(0), stood(HOUR, false)),
            (Depth(2), stood(HOUR, true)),
        ]));

        let pooled = 1.0 - (-0.5_f64).exp();
        assert!((rates.chance(Depth(0), HOUR) - pooled).abs() < 1e-9);
        assert!((rates.chance(Depth(2), HOUR) - pooled).abs() < 1e-9);
    }

    #[test]
    fn a_deeper_depth_that_filled_faster_is_pooled_with_the_shallower_one() {
        let rates = Rates::of(&fills(&[
            (Depth(1), stood(4 * HOUR, true)),
            (Depth(3), stood(HOUR, true)),
        ]));

        assert_eq!(rates.chance(Depth(1), HOUR), rates.chance(Depth(3), HOUR));
    }
}
