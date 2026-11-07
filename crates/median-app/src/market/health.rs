use std::sync::Arc;
use std::time::Duration;

use median_core::{Event, Instant, MarketEvent, seconds};
use median_engine::Input;
use median_market::Client;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::{info, warn};

/// Interval between checks of the market gate.
const EVERY: Duration = Duration::from_secs(5);

/// How long every call must fail before the market counts as not responding.
const GRACE: Duration = Duration::from_secs(20);

/// Sends the engine a market event once every call has failed for `GRACE`, and another once
/// the market responds again.
pub async fn tend(client: Arc<Client>, tx: Sender<Input>) -> Result<(), String> {
    let mut told = false;
    loop {
        let failing = client
            .gate()
            .failing_since()
            .filter(|since| since.elapsed() >= GRACE);
        let event = match (failing, told) {
            (Some(since), false) => {
                let gone = since.elapsed().as_secs();
                warn!(silent_s = gone, "market not responding");
                Some(MarketEvent::Failing {
                    since: seconds() - gone as Instant,
                })
            }
            (None, true) => {
                info!("market responding again");
                Some(MarketEvent::Answering)
            }
            _ => None,
        };
        if let Some(event) = event {
            told = !told;
            tx.send(Input::Command(Event::Market(event)))
                .await
                .map_err(|e| e.to_string())?;
        }
        sleep(EVERY).await;
    }
}
