use std::time::{Duration, Instant};

/// Token bucket for the request budget: tokens refill at a steady rate up to the burst size.
#[derive(Debug)]
pub struct Bucket {
    per_second: f64,
    capacity: f64,
    tokens: f64,
    last: Instant,
}

impl Bucket {
    /// Creates a full bucket; `per_second` is the sustained rate, `burst` the most requests
    /// that may be sent back to back.
    pub fn new(per_second: f64, burst: f64, now: Instant) -> Self {
        Self {
            per_second,
            capacity: burst,
            tokens: burst,
            last: now,
        }
    }

    /// How long until a token is free; zero when one is free right now.
    pub fn wait(&mut self, now: Instant) -> Duration {
        self.refill(now);
        if self.tokens >= 1.0 {
            return Duration::ZERO;
        }
        let missing = 1.0 - self.tokens;
        Duration::from_secs_f64(missing / self.per_second)
    }

    /// Spends a token; `false` when none is available.
    pub fn take(&mut self, now: Instant) -> bool {
        self.refill(now);
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }

    /// Returns a token that was taken but not used, up to the burst size.
    pub fn give_back(&mut self) {
        self.tokens = (self.tokens + 1.0).min(self.capacity);
    }

    fn refill(&mut self, now: Instant) {
        if now <= self.last {
            return;
        }
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.per_second).min(self.capacity);
        self.last = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bucket(now: Instant) -> Bucket {
        Bucket::new(3.0, 3.0, now)
    }

    #[test]
    fn spends_the_burst_then_makes_the_caller_wait() {
        let t = Instant::now();
        let mut b = bucket(t);
        for _ in 0..3 {
            assert!(b.take(t));
        }
        assert!(!b.take(t));
        assert!(b.wait(t) > Duration::ZERO);
    }

    #[test]
    fn refills_at_the_stated_rate() {
        let t = Instant::now();
        let mut b = bucket(t);
        for _ in 0..3 {
            assert!(b.take(t));
        }
        // A third of a second is one token at three per second.
        let later = t + Duration::from_millis(334);
        assert!(b.take(later));
        assert!(!b.take(later));
    }

    #[test]
    fn never_holds_more_than_the_burst() {
        let t = Instant::now();
        let mut b = bucket(t);
        let much_later = t + Duration::from_secs(60);
        for _ in 0..3 {
            assert!(b.take(much_later));
        }
        assert!(!b.take(much_later));
    }

    #[test]
    fn a_token_given_back_can_be_taken_again_but_never_past_the_burst() {
        let t = Instant::now();
        let mut b = bucket(t);
        assert!(b.take(t));
        b.give_back();
        b.give_back();
        for _ in 0..3 {
            assert!(b.take(t));
        }
        assert!(!b.take(t));
    }

    #[test]
    fn wait_shrinks_as_time_passes() {
        let t = Instant::now();
        let mut b = bucket(t);
        for _ in 0..3 {
            assert!(b.take(t));
        }
        let first = b.wait(t);
        let second = b.wait(t + Duration::from_millis(100));
        assert!(
            second < first,
            "{second:?} should be shorter than {first:?}"
        );
    }
}
