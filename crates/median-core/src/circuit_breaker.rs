use std::time::{Duration, Instant};

/// Rejects calls while a source keeps failing, so it is not retried on every call.
pub struct CircuitBreaker {
    failure_threshold: u32,
    cooldown: Duration,
    state: BreakerState,
}

enum BreakerState {
    Closed { failures: u32 },
    Open { until: Instant },
    HalfOpen,
}

impl CircuitBreaker {
    /// Opens after `failure_threshold` failures, staying open for `cooldown`.
    pub fn new(failure_threshold: u32, cooldown: Duration) -> Self {
        Self {
            failure_threshold,
            cooldown,
            state: BreakerState::Closed { failures: 0 },
        }
    }

    /// Whether a call may proceed at `now`; an open breaker whose cooldown has passed turns
    /// half-open and allows calls again.
    pub fn allow(&mut self, now: Instant) -> bool {
        match self.state {
            BreakerState::Closed { .. } | BreakerState::HalfOpen => true,
            BreakerState::Open { until } => {
                if now >= until {
                    self.state = BreakerState::HalfOpen;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Records a successful call, closing the breaker.
    pub fn on_success(&mut self) {
        self.state = BreakerState::Closed { failures: 0 };
    }

    /// Records a failed call at `now`; opens the breaker for `cooldown` when the failure count
    /// reaches the threshold, or on any failure while half-open.
    pub fn on_failure(&mut self, now: Instant) {
        self.state = match self.state {
            BreakerState::Closed { failures } => {
                let failures = failures + 1;
                if failures >= self.failure_threshold {
                    BreakerState::Open {
                        until: now + self.cooldown,
                    }
                } else {
                    BreakerState::Closed { failures }
                }
            }
            BreakerState::HalfOpen | BreakerState::Open { .. } => BreakerState::Open {
                until: now + self.cooldown,
            },
        };
    }

    /// Whether the breaker is currently open.
    pub fn is_open(&self) -> bool {
        matches!(self.state, BreakerState::Open { .. })
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::CircuitBreaker;

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new(3, Duration::from_secs(60))
    }

    #[test]
    fn stays_closed_below_threshold() {
        let mut b = breaker();
        let t = Instant::now();
        b.on_failure(t);
        b.on_failure(t);
        assert!(b.allow(t));
        assert!(!b.is_open());
    }

    #[test]
    fn opens_after_threshold() {
        let mut b = breaker();
        let t = Instant::now();
        for _ in 0..3 {
            b.on_failure(t);
        }
        assert!(b.is_open());
        assert!(!b.allow(t));
    }

    #[test]
    fn half_opens_after_cooldown() {
        let mut b = breaker();
        let t = Instant::now();
        for _ in 0..3 {
            b.on_failure(t);
        }
        assert!(b.allow(t + Duration::from_secs(61)));
        assert!(!b.is_open());
    }

    #[test]
    fn success_closes_the_breaker() {
        let mut b = breaker();
        let t = Instant::now();
        for _ in 0..3 {
            b.on_failure(t);
        }
        b.allow(t + Duration::from_secs(61)); // -> half-open trial
        b.on_success();
        assert!(b.allow(t + Duration::from_secs(61)));
        assert!(!b.is_open());
    }

    #[test]
    fn failure_while_half_open_reopens() {
        let mut b = breaker();
        let t = Instant::now();
        for _ in 0..3 {
            b.on_failure(t);
        }
        let later = t + Duration::from_secs(61);
        assert!(b.allow(later)); // half-open trial
        b.on_failure(later);
        assert!(b.is_open());
        assert!(!b.allow(later));
    }
}
