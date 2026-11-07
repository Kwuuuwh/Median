use std::time::{Duration, Instant};

use median_core::CircuitBreaker;
use median_engine::{Input, Raw};
use tokio::sync::mpsc::Sender;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{Instrument, debug, info_span, warn};

/// Where and how often to poll the world-state source.
#[derive(Debug, Clone)]
pub struct Source {
    /// Endpoint returning the raw world-state document.
    pub url: String,
    /// Delay between polls.
    pub interval: Duration,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            url: "https://api.warframe.com/cdn/worldState.php".into(),
            interval: Duration::from_secs(30),
        }
    }
}

/// Polls the source every `source.interval` and sends each response body to the engine as
/// `Raw::Worldstate`; returns `Ok` once the engine input is closed.
pub async fn poll(tx: Sender<Input>, source: Source) -> Result<(), String> {
    let client = crate::http::http_client()?;
    let mut breaker = CircuitBreaker::new(3, Duration::from_secs(60));
    let mut ticks = interval(source.interval);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticks.tick().await;

        if !breaker.allow(Instant::now()) {
            debug!("worldstate poll skipped, circuit open");
            continue;
        }

        let span = info_span!("worldstate.fetch");

        match fetch(&client, &source.url).instrument(span.clone()).await {
            Ok(body) => {
                breaker.on_success();
                if tx.send(Input::Raw(Raw::Worldstate(body))).await.is_err() {
                    return Ok(()); // engine input closed
                }
            }
            Err(reason) => {
                breaker.on_failure(Instant::now());
                warn!(parent: &span, circuit_open = breaker.is_open(), error = %reason, "worldstate fetch failed");
            }
        }
    }
}

/// GETs the raw document; `Err` holds the error text, including for a non-success status.
async fn fetch(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("source returned {status}"));
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    debug!(bytes = body.len(), "worldstate fetched");
    Ok(body)
}
