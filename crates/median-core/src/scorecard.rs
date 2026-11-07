use crate::earnings::{Deal, margins};
use crate::fills::Fills;
use crate::time::{DAY, Instant};

/// Results of the agent's trading.
#[derive(Debug, Clone, PartialEq)]
pub struct Scorecard {
    pub fills: Vec<Fills>,
    /// Sales margin in platinum per trade, purchases included in the trade count.
    pub per_exchange: Option<f64>,
    pub burned: Vec<Burned>,
}

/// Daily trades left unused at the end of a finished UTC day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Burned {
    pub day: Instant,
    pub exchanges: u32,
}

/// Sales margin in platinum of the deals from `from` on, divided by the number of those deals,
/// purchases included; `None` when there are none. Example: buy at 70 and sell at 100 gives 15.0.
/// Purchases before `from` still set the cost of later sales.
pub fn per_exchange(deals: &[Deal], from: Instant) -> Option<f64> {
    let counted: Vec<Option<i64>> = deals
        .iter()
        .zip(margins(deals))
        .filter(|(deal, _)| deal.at >= from)
        .map(|(_, margin)| margin)
        .collect();
    if counted.is_empty() {
        return None;
    }
    let made: i64 = counted.iter().flatten().sum();
    Some(made as f64 / counted.len() as f64)
}

/// Daily trades left unused at the end of each finished day: the trades left in the day's last
/// inventory dump, minus the trades made after that dump. Days without a dump are omitted.
pub fn burned(readings: &[(Instant, u32)], swaps: &[Instant], now: Instant) -> Vec<Burned> {
    // The game's day turns over at midnight UTC, so a day is a whole number of DAYs from the epoch.
    let today = now.div_euclid(DAY);
    let mut last: Vec<(Instant, u32)> = Vec::new();
    let mut sorted = readings.to_vec();
    sorted.sort_by_key(|(at, _)| *at);
    for (at, left) in sorted {
        match last.last_mut() {
            Some(kept) if kept.0.div_euclid(DAY) == at.div_euclid(DAY) => *kept = (at, left),
            _ => last.push((at, left)),
        }
    }
    last.into_iter()
        .filter(|(at, _)| at.div_euclid(DAY) < today)
        .map(|(at, left)| {
            let day = at.div_euclid(DAY) * DAY;
            let after = swaps
                .iter()
                .filter(|swap| **swap > at && **swap < day + DAY)
                .count();
            Burned {
                day,
                exchanges: left.saturating_sub(u32::try_from(after).unwrap_or(u32::MAX)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::HOUR;

    fn deal(at: Instant, selling: bool, platinum: u32) -> Deal {
        Deal {
            at,
            pair: "archon_flow:".into(),
            selling,
            platinum,
            quantity: 1,
        }
    }

    #[test]
    fn a_flip_is_worth_half_its_margin_per_exchange() {
        let deals = [deal(0, false, 70), deal(HOUR, true, 100)];

        assert_eq!(per_exchange(&deals, 0), Some(15.0));
        assert_eq!(per_exchange(&[], 0), None);
    }

    #[test]
    fn a_sale_in_the_window_is_costed_by_a_buy_before_it() {
        let deals = [deal(0, false, 70), deal(HOUR, true, 100)];

        assert_eq!(per_exchange(&deals, HOUR), Some(30.0));
    }

    #[test]
    fn a_day_burns_what_its_last_reading_left_less_the_swaps_after_it() {
        let readings = [(2 * HOUR, 20), (20 * HOUR, 9), (DAY + HOUR, 22)];
        let swaps = [21 * HOUR, 22 * HOUR, DAY + 2 * HOUR];

        assert_eq!(
            burned(&readings, &swaps, DAY + 5 * HOUR),
            [Burned {
                day: 0,
                exchanges: 7
            }]
        );
    }
}
