use std::sync::Arc;

use tokio::sync::Mutex;

use crate::agent::gear::Gear;
use crate::market::{Answers, Freshener, Heat};

/// Passes every fetched book to the agent while it runs, and sends the window how many watched
/// pairs have a fresh book whenever that count changes.
pub async fn tend(
    gear: Arc<Gear>,
    answers: Answers,
    freshener: Arc<Freshener>,
    running: Arc<Mutex<bool>>,
    heat: Heat,
) -> Result<(), String> {
    let mut told = None;
    loop {
        let (slug, looked) = answers.next().await;
        let warmth = freshener.warmth(&heat.items()).await;
        if told != Some(warmth) {
            told = Some(warmth);
            gear.warmed(warmth).await;
        }
        if *running.lock().await {
            gear.answered_with(&slug, &looked, &heat).await;
        }
    }
}
