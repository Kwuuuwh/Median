use std::sync::Arc;

use median_core::{Changed, Event, State, update};
use tokio::sync::mpsc::Receiver;

use crate::IngestError;
use crate::ingest::ingest;
use crate::input::{Input, Raw};
use crate::ports::Ports;
use crate::quarantine::{QuarantineRecord, describe};

/// The runtime loop: input -> event | quarantine -> update -> effects -> publish.
pub struct Engine {
    state: State,
    ports: Ports,
}

impl Engine {
    /// Creates an engine with default state and the given ports.
    pub fn new(ports: Ports) -> Self {
        Self::with_state(State::default(), ports)
    }

    /// Creates an engine seeded with `state` and the given ports.
    pub fn with_state(state: State, ports: Ports) -> Self {
        Self { state, ports }
    }

    /// The current state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Applies one input and publishes the state when any domain changed.
    pub async fn handle(&mut self, input: Input) {
        let changed = self.apply(input).await;
        if !changed.is_empty() {
            self.ports.sink.publish(&self.state, changed);
        }
    }

    /// Drains inputs from `rx`, applying each in arrival order and publishing
    /// once per burst.
    pub async fn run(mut self, mut rx: Receiver<Input>) {
        while let Some(first) = rx.recv().await {
            let mut changed = self.apply(first).await;
            while let Ok(next) = rx.try_recv() {
                changed |= self.apply(next).await;
            }
            if !changed.is_empty() {
                self.ports.sink.publish(&self.state, changed);
            }
        }
    }

    /// Applies one input, returning the domains it changed.
    async fn apply(&mut self, input: Input) -> Changed {
        let event = match input {
            Input::Command(event) => event,
            Input::Raw(raw) => match ingest(&raw) {
                Ok(event) => event,
                Err(error) => return self.quarantine(&raw, error),
            },
        };

        let (next, changed, effects) = update(&self.state, event);
        self.state = next;

        for effect in effects {
            let runner = Arc::clone(&self.ports.effects);
            tokio::spawn(async move { runner.run(effect).await });
        }

        changed
    }

    /// Records rejected data in quarantine and increments the quarantine counter in state.
    fn quarantine(&mut self, raw: &Raw, error: IngestError) -> Changed {
        self.ports.quarantine.record(QuarantineRecord {
            source: raw.source(),
            error,
            shape: describe(raw),
        });

        let (next, changed, _) = update(&self.state, Event::InputQuarantined);
        self.state = next;
        changed
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use median_core::{CatalogEvent, Changed, Domain, Effect, Event, Progress, State};
    use tokio::sync::Notify;
    use tokio::time::timeout;

    use super::Engine;
    use crate::error::IngestError;
    use crate::input::{Input, Raw};
    use crate::ports::{EffectRunner, Ports, QuarantineSink, StateSink};
    use crate::quarantine::QuarantineRecord;

    /// Test double recording everything the engine sends to its ports.
    #[derive(Default)]
    struct Spy {
        quarantined: Mutex<Vec<QuarantineRecord>>,
        published: Mutex<Vec<(State, Changed)>>,
        ran: Mutex<Vec<Effect>>,
        /// Notified after an effect is added to `ran`.
        finished: Notify,
    }

    /// Effect runner that holds the effect until the test releases it.
    #[derive(Default)]
    struct Blocking {
        release: Notify,
        finished: Notify,
        ran: Mutex<Vec<Effect>>,
    }

    impl QuarantineSink for Spy {
        fn record(&self, record: QuarantineRecord) {
            self.quarantined.lock().unwrap().push(record);
        }
    }

    impl StateSink for Spy {
        fn publish(&self, state: &State, changed: Changed) {
            self.published
                .lock()
                .unwrap()
                .push((state.clone(), changed));
        }
    }

    #[async_trait]
    impl EffectRunner for Spy {
        async fn run(&self, effect: Effect) {
            self.ran.lock().unwrap().push(effect);
            self.finished.notify_one();
        }
    }

    #[async_trait]
    impl EffectRunner for Blocking {
        async fn run(&self, effect: Effect) {
            self.release.notified().await;
            self.ran.lock().unwrap().push(effect);
            self.finished.notify_one();
        }
    }

    fn engine_with(spy: Arc<Spy>) -> Engine {
        Engine::new(Ports {
            quarantine: spy.clone(),
            effects: spy.clone(),
            sink: spy,
        })
    }

    /// A world-state document trimmed to the fields the parser insists on.
    const DOCUMENT: &str = r#"{
        "Time": 1785240441,
        "SyndicateMissions": [
          { "Tag": "CetusSyndicate", "Expiry": { "$date": { "$numberLong": "1785243478000" } } }
        ],
        "ActiveMissions": []
    }"#;

    fn worldstate_input(body: &str) -> Input {
        Input::Raw(Raw::Worldstate(body.into()))
    }

    #[tokio::test]
    async fn accepted_input_flows_through_the_backbone() {
        let spy = Arc::new(Spy::default());
        let mut engine = engine_with(spy.clone());

        engine.handle(worldstate_input(DOCUMENT)).await;

        assert!(engine.state().world.worldstate.is_some());
        assert_eq!(spy.published.lock().unwrap().len(), 1);
        assert!(spy.quarantined.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_publication_names_the_domains_that_moved() {
        let spy = Arc::new(Spy::default());
        let mut engine = engine_with(spy.clone());

        engine.handle(worldstate_input(DOCUMENT)).await;

        let published = spy.published.lock().unwrap();
        assert_eq!(published[0].1, Changed::of(Domain::World));
    }

    #[tokio::test]
    async fn a_command_needs_no_parsing() {
        let spy = Arc::new(Spy::default());
        let mut engine = engine_with(spy.clone());

        engine
            .handle(Input::Command(Event::Catalog(CatalogEvent::SyncRequested)))
            .await;

        let published = spy.published.lock().unwrap();
        assert_eq!(published[0].1, Changed::of(Domain::Catalog));
        assert!(spy.quarantined.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn malformed_input_is_quarantined_and_counted() {
        let spy = Arc::new(Spy::default());
        let mut engine = engine_with(spy.clone());

        engine
            .handle(Input::Raw(Raw::CatalogVersion("ten".into())))
            .await;

        assert!(engine.state().catalog.installed.is_none()); // domain untouched
        assert_eq!(engine.state().quarantined, 1); // health counter advanced
        assert!(spy.ran.lock().unwrap().is_empty());

        let published = spy.published.lock().unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].1, Changed::of(Domain::Quarantine));

        let records = spy.quarantined.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, "catalog");
        assert_eq!(
            records[0].error,
            IngestError::MalformedCatalogVersion("ten".into())
        );
    }

    #[tokio::test]
    async fn a_noop_input_is_not_published() {
        let spy = Arc::new(Spy::default());
        let mut engine = engine_with(spy.clone());

        engine
            .handle(Input::Command(Event::Progressed(Progress {
                domain: Domain::World,
                done: 1,
                total: 2,
            })))
            .await; // no domain carries progress but the catalog

        assert!(spy.ran.lock().unwrap().is_empty());
        assert!(spy.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn valid_worldstate_reaches_state() {
        let spy = Arc::new(Spy::default());
        let mut engine = engine_with(spy.clone());

        engine.handle(worldstate_input(DOCUMENT)).await;

        assert_eq!(
            engine.state().world.worldstate.as_ref().map(|w| w.at),
            Some(1_785_240_441)
        );
        assert_eq!(spy.published.lock().unwrap().len(), 1);
        assert!(spy.quarantined.lock().unwrap().is_empty());
        assert!(spy.ran.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn broken_worldstate_is_quarantined_and_counted() {
        let spy = Arc::new(Spy::default());
        let mut engine = engine_with(spy.clone());

        engine.handle(worldstate_input("not json")).await;

        assert_eq!(engine.state().world.worldstate, None);
        assert_eq!(engine.state().quarantined, 1);

        let records = spy.quarantined.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, "worldstate");
        assert_eq!(records[0].shape, "world-state document, 8 bytes");
        assert!(matches!(
            records[0].error,
            IngestError::MalformedWorldstate(_)
        ));
    }

    #[tokio::test]
    async fn a_burst_is_published_once() {
        let spy = Arc::new(Spy::default());
        let engine = engine_with(spy.clone());

        let (tx, rx) = tokio::sync::mpsc::channel(16);
        tx.send(Input::Command(Event::Catalog(CatalogEvent::SyncRequested)))
            .await
            .unwrap();
        tx.send(worldstate_input(DOCUMENT)).await.unwrap();
        drop(tx); // close so run() returns after draining

        engine.run(rx).await;
        spy.finished.notified().await;

        let published = spy.published.lock().unwrap();
        assert_eq!(published.len(), 1); // two changes, one publication
        assert!(published[0].0.world.worldstate.is_some());
        assert!(published[0].1.has(Domain::Catalog));
        assert!(published[0].1.has(Domain::World));
        assert_eq!(spy.ran.lock().unwrap().len(), 1); // effects stay per-event
    }

    #[tokio::test]
    async fn a_blocked_effect_does_not_hold_up_the_next_input() {
        let spy = Arc::new(Spy::default());
        let effects = Arc::new(Blocking::default());
        let mut engine = Engine::new(Ports {
            quarantine: spy.clone(),
            effects: effects.clone(),
            sink: spy.clone(),
        });

        timeout(Duration::from_secs(5), async {
            engine
                .handle(Input::Command(Event::Catalog(CatalogEvent::SyncRequested)))
                .await;
            engine.handle(worldstate_input(DOCUMENT)).await;

            assert!(engine.state().world.worldstate.is_some());
            assert!(effects.ran.lock().unwrap().is_empty());

            effects.release.notify_one();
            effects.finished.notified().await;
        })
        .await
        .expect("the loop waited for the effect");

        assert_eq!(
            effects.ran.lock().unwrap().as_slice(),
            &[Effect::CheckCatalog]
        );
    }
}
