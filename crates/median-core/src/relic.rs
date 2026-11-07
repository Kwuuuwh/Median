/// One row of a relic's reward table, with the value of its item.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Prize {
    /// Chance of this reward, in tenths of a percent.
    pub chance: u32,
    /// Copies the row rewards; more than one for Forma.
    pub count: u32,
    /// Market price of one copy, in platinum; `None` when the market cache has no price.
    pub platinum: Option<f64>,
    /// Ducat value of one copy; `None` when the item cannot be traded in for ducats.
    pub ducats: Option<u32>,
}

impl Prize {
    /// Platinum value of the whole row; an unpriced row counts as 0.
    fn plat(&self) -> f64 {
        self.platinum.unwrap_or(0.0) * f64::from(self.count)
    }

    /// Ducat value of the whole row.
    fn duc(&self) -> f64 {
        f64::from(self.ducats.unwrap_or(0)) * f64::from(self.count)
    }
}

/// How often a row of the reward table is the reward the squad takes.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Taken {
    /// Share of runs in which this row is the highest-platinum reward drawn.
    pub platinum: f64,
    /// Share of runs in which this row is the highest-ducat reward drawn.
    pub ducats: f64,
}

/// Expected value of opening a relic, and which rows it comes from.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Payback {
    pub platinum: f64,
    pub ducats: f64,
    /// One entry per row, in the order the rows were given.
    pub taken: Vec<Taken>,
    /// Rows without a price in the market cache.
    pub unpriced: u32,
}

/// Expected platinum and ducats per member of a squad of `squad` players who all open this relic:
/// each member draws once and every member takes the best draw. Platinum and ducats rank the
/// rows differently, so each is computed separately.
pub fn payback(prizes: &[Prize], squad: u32) -> Payback {
    let total: f64 = prizes.iter().map(|prize| f64::from(prize.chance)).sum();
    if total <= 0.0 {
        return Payback {
            taken: vec![Taken::default(); prizes.len()],
            ..Payback::default()
        };
    }

    let draws = i32::try_from(squad.max(1)).unwrap_or(i32::MAX);
    let (platinum, in_platinum) = best(prizes, total, draws, Prize::plat);
    let (ducats, in_ducats) = best(prizes, total, draws, Prize::duc);

    Payback {
        platinum,
        ducats,
        taken: in_platinum
            .into_iter()
            .zip(in_ducats)
            .map(|(platinum, ducats)| Taken { platinum, ducats })
            .collect(),
        unpriced: prizes
            .iter()
            .filter(|prize| prize.platinum.is_none())
            .count() as u32,
    }
}

/// Probability that a reward appears at least once over all `(chance, draws)` pairs; each pair is
/// one relic's chance of the reward and the number of draws from it (a squad draws once per
/// member).
pub fn at_least_once(draws: &[(f64, u32)]) -> f64 {
    let missed: f64 = draws
        .iter()
        .map(|&(chance, times)| {
            (1.0 - chance.clamp(0.0, 1.0)).powi(times.min(i32::MAX as u32) as i32)
        })
        .product();
    1.0 - missed
}

/// Expected `worth` of the best of `draws` draws, and the share of runs in which each row is that
/// best draw.
fn best(
    prizes: &[Prize],
    total: f64,
    draws: i32,
    worth: impl Fn(&Prize) -> f64,
) -> (f64, Vec<f64>) {
    let mut order: Vec<usize> = (0..prizes.len()).collect();
    order.sort_by(|&one, &other| worth(&prizes[one]).total_cmp(&worth(&prizes[other])));

    let mut shares = vec![0.0; prizes.len()];
    let mut expected = 0.0;
    let mut below = 0.0;
    let mut at = 0;

    while at < order.len() {
        let value = worth(&prizes[order[at]]);
        // Rows of equal worth form one tier: which of them is taken does not change the value,
        // so the tier's weight is split by each row's chance.
        let end = order[at..]
            .iter()
            .position(|&index| worth(&prizes[index]) != value)
            .map_or(order.len(), |offset| at + offset);

        let rung: f64 = order[at..end]
            .iter()
            .map(|&index| f64::from(prizes[index].chance) / total)
            .sum();
        let above = below + rung;
        let weight = above.powi(draws) - below.powi(draws);
        expected += weight * value;

        if rung > 0.0 {
            for &index in &order[at..end] {
                shares[index] = weight * (f64::from(prizes[index].chance) / total) / rung;
            }
        }

        below = above;
        at = end;
    }

    (expected, shares)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lith S1 as in the catalog, valued in ducats: three commons at 15, a Forma blueprint
    /// rewarded as 2 copies and worth 0, an uncommon part at 45 and a rare at 100.
    fn lith_s1(common: u32, uncommon: u32, rare: u32) -> Vec<Prize> {
        let mut table = vec![
            Prize {
                chance: uncommon,
                count: 2,
                platinum: None,
                ducats: None,
            },
            Prize {
                chance: uncommon,
                count: 1,
                platinum: Some(12.0),
                ducats: Some(45),
            },
            Prize {
                chance: rare,
                count: 1,
                platinum: Some(30.0),
                ducats: Some(100),
            },
        ];
        table.extend([0; 3].iter().map(|_| Prize {
            chance: common,
            count: 1,
            platinum: Some(4.0),
            ducats: Some(15),
        }));
        table
    }

    fn intact() -> Vec<Prize> {
        lith_s1(253, 110, 20)
    }

    #[test]
    fn a_squad_of_four_on_radiant_relics_finds_a_rare_part_one_time_in_three() {
        let squad = at_least_once(&[(0.10, 4)]);

        assert!((squad - 0.3439).abs() < 1e-9);
    }

    #[test]
    fn relics_of_different_kinds_add_their_chances_up() {
        let owned = at_least_once(&[(0.02, 3), (0.11, 1)]);

        assert!((owned - (1.0 - 0.98_f64.powi(3) * 0.89)).abs() < 1e-9);
        assert_eq!(at_least_once(&[]), 0.0);
    }

    #[test]
    fn one_carrier_is_the_plain_sum_of_chance_times_worth() {
        let paid = payback(&intact(), 1);

        let plain: f64 = intact()
            .iter()
            .map(|prize| f64::from(prize.chance) / 999.0 * prize.duc())
            .sum();
        assert!((paid.ducats - plain).abs() < 1e-9);
    }

    #[test]
    fn lith_s1_pays_back_what_was_measured() {
        let measured = [
            (intact(), [18.3, 24.3, 28.5, 32.1]),
            (lith_s1(233, 130, 40), [20.4, 28.4, 34.2, 39.0]),
            (lith_s1(200, 170, 60), [22.7, 33.2, 40.5, 46.5]),
            (lith_s1(167, 200, 100), [26.5, 40.1, 49.5, 56.7]),
        ];

        for (table, expected) in measured {
            for (squad, want) in expected.iter().enumerate() {
                let got = payback(&table, squad as u32 + 1).ducats;
                assert!(
                    (got - want).abs() < 0.1,
                    "squad {}: got {got}, expected {want}",
                    squad + 1
                );
            }
        }
    }

    #[test]
    fn a_bigger_squad_never_pays_back_less() {
        let table = intact();
        let mut before = 0.0;

        for squad in 1..=8 {
            let paid = payback(&table, squad);
            assert!(paid.ducats >= before);
            before = paid.ducats;
        }
    }

    #[test]
    fn rows_worth_the_same_split_the_rung_by_how_often_they_come_up() {
        let paid = payback(&intact(), 4);

        let commons: Vec<f64> = paid.taken[3..].iter().map(|taken| taken.ducats).collect();
        assert!((commons[0] - commons[1]).abs() < 1e-12);
        assert!((commons[1] - commons[2]).abs() < 1e-12);
        assert!(commons[0] > 0.0);
    }

    #[test]
    fn every_run_carries_something_away() {
        let paid = payback(&intact(), 3);

        let shares: f64 = paid.taken.iter().map(|taken| taken.platinum).sum();
        assert!((shares - 1.0).abs() < 1e-9);
    }

    #[test]
    fn an_item_the_market_cache_cannot_price_is_counted_and_worth_nothing() {
        let paid = payback(&intact(), 4);

        assert_eq!(paid.unpriced, 1);
        assert!(paid.platinum > 0.0);
    }

    #[test]
    fn one_carrier_counts_every_priced_reward_in_the_payback() {
        let paid = payback(&intact(), 1);

        assert!(
            (paid.platinum - (3.0 * 253.0 * 4.0 + 110.0 * 12.0 + 20.0 * 30.0) / 999.0).abs() < 1e-9
        );
    }

    #[test]
    fn a_table_with_no_chances_pays_back_nothing() {
        let paid = payback(&[Prize::default(); 2], 4);

        assert_eq!(paid.platinum, 0.0);
        assert_eq!(paid.taken.len(), 2);
    }
}
