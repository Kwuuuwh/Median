use crate::riven::{Riven, RivenTrades};

/// One stat of a listed riven, as its seller entered it.
#[derive(Debug, Clone, PartialEq)]
pub struct ListedStat {
    pub tag: String,
    /// Value as the game displays it: `274.4` for +274.4%, `1.58` for x1.58.
    pub shown: f64,
    pub positive: bool,
}

/// Riven auction lot listed at a fixed buyout price.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    /// warframe.market auction id.
    pub id: String,
    pub price: u32,
    pub rank: u8,
    pub stats: Vec<ListedStat>,
}

/// Minimum likeness for a lot to count in pricing a riven; two shared stats out of three pass.
const PRICING_LIKENESS: f64 = 0.6;

/// Similar lots needed to price a riven by their quartiles, with outliers removed.
const MANY: usize = 8;

/// Minimum similar lots needed to price a riven from lots at all.
const FEW: usize = 3;

/// Tukey fence factor: a price more than this many interquartile ranges beyond the quartiles is
/// an outlier.
const FENCE: f64 = 1.5;

/// Ask ceiling, in deviations above last week's median trade price.
const CEILING_DEVIATIONS: f64 = 2.0;

/// Source of a price band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// At least `MANY` similar lots: their quartiles, with outliers removed.
    Lots,
    /// `FEW` to `MANY` similar lots: their median, clamped to last week's trade bounds.
    FewLots,
    /// Fewer than `FEW` similar lots: last week's trades of the weapon's rivens.
    Trades,
}

/// Price band of similar rivens: low, median and high, in platinum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub low: f64,
    pub middle: f64,
    pub high: f64,
    /// Number of similar lots the band is computed from.
    pub lots: usize,
    pub basis: Basis,
    /// Whether the ceiling from last week's trades lowered the band.
    pub is_capped: bool,
}

/// Lot similar to a riven.
#[derive(Debug, Clone, PartialEq)]
pub struct Analog {
    /// warframe.market auction id.
    pub id: String,
    /// Share of stats it has in common with the riven on the same side (buff or curse).
    pub likeness: f64,
}

/// Price bounds from last week's trades of a weapon's rivens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeBounds {
    pub low: f64,
    pub median: f64,
    /// Median trade price plus `CEILING_DEVIATIONS` deviations.
    pub ceiling: f64,
}

impl TradeBounds {
    pub fn of(trades: &RivenTrades) -> Self {
        Self {
            low: trades.low,
            median: trades.median,
            ceiling: trades.median + CEILING_DEVIATIONS * trades.deviation,
        }
    }
}

/// Riven auction lots listed for one weapon.
#[derive(Debug, Clone, PartialEq)]
pub struct Asks {
    listed: Vec<Listed>,
}

impl Asks {
    pub fn new(listed: Vec<Listed>) -> Self {
        Self { listed }
    }

    pub fn lots(&self) -> usize {
        self.listed.len()
    }

    /// Price band of `riven` from similar lots, bounded by last week's `traded`; `None` with
    /// fewer than `FEW` similar lots and no trades.
    pub fn band(&self, riven: &Riven, traded: Option<TradeBounds>) -> Option<Band> {
        let mut alike: Vec<f64> = self
            .listed
            .iter()
            .filter(|lot| likeness(riven, lot) >= PRICING_LIKENESS)
            .map(|lot| f64::from(lot.price))
            .collect();
        alike.sort_by(f64::total_cmp);
        let ceiling = traded.map_or(f64::INFINITY, |traded| traded.ceiling);
        let floor = traded.map_or(0.0, |traded| traded.low);

        if alike.len() >= MANY {
            let kept = within_fences(&alike);
            let [low, middle, high] = [0.25, 0.5, 0.75].map(|at| quantile(&kept, at));
            return Some(Band {
                low: low.min(ceiling),
                middle: middle.min(ceiling),
                high: high.min(ceiling),
                lots: kept.len(),
                basis: Basis::Lots,
                is_capped: high > ceiling,
            });
        }
        if alike.len() >= FEW {
            let middle = quantile(&alike, 0.5);
            let high = alike[alike.len() - 1];
            return Some(Band {
                low: alike[0].clamp(floor.min(ceiling), ceiling),
                middle: middle.clamp(floor.min(ceiling), ceiling),
                high: high.min(ceiling),
                lots: alike.len(),
                basis: Basis::FewLots,
                is_capped: high > ceiling,
            });
        }
        traded.map(|traded| Band {
            low: traded.low,
            middle: traded.median,
            high: traded.ceiling,
            lots: alike.len(),
            basis: Basis::Trades,
            is_capped: false,
        })
    }

    /// Up to `count` lots most similar to `riven`, cheapest first on equal likeness; lots with no
    /// stat in common are left out.
    pub fn analogs(&self, riven: &Riven, count: usize) -> Vec<Analog> {
        let mut near: Vec<(f64, &Listed)> = self
            .listed
            .iter()
            .map(|lot| (likeness(riven, lot), lot))
            .filter(|(likeness, _)| *likeness > 0.0)
            .collect();
        near.sort_by(|(one_likeness, one), (other_likeness, other)| {
            other_likeness
                .total_cmp(one_likeness)
                .then(one.price.cmp(&other.price))
        });
        near.into_iter()
            .take(count)
            .map(|(likeness, lot)| Analog {
                id: lot.id.clone(),
                likeness,
            })
            .collect()
    }
}

/// Stats `lot` shares with `riven` on the same side, divided by the larger of the two stat
/// counts; 0 when both have none.
fn likeness(riven: &Riven, lot: &Listed) -> f64 {
    let shared = lot
        .stats
        .iter()
        .filter(|stat| {
            let side = if stat.positive {
                &riven.buffs
            } else {
                &riven.curses
            };
            side.iter().any(|held| held.tag == stat.tag)
        })
        .count();
    let larger = lot.stats.len().max(riven.buffs.len() + riven.curses.len());
    match larger {
        0 => 0.0,
        _ => shared as f64 / larger as f64,
    }
}

/// Prices of `sorted` within Tukey's fences, computed on the logarithms of the prices.
fn within_fences(sorted: &[f64]) -> Vec<f64> {
    let logs: Vec<f64> = sorted.iter().map(|price| price.max(1.0).ln()).collect();
    let (lower, upper) = (quantile(&logs, 0.25), quantile(&logs, 0.75));
    let spread = upper - lower;
    let fences = (lower - FENCE * spread)..=(upper + FENCE * spread);
    sorted
        .iter()
        .zip(&logs)
        .filter(|(_, log)| fences.contains(log))
        .map(|(price, _)| *price)
        .collect()
}

/// Quantile `at` (0.0 to 1.0) of `sorted`, interpolated linearly between neighbours; 0 for an
/// empty slice.
fn quantile(sorted: &[f64], at: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let position = at * (sorted.len() - 1) as f64;
    let below = position.floor() as usize;
    let above = position.ceil() as usize;
    sorted[below] + (sorted[above] - sorted[below]) * (position - below as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riven::Stat;

    const CD: &str = "WeaponCritDamageMod";
    const MS: &str = "WeaponFireIterationsMod";
    const TOX: &str = "WeaponToxinDamageMod";
    const ZOOM: &str = "WeaponZoomFovMod";
    const IMP: &str = "WeaponImpactDamageMod";

    fn riven(buffs: &[&str], curses: &[&str]) -> Riven {
        let stats = |tags: &[&str]| {
            tags.iter()
                .map(|tag| Stat {
                    tag: (*tag).to_owned(),
                    roll: 0,
                })
                .collect()
        };
        Riven {
            weapon: "Soma".into(),
            mastery: 8,
            rerolls: 0,
            polarity: None,
            buffs: stats(buffs),
            curses: stats(curses),
        }
    }

    fn lot(id: &str, price: u32, buffs: &[&str], curses: &[&str]) -> Listed {
        fn side<'a>(tags: &'a [&str], positive: bool) -> impl Iterator<Item = ListedStat> + 'a {
            tags.iter().map(move |tag| ListedStat {
                tag: (*tag).to_owned(),
                shown: 1.0,
                positive,
            })
        }
        Listed {
            id: id.into(),
            price,
            rank: 8,
            stats: side(buffs, true).chain(side(curses, false)).collect(),
        }
    }

    fn bounds() -> TradeBounds {
        TradeBounds {
            low: 100.0,
            median: 300.0,
            ceiling: 1_300.0,
        }
    }

    #[test]
    fn likeness_counts_stats_on_the_same_side_over_the_larger_count() {
        let held = riven(&[CD, MS, TOX], &[ZOOM]);
        assert_eq!(likeness(&held, &lot("a", 1, &[CD, MS, TOX], &[ZOOM])), 1.0);
        assert_eq!(likeness(&held, &lot("b", 1, &[CD, MS], &[])), 0.5);
        assert_eq!(likeness(&held, &lot("c", 1, &[CD, MS, ZOOM], &[TOX])), 0.5);
    }

    #[test]
    fn many_lots_alike_price_a_riven_with_outliers_cut() {
        let mut listed: Vec<Listed> = [285, 300, 350, 400, 450, 500, 520, 557, 600]
            .iter()
            .enumerate()
            .map(|(at, price)| lot(&at.to_string(), *price, &[CD, MS, TOX], &[ZOOM]))
            .collect();
        listed.push(lot("silly", 65_000, &[CD, MS, TOX], &[ZOOM]));
        listed.push(lot("other", 2, &[IMP], &[]));
        let band = Asks::new(listed)
            .band(&riven(&[CD, MS, TOX], &[ZOOM]), Some(bounds()))
            .expect("band");
        assert_eq!(band.basis, Basis::Lots);
        assert_eq!(band.lots, 9);
        assert_eq!(band.middle, 450.0);
        assert!(!band.is_capped);
    }

    #[test]
    fn a_few_lots_alike_are_held_within_last_weeks_trades() {
        let listed = vec![
            lot("a", 2_000, &[CD, MS, TOX], &[]),
            lot("b", 2_200, &[CD, MS, TOX], &[]),
            lot("c", 2_500, &[CD, MS, TOX], &[]),
        ];
        let band = Asks::new(listed)
            .band(&riven(&[CD, MS, TOX], &[]), Some(bounds()))
            .expect("band");
        assert_eq!(band.basis, Basis::FewLots);
        assert_eq!(band.middle, 1_300.0);
        assert!(band.is_capped);
    }

    #[test]
    fn too_few_lots_alike_leave_the_price_to_last_weeks_trades() {
        let listed = vec![lot("a", 900, &[CD, MS, TOX], &[])];
        let held = riven(&[CD, MS, TOX], &[]);
        let band = Asks::new(listed.clone())
            .band(&held, Some(bounds()))
            .expect("band");
        assert_eq!(
            (band.basis, band.middle, band.lots),
            (Basis::Trades, 300.0, 1)
        );
        assert_eq!(Asks::new(listed).band(&held, None), None);
    }

    #[test]
    fn analogs_come_most_alike_first_then_cheapest() {
        let listed = vec![
            lot("half", 100, &[CD, MS], &[]),
            lot("dear", 900, &[CD, MS, TOX], &[ZOOM]),
            lot("cheap", 500, &[CD, MS, TOX], &[ZOOM]),
            lot("none", 1, &[IMP], &[]),
        ];
        let ids: Vec<String> = Asks::new(listed)
            .analogs(&riven(&[CD, MS, TOX], &[ZOOM]), 8)
            .into_iter()
            .map(|analog| analog.id)
            .collect();
        assert_eq!(ids, ["cheap", "dear", "half"]);
    }
}
