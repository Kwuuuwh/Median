use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

/// Least gap between auction calls; the market refuses calls that come closer.
pub const AUCTION_GAP: Duration = Duration::from_millis(3500);

/// Keeps calls of one kind at least `gap` apart.
pub struct Spacing {
    gap: Duration,
    /// Earliest time the next call of this kind may be sent.
    next: Mutex<Instant>,
}

impl Spacing {
    pub fn new(gap: Duration) -> Self {
        Self {
            gap,
            next: Mutex::new(Instant::now()),
        }
    }

    /// Waits until a call of this kind may be sent, and sets the next allowed time `gap` later.
    pub async fn wait(&self) {
        let mut next = self.next.lock().await;
        tokio::time::sleep_until(*next).await;
        *next = Instant::now() + self.gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn calls_go_out_a_gap_apart() {
        let spacing = Spacing::new(Duration::from_millis(40));
        let started = Instant::now();

        spacing.wait().await;
        spacing.wait().await;
        spacing.wait().await;

        assert!(started.elapsed() >= Duration::from_millis(80));
    }

    #[tokio::test]
    async fn the_first_call_does_not_wait() {
        let spacing = Spacing::new(Duration::from_secs(60));
        let started = Instant::now();

        spacing.wait().await;

        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
