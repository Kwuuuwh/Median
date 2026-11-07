use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use median_market::{Client, Doorman, MarketError, Pass};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, oneshot};
use tracing::warn;

use super::spacing::{AUCTION_GAP, Spacing};

/// Most market requests in flight at once.
const SLOTS: usize = 4;

/// Most of `SLOTS` the scheduler's routine work may hold.
const BACKGROUND: usize = 3;

/// Pause before checking for work again when there was none.
const IDLE: Duration = Duration::from_millis(250);

/// Pause when the market refuses all requests without a retry time.
const HELD: Duration = Duration::from_secs(1);

/// Source of a call; calls are served in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// The user waits for the response.
    Person,
    /// Writes or reads of the account's own orders.
    Commit,
    /// Routine work nobody waits on.
    Background,
}

struct Turn {
    pass: Pass,
    _slot: OwnedSemaphorePermit,
}

/// Calls waiting for a request pass, by priority and arrival.
#[derive(Default)]
struct Queue {
    calls: BTreeMap<(Priority, u64), oneshot::Sender<Result<Turn, MarketError>>>,
    next: u64,
}

struct Shared {
    client: Arc<Client>,
    queue: Mutex<Queue>,
    /// End time of the current refusal of all requests, and its error.
    held: Mutex<Option<(std::time::Instant, MarketError)>>,
    woken: Notify,
    slots: Arc<Semaphore>,
    background: Arc<Semaphore>,
    /// Spacing of auction calls, which the market rate-limits more strictly than the rest.
    auctions: Spacing,
}

/// Single entry point to the market: every request waits here for its turn.
#[derive(Clone)]
pub struct Gateway(Arc<Shared>);

/// Refresh work the gateway runs when no call waits.
#[async_trait]
pub trait Filler: Send + Sync + 'static {
    type Work: Send + 'static;

    /// Overdue work that the user is waiting on.
    async fn pressing(&self) -> Option<Self::Work>;

    /// The most overdue refresh.
    async fn pick(&self) -> Option<Self::Work>;

    /// Makes the one market request for this work.
    async fn run(&self, work: Self::Work, pass: Pass);
}

impl Gateway {
    pub fn new(client: Arc<Client>) -> Self {
        Self(Arc::new(Shared {
            client,
            queue: Mutex::new(Queue::default()),
            held: Mutex::new(None),
            woken: Notify::new(),
            slots: Arc::new(Semaphore::new(SLOTS)),
            background: Arc::new(Semaphore::new(BACKGROUND)),
            auctions: Spacing::new(AUCTION_GAP),
        }))
    }

    pub fn client(&self) -> &Arc<Client> {
        &self.0.client
    }

    /// Waits for a request pass at `priority` and makes one request with it; fails at once
    /// while the market refuses all requests.
    pub async fn call<T, F, Fut>(&self, priority: Priority, call: F) -> Result<T, MarketError>
    where
        F: FnOnce(Pass) -> Fut,
        Fut: Future<Output = Result<T, MarketError>>,
    {
        if let Some(refusal) = self.refusal() {
            return Err(refusal);
        }
        let (tx, rx) = oneshot::channel();
        {
            let mut queue = self.queue();
            let seq = queue.next;
            queue.next += 1;
            queue.calls.insert((priority, seq), tx);
        }
        self.0.woken.notify_one();

        let turn = rx
            .await
            .map_err(|_| MarketError::Unreachable("the gateway has stopped".into()))??;
        let Turn { pass, _slot } = turn;
        call(pass).await
    }

    /// Makes one auction call at least `AUCTION_GAP` after the previous one.
    pub async fn call_auction<T, F, Fut>(
        &self,
        priority: Priority,
        call: F,
    ) -> Result<T, MarketError>
    where
        F: FnOnce(Pass) -> Fut,
        Fut: Future<Output = Result<T, MarketError>>,
    {
        self.0.auctions.wait().await;
        self.call(priority, call).await
    }

    fn queue(&self) -> std::sync::MutexGuard<'_, Queue> {
        self.0.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Removes and returns the next queued call if its priority is `least` or higher.
    fn first(&self, least: Priority) -> Option<oneshot::Sender<Result<Turn, MarketError>>> {
        let mut queue = self.queue();
        let (&(priority, _), _) = queue.calls.first_key_value()?;
        if priority > least {
            return None;
        }
        queue.calls.pop_first().map(|(_, tx)| tx)
    }

    fn refusal(&self) -> Option<MarketError> {
        let held = self.0.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.as_ref()
            .filter(|(until, _)| std::time::Instant::now() < *until)
            .map(|(_, why)| why.clone())
    }

    /// Refuses every call for `pause`, failing the queued calls too.
    fn hold(&self, pause: Duration, why: &MarketError) {
        *self.0.held.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((std::time::Instant::now() + pause, why.clone()));
        let calls = std::mem::take(&mut self.queue().calls);
        for (_, tx) in calls {
            let _ = tx.send(Err(why.clone()));
        }
    }
}

/// Hands out request passes in this order: queued `Person` calls, pressing filler work, other
/// queued calls, then routine filler work, which holds at most `BACKGROUND` slots.
pub async fn tend<F: Filler>(
    gateway: Gateway,
    doorman: Arc<Doorman>,
    filler: Arc<F>,
) -> Result<(), String> {
    loop {
        let slot = Arc::clone(&gateway.0.slots)
            .acquire_owned()
            .await
            .map_err(|e| e.to_string())?;

        let pass = match doorman.admit().await {
            Ok(pass) => pass,
            Err(refusal) => {
                let pause = match refusal {
                    MarketError::RateLimited { after } => after,
                    _ => HELD,
                };
                gateway.hold(pause, &refusal);
                warn!(pause_s = pause.as_secs(), error = %refusal, "market requests paused");
                tokio::time::sleep(pause).await;
                continue;
            }
        };

        if let Some(waiting) = gateway.first(Priority::Person) {
            let _ = waiting.send(Ok(Turn { pass, _slot: slot }));
            continue;
        }
        if let Some(work) = filler.pressing().await {
            let filler = Arc::clone(&filler);
            tokio::spawn(async move {
                filler.run(work, pass).await;
                drop(slot);
            });
            continue;
        }
        if let Some(waiting) = gateway.first(Priority::Background) {
            let _ = waiting.send(Ok(Turn { pass, _slot: slot }));
            continue;
        }

        let work = match Arc::clone(&gateway.0.background).try_acquire_owned() {
            Ok(held) => filler.pick().await.map(|work| (work, held)),
            Err(_) => None,
        };
        match work {
            Some((work, held)) => {
                let filler = Arc::clone(&filler);
                tokio::spawn(async move {
                    filler.run(work, pass).await;
                    drop(held);
                    drop(slot);
                });
            }
            None => {
                drop(pass);
                drop(slot);
                tokio::select! {
                    () = gateway.0.woken.notified() => {}
                    () = tokio::time::sleep(IDLE) => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Instant;

    use median_market::{Budget, Gate, Host};

    use super::*;

    fn opened(per_second: f64) -> (Gateway, Doorman) {
        let (gate, doorman) = Gate::open(
            Budget {
                per_second,
                burst: 1.0,
                failures_before_open: 3,
                cooldown: Duration::from_secs(60),
            },
            Instant::now(),
        );
        let client = Client::new(gate, Host::default(), "median-test/0.1").expect("client");
        (Gateway::new(Arc::new(client)), doorman)
    }

    /// Filler that always has work and counts its runs.
    struct Endless(AtomicU32);

    #[async_trait]
    impl Filler for Endless {
        type Work = ();

        async fn pressing(&self) -> Option<()> {
            None
        }

        async fn pick(&self) -> Option<()> {
            Some(())
        }

        async fn run(&self, (): (), pass: Pass) {
            self.0.fetch_add(1, Ordering::Relaxed);
            drop(pass);
        }
    }

    /// Filler with no work.
    struct Nothing;

    #[async_trait]
    impl Filler for Nothing {
        type Work = ();

        async fn pressing(&self) -> Option<()> {
            None
        }

        async fn pick(&self) -> Option<()> {
            None
        }

        async fn run(&self, (): (), _pass: Pass) {}
    }

    #[tokio::test]
    async fn a_call_gets_its_turn_and_its_answer() {
        let (gateway, doorman) = opened(50.0);
        tokio::spawn(tend(gateway.clone(), Arc::new(doorman), Arc::new(Nothing)));

        let answer = gateway
            .call(Priority::Person, |pass| async move {
                drop(pass);
                Ok::<u32, MarketError>(7)
            })
            .await;

        assert_eq!(answer, Ok(7));
    }

    #[tokio::test]
    async fn calls_are_served_by_whose_they_are_before_the_order_they_came_in() {
        let (gateway, doorman) = opened(50.0);
        let served: Arc<Mutex<Vec<Priority>>> = Arc::default();

        let mut calls = Vec::new();
        for priority in [Priority::Background, Priority::Commit, Priority::Person] {
            let gateway = gateway.clone();
            let served = Arc::clone(&served);
            calls.push(tokio::spawn(async move {
                gateway
                    .call(priority, |pass| async move {
                        served
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .push(priority);
                        drop(pass);
                        Ok::<(), MarketError>(())
                    })
                    .await
            }));
        }
        while gateway.queue().calls.len() < 3 {
            tokio::task::yield_now().await;
        }
        tokio::spawn(tend(gateway.clone(), Arc::new(doorman), Arc::new(Nothing)));
        for call in calls {
            call.await.expect("joined").expect("served");
        }

        assert_eq!(
            *served.lock().unwrap_or_else(PoisonError::into_inner),
            vec![Priority::Person, Priority::Commit, Priority::Background]
        );
    }

    #[tokio::test]
    async fn the_budget_goes_to_the_background_when_nobody_calls() {
        let (gateway, doorman) = opened(200.0);
        let filler = Arc::new(Endless(AtomicU32::new(0)));
        tokio::spawn(tend(gateway, Arc::new(doorman), Arc::clone(&filler)));

        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(filler.0.load(Ordering::Relaxed) > 5);
    }

    #[tokio::test]
    async fn a_call_is_not_starved_by_a_background_that_never_runs_out() {
        let (gateway, doorman) = opened(20.0);
        tokio::spawn(tend(
            gateway.clone(),
            Arc::new(doorman),
            Arc::new(Endless(AtomicU32::new(0))),
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;

        let started = Instant::now();
        gateway
            .call(Priority::Person, |pass| async move {
                drop(pass);
                Ok::<(), MarketError>(())
            })
            .await
            .expect("served");

        assert!(started.elapsed() < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn a_refusal_reaches_every_waiting_call() {
        let (gateway, doorman) = opened(50.0);
        let pass = doorman.admit().await.expect("a token");
        let _ = gateway
            .client()
            .gate()
            .spend(pass, || async {
                Err::<(), MarketError>(MarketError::RateLimited {
                    after: Duration::from_secs(30),
                })
            })
            .await;
        tokio::spawn(tend(gateway.clone(), Arc::new(doorman), Arc::new(Nothing)));

        let answer = gateway
            .call(Priority::Person, |pass| async move {
                drop(pass);
                Ok::<(), MarketError>(())
            })
            .await;

        assert!(matches!(answer, Err(MarketError::RateLimited { .. })));
    }
}
