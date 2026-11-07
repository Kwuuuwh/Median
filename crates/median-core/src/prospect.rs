use crate::market::{FAR, NEAR, THIN, Traded, realized};
use crate::time::{DAY, Instant};

/// Sale outlook for one copy of a variant: sold now to the highest bid, or at the ask after a wait.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Prospect {
    /// Highest bid from a player who is in game: the price of an immediate sale.
    pub bid: Option<u32>,
    /// Expected sale price: the closed-trade price, or the lowest in-game ask when that is lower.
    pub ask: Option<u32>,
    /// Copies traded per day.
    pub per_day: f64,
    /// In-game sellers a new listing queues behind; `None` when the order book was not read.
    pub queue: Option<u32>,
}

impl Prospect {
    /// Expected days until a copy listed at the ask sells, `(queue + 1) / per_day`; `None` when
    /// nothing trades.
    pub fn days(&self) -> Option<f64> {
        (self.per_day > 0.0).then(|| f64::from(self.queue.unwrap_or(0) + 1) / self.per_day)
    }

    /// Probability that a copy listed at the ask sells within `days`, assuming an exponentially
    /// distributed wait; 0 when nothing trades.
    pub fn chance(&self, days: f64) -> f64 {
        match self.days() {
            Some(expected) => 1.0 - (-days / expected).exp(),
            None => 0.0,
        }
    }

    /// Expected platinum per copy within `days`: the ask weighted by the chance to sell in time,
    /// plus the highest bid for the remaining chance. Without an ask, the bid alone.
    pub fn expected(&self, days: f64) -> Option<f64> {
        let bid = self.bid.map(f64::from);
        let Some(ask) = self.ask.map(f64::from) else {
            return bid;
        };
        let sold = self.chance(days);
        Some(ask * sold + bid.unwrap_or(0.0) * (1.0 - sold))
    }

    /// `expected` rounded to whole platinum.
    pub fn worth(&self, days: u32) -> Option<u32> {
        self.expected(f64::from(days))
            .map(|platinum| platinum.round().clamp(0.0, f64::from(u32::MAX)) as u32)
    }
}

/// Sale outlook of one variant from its hourly and daily closed trades and the latest read of its
/// order book.
pub fn prospect(
    hours: &[Traded],
    days: &[Traded],
    now: Instant,
    bid: Option<u32>,
    lowest_ingame: Option<u32>,
    sellers_ingame: Option<u32>,
) -> Prospect {
    let ask = realized(hours, days, now).map(|trades| {
        let traded = trades.platinum.round().clamp(0.0, f64::from(u32::MAX)) as u32;
        lowest_ingame.map_or(traded, |ingame| ingame.min(traded))
    });
    Prospect {
        bid,
        ask,
        per_day: per_day(hours, days, now),
        queue: sellers_ingame,
    }
}

/// Copies traded per day over the `NEAR` window, or over the `FAR` window when the near volume is
/// below `THIN`. The near window uses hourly buckets when there are any, daily buckets otherwise.
fn per_day(hours: &[Traded], days: &[Traded], now: Instant) -> f64 {
    let within = |buckets: &[Traded], window: Instant| -> u64 {
        buckets
            .iter()
            .filter(|bucket| bucket.starts >= now - window)
            .map(|bucket| bucket.volume)
            .sum()
    };
    let near = match hours.is_empty() {
        true => within(days, NEAR),
        false => within(hours, NEAR),
    };
    match near >= THIN {
        true => near as f64 / (NEAR as f64 / DAY as f64),
        false => within(days, FAR) as f64 / (FAR as f64 / DAY as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::HOUR;

    const NOW: Instant = 1_000 * DAY;

    fn bucket(ago: Instant, volume: u64, median: f64) -> Traded {
        Traded {
            starts: NOW - ago,
            volume,
            median,
            low: median,
        }
    }

    #[test]
    fn a_busy_pair_counts_its_copies_over_two_days() {
        let hours = [bucket(HOUR, 30, 40.0), bucket(30 * HOUR, 30, 41.0)];
        let days = [bucket(HOUR, 60, 40.0)];

        assert_eq!(per_day(&hours, &days, NOW), 30.0);
    }

    #[test]
    fn a_thin_pair_counts_its_copies_over_ninety_days() {
        let hours = [bucket(HOUR, 2, 40.0)];
        let days = [bucket(HOUR, 2, 40.0), bucket(20 * DAY, 7, 38.0)];

        assert_eq!(per_day(&hours, &days, NOW), 0.1);
    }

    #[test]
    fn a_listing_waits_behind_the_sellers_in_game() {
        let busy = Prospect {
            per_day: 10.0,
            queue: Some(9),
            ..Prospect::default()
        };

        assert_eq!(busy.days(), Some(1.0));
    }

    #[test]
    fn nothing_trading_means_nothing_sells() {
        let dead = Prospect {
            bid: Some(5),
            ask: Some(40),
            ..Prospect::default()
        };

        assert_eq!(dead.days(), None);
        assert_eq!(dead.chance(7.0), 0.0);
        assert_eq!(
            dead.expected(7.0),
            Some(5.0),
            "the dearest bid is all a copy fetches"
        );
    }

    #[test]
    fn a_quick_seller_is_worth_close_to_its_ask() {
        let quick = Prospect {
            bid: Some(30),
            ask: Some(40),
            per_day: 50.0,
            queue: Some(4),
        };

        let expected = quick.expected(3.0).expect("priced");
        assert!(expected > 39.9 && expected <= 40.0, "{expected}");
    }

    #[test]
    fn a_slow_seller_is_worth_close_to_its_bid() {
        let slow = Prospect {
            bid: Some(10),
            ask: Some(40),
            per_day: 0.05,
            queue: Some(20),
        };

        let expected = slow.expected(3.0).expect("priced");
        assert!(expected < 10.5, "{expected}");
    }

    #[test]
    fn without_anyone_in_game_the_closed_trades_set_the_ask() {
        let traded = [bucket(HOUR, 12, 52.0)];

        let seen = prospect(&traded, &[], NOW, None, None, None);

        assert_eq!(seen.ask, Some(52));
        assert_eq!(seen.bid, None);
    }

    #[test]
    fn a_seller_in_game_under_the_trades_sets_the_ask() {
        let traded = [bucket(HOUR, 12, 52.0)];

        assert_eq!(
            prospect(&traded, &[], NOW, Some(40), Some(48), Some(3)).ask,
            Some(48)
        );
    }

    #[test]
    fn an_ask_in_game_over_the_trades_is_not_what_a_copy_fetches() {
        let traded = [bucket(HOUR, 12, 15.0)];

        assert_eq!(
            prospect(&traded, &[], NOW, Some(11), Some(40), Some(8)).ask,
            Some(15)
        );
    }

    #[test]
    fn nothing_traded_is_no_ask_whatever_is_listed() {
        assert_eq!(
            prospect(&[], &[], NOW, Some(11), Some(40), Some(8)).ask,
            None
        );
    }
}
