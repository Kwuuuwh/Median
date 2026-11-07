use crate::appraisal::PoolStat;
use crate::riven::{Riven, Stat};

/// Kuva cost of a reroll, indexed by the number of earlier rerolls; the last value applies to
/// every later reroll.
const REROLL_COSTS: [u32; 10] = [900, 1000, 1200, 1400, 1700, 2000, 2350, 2750, 3150, 3500];

/// Maximum kuva cost of a locked reroll.
const LOCKED_CAP: u32 = 7000;

/// Possible (buffs, curses) counts after an unlocked reroll, assumed equally likely.
const SPLITS: [(usize, usize); 4] = [(2, 0), (2, 1), (3, 0), (3, 1)];

/// Roll value at the top of a stat's range.
const ROLL_TOP: u64 = 1 << 30;

/// Kuva cost of the next reroll after `rerolls` rerolls; doubled when a stat is locked, capped at
/// `LOCKED_CAP`.
pub fn reroll_cost(rerolls: u32, is_locked: bool) -> u32 {
    let index = usize::try_from(rerolls).map_or(REROLL_COSTS.len() - 1, |index| {
        index.min(REROLL_COSTS.len() - 1)
    });
    let cost = REROLL_COSTS[index];
    if is_locked {
        (cost * 2).min(LOCKED_CAP)
    } else {
        cost
    }
}

/// Expected kuva spent until a reroll gives a better riven, when each reroll succeeds with
/// probability `chance`; `None` when `chance` is 0 or below.
pub fn expected_kuva(chance: f64, rerolls: u32, is_locked: bool) -> Option<f64> {
    if chance <= 0.0 {
        return None;
    }
    let missed = 1.0 - chance;
    let mut reach = 1.0;
    let mut spent = 0.0;
    let mut done = rerolls;
    while (done as usize) < REROLL_COSTS.len() - 1 {
        spent += reach * f64::from(reroll_cost(done, is_locked));
        reach *= missed;
        done += 1;
    }
    Some(spent + reach * f64::from(reroll_cost(done, is_locked)) / chance)
}

/// Chance that a reroll under one lock choice gives a pricier riven, and the expected kuva for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Odds {
    /// Tag of the stat locked through the reroll; `None` when nothing is locked.
    pub locked: Option<String>,
    pub chance: f64,
    /// Expected kuva spent until a reroll gives a pricier riven; `None` when no sample did.
    pub expected_kuva: Option<f64>,
    /// Mean price of the sampled rerolls that came out pricier; `None` when none did.
    pub better_mean: Option<f64>,
}

/// Chance that a reroll of `riven` gives a riven that `price` values above `standing`, for no
/// lock and for each of its stats locked (curses included), with the mean price of the pricier
/// results. `samples` rerolls are drawn per lock choice, from `seed`.
pub fn odds(
    riven: &Riven,
    pool: &[PoolStat],
    samples: u32,
    seed: u64,
    standing: f64,
    price: impl Fn(&Riven) -> Option<f64>,
) -> Vec<Odds> {
    let locks = std::iter::once(None)
        .chain(riven.buffs.iter().map(|stat| Some((stat, false))))
        .chain(riven.curses.iter().map(|stat| Some((stat, true))));

    let mut random = SplitMix(seed);
    locks
        .map(|lock| {
            let mut better = 0u32;
            let mut prices = Vec::new();
            for _ in 0..samples {
                let rolled = reroll(riven, pool, lock, &mut random);
                if let Some(fetched) = price(&rolled).filter(|fetched| *fetched > standing) {
                    better += 1;
                    prices.push(fetched);
                }
            }
            let chance = f64::from(better) / f64::from(samples.max(1));
            Odds {
                locked: lock.map(|(stat, _)| stat.tag.clone()),
                chance,
                expected_kuva: expected_kuva(chance, riven.rerolls, lock.is_some()),
                better_mean: (!prices.is_empty())
                    .then(|| prices.iter().sum::<f64>() / prices.len() as f64),
            }
        })
        .collect()
}

/// One simulated reroll of `riven`; `lock`, when set, is the stat kept and whether it is a curse.
fn reroll(
    riven: &Riven,
    pool: &[PoolStat],
    lock: Option<(&Stat, bool)>,
    random: &mut SplitMix,
) -> Riven {
    let (buffs, curses) = match lock {
        Some(_) => (riven.buffs.len(), riven.curses.len()),
        None => SPLITS[random.below(SPLITS.len())],
    };
    let mut taken: Vec<Stat> = Vec::new();
    let mut rolled_buffs = Vec::new();
    let mut rolled_curses = Vec::new();
    match lock {
        Some((stat, false)) => rolled_buffs.push(stat.clone()),
        Some((stat, true)) => rolled_curses.push(stat.clone()),
        None => {}
    }
    taken.extend(rolled_buffs.iter().chain(&rolled_curses).cloned());

    while rolled_buffs.len() < buffs {
        let Some(stat) = draw(pool, &taken, |pooled| pooled.can_buff, random) else {
            break;
        };
        taken.push(stat.clone());
        rolled_buffs.push(stat);
    }
    while rolled_curses.len() < curses {
        let Some(stat) = draw(pool, &taken, |pooled| pooled.can_curse, random) else {
            break;
        };
        taken.push(stat.clone());
        rolled_curses.push(stat);
    }

    Riven {
        buffs: rolled_buffs,
        curses: rolled_curses,
        ..riven.clone()
    }
}

/// Random stat from `pool` that `fits` and is not in `taken`, with a random roll; `None` when no
/// such stat is left.
fn draw(
    pool: &[PoolStat],
    taken: &[Stat],
    fits: impl Fn(&PoolStat) -> bool,
    random: &mut SplitMix,
) -> Option<Stat> {
    let open: Vec<&PoolStat> = pool
        .iter()
        .filter(|pooled| fits(pooled) && taken.iter().all(|stat| stat.tag != pooled.tag))
        .collect();
    if open.is_empty() {
        return None;
    }
    let pooled = open[random.below(open.len())];
    let roll = u32::try_from(random.next() % (ROLL_TOP + 1)).unwrap_or(u32::MAX);
    Some(Stat {
        tag: pooled.tag.to_owned(),
        roll,
    })
}

/// Seeded SplitMix64 generator, so the same riven always gets the same odds.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// Random number below `bound`.
    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riven::StatForm;

    const POOL: &[PoolStat] = &[
        PoolStat {
            tag: "A",
            base: 0.01,
            can_buff: true,
            can_curse: true,
            form: StatForm::Percent,
        },
        PoolStat {
            tag: "B",
            base: 0.01,
            can_buff: true,
            can_curse: true,
            form: StatForm::Percent,
        },
        PoolStat {
            tag: "C",
            base: 0.01,
            can_buff: true,
            can_curse: true,
            form: StatForm::Percent,
        },
        PoolStat {
            tag: "D",
            base: 0.01,
            can_buff: true,
            can_curse: false,
            form: StatForm::Percent,
        },
    ];

    fn stat(tag: &str) -> Stat {
        Stat {
            tag: tag.into(),
            roll: 0,
        }
    }

    fn riven(buffs: &[&str], curses: &[&str]) -> Riven {
        Riven {
            weapon: "/Gun".into(),
            mastery: 8,
            rerolls: 0,
            polarity: None,
            buffs: buffs.iter().map(|tag| stat(tag)).collect(),
            curses: curses.iter().map(|tag| stat(tag)).collect(),
        }
    }

    fn fetches_more_holding_a(riven: &Riven) -> Option<f64> {
        Some(if riven.buffs.iter().any(|stat| stat.tag == "A") {
            100.0
        } else {
            40.0
        })
    }

    #[test]
    fn a_free_reroll_and_a_locked_one_land_as_often_as_counted() {
        let found = odds(
            &riven(&["B", "C"], &[]),
            POOL,
            20_000,
            7,
            50.0,
            fetches_more_holding_a,
        );

        assert_eq!(found[0].locked, None);
        assert!(
            (found[0].chance - 0.625).abs() < 0.02,
            "{}",
            found[0].chance
        );
        assert_eq!(found[1].locked.as_deref(), Some("B"));
        assert!(
            (found[1].chance - 1.0 / 3.0).abs() < 0.02,
            "{}",
            found[1].chance
        );
        assert_eq!(found[0].better_mean, Some(100.0));
    }

    #[test]
    fn a_locked_curse_keeps_its_place_and_the_split() {
        let mut random = SplitMix(1);
        let current = riven(&["B", "C"], &["A"]);

        for _ in 0..200 {
            let rolled = reroll(
                &current,
                POOL,
                Some((&current.curses[0], true)),
                &mut random,
            );
            assert_eq!(rolled.buffs.len(), 2);
            assert_eq!(rolled.curses, vec![stat("A")]);
            assert!(rolled.buffs.iter().all(|stat| stat.tag != "A"));
        }
    }

    #[test]
    fn a_stat_that_never_hurts_is_never_a_curse() {
        let mut random = SplitMix(3);
        let current = riven(&["A", "B", "C"], &["D"]);

        for _ in 0..200 {
            let rolled = reroll(
                &current,
                POOL,
                Some((&current.buffs[0], false)),
                &mut random,
            );
            assert!(rolled.curses.iter().all(|stat| stat.tag != "D"));
        }
    }

    #[test]
    fn rerolls_climb_the_ladder_and_a_lock_doubles_up_to_its_cap() {
        assert_eq!(reroll_cost(0, false), 900);
        assert_eq!(reroll_cost(40, false), 3500);
        assert_eq!(reroll_cost(2, true), 2400);
        assert_eq!(reroll_cost(40, true), 7000);
    }

    #[test]
    fn expected_kuva_sums_the_ladder_until_a_hit() {
        assert_eq!(expected_kuva(1.0, 0, false), Some(900.0));
        assert_eq!(expected_kuva(0.5, 40, false), Some(7000.0));
        assert_eq!(expected_kuva(0.5, 40, true), Some(14_000.0));
        assert_eq!(expected_kuva(0.0, 0, false), None);
    }
}
