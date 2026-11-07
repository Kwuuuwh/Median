use crate::demand::Demand;
use crate::policy::Policy;
use crate::reroll::Odds;

/// Offers are rounded down to a multiple of this, in platinum.
const OFFER_STEP: f64 = 5.0;

/// Whether a lot's buyout is at or below last week's median trade price for its weapon; `false`
/// when there were no trades.
pub fn worth_a_look(buyout: f64, traded: Option<f64>) -> bool {
    traded.is_some_and(|middle| buyout <= middle)
}

/// Market data for one riven auction lot.
#[derive(Debug, Clone, PartialEq)]
pub struct LotView {
    pub buyout: f64,
    /// Median ask of similar rivens.
    pub fair: Option<f64>,
    pub rerolls: u32,
    /// Chance of a better reroll, one entry per lock choice.
    pub odds: Vec<Odds>,
    /// Demand for rivens of its weapon; `None` when it could not be determined.
    pub demand: Option<Demand>,
}

/// Why a lot was picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickReason {
    /// Buyout well below the asks of similar rivens.
    Cheap,
    /// Never rerolled, and the expected value of one reroll is well above its buyout.
    Unrolled,
}

/// Lot selected for purchase.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    pub reason: PickReason,
    pub buyout: f64,
    /// Expected resale price, in platinum.
    pub worth: f64,
    /// First price offered to the seller, in platinum.
    pub offer: f64,
}

/// Upper bound of the required margin, as a fraction of `worth`, whatever the weapon's demand.
const MOST_MARGIN: f64 = 0.9;

/// Selects the lot when its buyout is at least the policy's margin below its expected resale
/// price and the profit is at least the policy's minimum; `None` otherwise. `Dead` demand is never
/// picked; `Slow`, `Unclear` or unknown demand requires twice the margin, capped at `MOST_MARGIN`.
/// A rerolled lot can only be picked as `Cheap`.
pub fn pick(lot: &LotView, policy: &Policy) -> Option<Pick> {
    let asked = f64::from(policy.riven_margin.min(100)) / 100.0;
    let margin = match lot.demand {
        Some(Demand::Dead) => return None,
        Some(Demand::Lively) => asked,
        Some(Demand::Slow | Demand::Unclear) | None => (asked * 2.0).min(MOST_MARGIN),
    };
    let least_profit = f64::from(policy.riven_least_profit);
    let under =
        |worth: f64| lot.buyout <= worth * (1.0 - margin) && worth - lot.buyout >= least_profit;
    let picked = |reason, worth: f64| Pick {
        reason,
        buyout: lot.buyout,
        worth,
        offer: offer(lot.buyout, worth, margin),
    };
    if let Some(fair) = lot.fair.filter(|fair| under(*fair)) {
        return Some(picked(PickReason::Cheap, fair));
    }
    if lot.rerolls > 0 {
        return None;
    }
    rolled_worth(&lot.odds, lot.fair.unwrap_or(0.0))
        .filter(|worth| under(*worth))
        .map(|worth| picked(PickReason::Unrolled, worth))
}

/// Expected value of one reroll for the best lock choice: the chance of a better roll times its
/// mean ask, plus the remaining chance times `fair`. `None` when no choice has a better-roll ask.
fn rolled_worth(odds: &[Odds], fair: f64) -> Option<f64> {
    odds.iter()
        .filter_map(|odds| {
            let better = odds.better_mean?;
            Some(odds.chance * better + (1.0 - odds.chance) * fair)
        })
        .max_by(f64::total_cmp)
}

fn offer(buyout: f64, worth: f64, margin: f64) -> f64 {
    let doubled = worth * (1.0 - 2.0 * margin);
    ((doubled.min(buyout) / OFFER_STEP).floor() * OFFER_STEP).max(OFFER_STEP)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn odds(better: f64) -> Odds {
        Odds {
            locked: None,
            chance: 0.3,
            expected_kuva: Some(20_000.0),
            better_mean: Some(better),
        }
    }

    fn lot(buyout: f64, fair: Option<f64>, rerolls: u32, odds: Vec<Odds>) -> LotView {
        LotView {
            buyout,
            fair,
            rerolls,
            odds,
            demand: Some(Demand::Lively),
        }
    }

    #[test]
    fn a_lot_of_a_weapon_that_does_not_sell_is_left_however_cheap() {
        let dead = LotView {
            demand: Some(Demand::Dead),
            ..lot(100.0, Some(500.0), 4, Vec::new())
        };
        assert_eq!(pick(&dead, &Policy::default()), None);
    }

    #[test]
    fn a_lot_of_a_slow_weapon_is_asked_twice_the_margin() {
        let slow = |buyout| LotView {
            demand: Some(Demand::Slow),
            ..lot(buyout, Some(500.0), 4, Vec::new())
        };
        assert_eq!(pick(&slow(300.0), &Policy::default()), None);
        assert!(pick(&slow(240.0), &Policy::default()).is_some());
    }

    #[test]
    fn a_look_needs_a_buyout_at_most_last_weeks_middle() {
        assert!(worth_a_look(80.0, Some(100.0)));
        assert!(!worth_a_look(120.0, Some(100.0)));
        assert!(!worth_a_look(80.0, None));
    }

    #[test]
    fn a_lot_well_under_its_asks_is_picked_cheap_with_an_offer_under_the_buyout() {
        let picked = pick(&lot(300.0, Some(500.0), 4, Vec::new()), &Policy::default()).unwrap();

        assert_eq!(picked.reason, PickReason::Cheap);
        assert_eq!(picked.worth, 500.0);
        assert_eq!(picked.offer, 250.0);
    }

    #[test]
    fn a_lot_near_its_asks_is_left() {
        assert_eq!(
            pick(&lot(400.0, Some(500.0), 4, Vec::new()), &Policy::default()),
            None
        );
    }

    #[test]
    fn a_lot_far_under_its_asks_but_bringing_in_little_is_left() {
        assert_eq!(
            pick(&lot(10.0, Some(18.0), 4, Vec::new()), &Policy::default()),
            None
        );
    }

    #[test]
    fn an_unrolled_lot_is_picked_for_what_one_roll_is_worth_at_its_chance() {
        let rolled = vec![odds(700.0), odds(900.0)];
        let picked = pick(&lot(300.0, Some(350.0), 0, rolled), &Policy::default()).unwrap();

        assert_eq!(picked.reason, PickReason::Unrolled);
        assert!((picked.worth - (0.3 * 900.0 + 0.7 * 350.0)).abs() < 1e-9);
        assert_eq!(picked.offer, 255.0);
    }

    #[test]
    fn a_rolled_lot_is_not_picked_for_its_rolls() {
        let rolled = vec![odds(900.0)];
        assert_eq!(
            pick(&lot(500.0, Some(450.0), 3, rolled), &Policy::default()),
            None
        );
    }
}
