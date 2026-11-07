use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use median_core::CircuitBreaker;
use tracing::warn;

use crate::bucket::Bucket;
use crate::error::MarketError;

/// Rate and failure limits for all requests to warframe.market.
#[derive(Debug, Clone)]
pub struct Budget {
    /// Sustained requests per second. The documented public limit is 3.
    pub per_second: f64,
    /// Most requests that may be sent back to back.
    pub burst: f64,
    /// Consecutive market-wide failures before the circuit breaker opens.
    pub failures_before_open: u32,
    /// How long the circuit breaker stays open.
    pub cooldown: Duration,
}

/// Rate limiter and circuit breaker that every market request passes through.
pub struct Gate {
    inner: Mutex<Inner>,
}

struct Inner {
    bucket: Bucket,
    breaker: CircuitBreaker,
    /// Set when the market rate-limited the client; no request is sent before this time.
    pause_until: Option<Instant>,
    /// When the market started failing calls market-wide; cleared by the next successful call.
    failing_since: Option<Instant>,
}

/// Issues a gate's passes; the only way to obtain a `Pass`.
pub struct Doorman(Arc<Gate>);

impl Doorman {
    /// Waits for a token and returns a pass for it; `Err(RateLimited)` during a rate-limit pause,
    /// `Err(Unreachable)` while the circuit breaker is open.
    pub async fn admit(&self) -> Result<Pass, MarketError> {
        loop {
            let step = self.0.lock().step(Instant::now());
            match step {
                Step::Go => {
                    return Ok(Pass {
                        gate: Arc::clone(&self.0),
                        spent: false,
                    });
                }
                Step::Wait(delay) => tokio::time::sleep(delay).await,
                Step::Refuse(reason) => return Err(reason),
            }
        }
    }
}

/// A token taken for one request; returned to the bucket when dropped without being spent.
pub struct Pass {
    gate: Arc<Gate>,
    spent: bool,
}

impl Drop for Pass {
    fn drop(&mut self) {
        if !self.spent {
            self.gate.lock().bucket.give_back();
        }
    }
}

/// What a caller waiting for a pass should do next.
#[derive(Debug, PartialEq)]
enum Step {
    /// A token was taken; the caller gets the pass.
    Go,
    /// No token yet; retry after this delay.
    Wait(Duration),
    /// Do not call; the error gives the reason.
    Refuse(MarketError),
}

impl Gate {
    /// Creates a gate with a full bucket, and the doorman that issues its passes.
    pub fn open(budget: Budget, now: Instant) -> (Arc<Self>, Doorman) {
        let gate = Arc::new(Self {
            inner: Mutex::new(Inner {
                bucket: Bucket::new(budget.per_second, budget.burst, now),
                breaker: CircuitBreaker::new(budget.failures_before_open, budget.cooldown),
                pause_until: None,
                failing_since: None,
            }),
        });
        (Arc::clone(&gate), Doorman(gate))
    }

    /// Makes one call with a pass and records the outcome for the rate-limit pause and the
    /// circuit breaker.
    pub async fn spend<T, F, Fut>(&self, mut pass: Pass, call: F) -> Result<T, MarketError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, MarketError>>,
    {
        pass.spent = true;
        let outcome = call().await;
        self.lock().note(outcome.as_ref().err(), Instant::now());
        outcome
    }

    /// Whether the gate refuses every call right now: the circuit breaker is open or a
    /// rate-limit pause is active.
    pub fn shut(&self) -> bool {
        let inner = self.lock();
        inner.breaker.is_open()
            || inner
                .pause_until
                .is_some_and(|until| Instant::now() < until)
    }

    /// When market-wide failures started; `None` since the last successful call.
    pub fn failing_since(&self) -> Option<Instant> {
        self.lock().failing_since
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poisoned| {
            warn!("market rate gate lock poisoned, state kept");
            poisoned.into_inner()
        })
    }
}

impl Inner {
    fn step(&mut self, now: Instant) -> Step {
        match self.pause_until {
            Some(until) if now < until => {
                return Step::Refuse(MarketError::RateLimited { after: until - now });
            }
            Some(_) => self.pause_until = None,
            None => {}
        }

        if !self.breaker.allow(now) {
            return Step::Refuse(MarketError::Unreachable(
                "the market kept failing, so calls are held".into(),
            ));
        }

        let wait = self.bucket.wait(now);
        if !wait.is_zero() {
            return Step::Wait(wait);
        }
        self.bucket.take(now);
        Step::Go
    }

    fn note(&mut self, failure: Option<&MarketError>, now: Instant) {
        match failure {
            None => {
                self.breaker.on_success();
                self.failing_since = None;
            }
            Some(error) => {
                if let MarketError::RateLimited { after } = error {
                    self.pause_until = Some(now + *after);
                }
                if error.counts_against_the_source() {
                    self.breaker.on_failure(now);
                    self.failing_since.get_or_insert(now);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(now: Instant) -> (Arc<Gate>, Doorman) {
        Gate::open(
            Budget {
                per_second: 3.0,
                burst: 3.0,
                failures_before_open: 3,
                cooldown: Duration::from_secs(60),
            },
            now,
        )
    }

    fn gate(now: Instant) -> Arc<Gate> {
        opened(now).0
    }

    #[test]
    fn spends_the_burst_then_asks_the_caller_to_wait() {
        let t = Instant::now();
        let g = gate(t);
        for _ in 0..3 {
            assert_eq!(g.lock().step(t), Step::Go);
        }
        match g.lock().step(t) {
            Step::Wait(delay) => assert!(delay > Duration::ZERO),
            other => panic!("expected a wait, got {other:?}"),
        }
    }

    #[test]
    fn a_rate_limit_refuses_everything_until_it_passes() {
        let t = Instant::now();
        let g = gate(t);
        g.lock().note(
            Some(&MarketError::RateLimited {
                after: Duration::from_secs(30),
            }),
            t,
        );

        match g.lock().step(t + Duration::from_secs(1)) {
            Step::Refuse(MarketError::RateLimited { after }) => {
                assert!(after <= Duration::from_secs(29));
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert!(g.shut());
        // After the rate-limit pause the gate allows calls again.
        assert_eq!(g.lock().step(t + Duration::from_secs(31)), Step::Go);
    }

    #[test]
    fn repeated_source_failures_shut_the_gate() {
        let t = Instant::now();
        let g = gate(t);
        for _ in 0..3 {
            g.lock()
                .note(Some(&MarketError::Unreachable("timeout".into())), t);
        }
        assert!(matches!(g.lock().step(t), Step::Refuse(_)));
        assert!(g.shut());
    }

    #[test]
    fn the_gate_remembers_since_when_the_source_fails_until_it_answers() {
        let t = Instant::now();
        let g = gate(t);
        g.lock()
            .note(Some(&MarketError::Forbidden("items".into())), t);
        g.lock().note(
            Some(&MarketError::Forbidden("my orders".into())),
            t + Duration::from_secs(5),
        );
        assert_eq!(g.failing_since(), Some(t));
        g.lock().note(None, t + Duration::from_secs(9));
        assert_eq!(g.failing_since(), None);
    }

    #[test]
    fn a_missing_item_is_not_an_outage() {
        let t = Instant::now();
        let g = gate(t);
        for _ in 0..10 {
            g.lock()
                .note(Some(&MarketError::NotListed("excalibur_prime".into())), t);
        }
        assert!(!g.shut());
    }

    #[tokio::test]
    async fn spend_reports_the_outcome_and_returns_it() {
        let (g, doorman) = opened(Instant::now());
        let pass = doorman.admit().await.expect("a token");
        let answer = g.spend(pass, || async { Ok::<u32, MarketError>(7) }).await;
        assert_eq!(answer, Ok(7));

        let pass = doorman.admit().await.expect("a token");
        let refused = g
            .spend(pass, || async {
                Err::<u32, MarketError>(MarketError::RateLimited {
                    after: Duration::from_secs(30),
                })
            })
            .await;
        assert!(refused.is_err());
        // The rate limit is kept: no pass is issued until the pause ends.
        assert!(g.shut());
        assert!(matches!(
            doorman.admit().await,
            Err(MarketError::RateLimited { .. })
        ));
    }

    #[tokio::test]
    async fn a_pass_dropped_unspent_gives_its_token_back() {
        let (g, doorman) = opened(Instant::now());
        let passes: Vec<Pass> = vec![
            doorman.admit().await.expect("a token"),
            doorman.admit().await.expect("a token"),
            doorman.admit().await.expect("a token"),
        ];
        assert!(matches!(g.lock().step(Instant::now()), Step::Wait(_)));

        drop(passes);
        assert_eq!(g.lock().step(Instant::now()), Step::Go);
    }
}
