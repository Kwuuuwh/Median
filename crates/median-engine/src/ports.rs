use std::sync::Arc;

use async_trait::async_trait;
use median_core::{Changed, Effect, State};

use crate::quarantine::QuarantineRecord;

/// Sink for inputs that could not become events.
pub trait QuarantineSink: Send + Sync {
    /// Stores one quarantine record.
    fn record(&self, record: QuarantineRecord);
}

/// Sink that publishes the latest state to the UI.
pub trait StateSink: Send + Sync {
    /// Publishes one state snapshot, naming the domains that changed.
    fn publish(&self, state: &State, changed: Changed);
}

/// Executor of effects.
#[async_trait]
pub trait EffectRunner: Send + Sync {
    /// Executes one effect.
    async fn run(&self, effect: Effect);
}

/// The engine's outbound ports.
pub struct Ports {
    pub quarantine: Arc<dyn QuarantineSink>,
    pub effects: Arc<dyn EffectRunner>,
    pub sink: Arc<dyn StateSink>,
}
