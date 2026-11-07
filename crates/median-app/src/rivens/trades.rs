use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use median_core::{RivenTrades, TradeBounds};
use median_engine::parse_riven_trades;
use tokio::time::sleep;
use tracing::{info, warn};

/// URL of the official weekly riven trade statistics for PC, published on warframe.com.
const WEEKLY: &str = "https://www.warframe.com/repos/weeklyRivensPC.json";

/// Refresh interval of the weekly file.
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// Delay before the next attempt after a failed download.
const RETRY: Duration = Duration::from_secs(60 * 60);

/// Latest official weekly riven trade statistics; empty until the first download.
#[derive(Default)]
pub struct Weekly(RwLock<Vec<RivenTrades>>);

impl Weekly {
    fn replace(&self, trades: Vec<RivenTrades>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = trades;
    }

    /// Median price of veiled rivens of `kind` in last week's trades.
    pub(super) fn veiled(&self, kind: &str) -> Option<f64> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|trades| trades.weapon.is_none() && !trades.rerolled && trades.kind == kind)
            .map(|trades| trades.median)
    }

    /// Median price in last week's trades of `family` rivens, rerolled or not.
    pub fn traded(&self, family: &str, rerolled: bool) -> Option<f64> {
        self.of_family(family, rerolled, |trades| trades.median)
    }

    /// Whether the weekly file has been downloaded and holds rows.
    pub(super) fn is_read(&self) -> bool {
        !self
            .0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    }

    /// Popularity of `family` rivens, rerolled or not, relative to the others in last week's
    /// trades.
    pub(super) fn popularity(&self, family: &str, rerolled: bool) -> Option<f64> {
        self.of_family(family, rerolled, |trades| trades.popularity)
    }

    /// Price bounds from last week's trades of `family` rivens, rerolled or not.
    pub(super) fn bounds(&self, family: &str, rerolled: bool) -> Option<TradeBounds> {
        self.of_family(family, rerolled, TradeBounds::of)
    }

    fn of_family<T>(
        &self,
        family: &str,
        rerolled: bool,
        read: impl Fn(&RivenTrades) -> T,
    ) -> Option<T> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|trades| {
                trades.rerolled == rerolled
                    && trades
                        .weapon
                        .as_deref()
                        .is_some_and(|weapon| weapon.eq_ignore_ascii_case(family))
            })
            .map(read)
    }
}

/// Downloads the weekly file now and every `EVERY`, after `RETRY` on failure.
pub async fn tend(weekly: Arc<Weekly>) -> Result<(), String> {
    let client = crate::http::http_client()?;
    loop {
        match fetch(&client).await {
            Ok(trades) => {
                info!(rows = trades.len(), "weekly riven trades read");
                weekly.replace(trades);
                sleep(EVERY).await;
            }
            Err(why) => {
                warn!(error = %why, "weekly riven trades read failed");
                sleep(RETRY).await;
            }
        }
    }
}

async fn fetch(client: &reqwest::Client) -> Result<Vec<RivenTrades>, String> {
    let response = client.get(WEEKLY).send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("source returned {status}"));
    }
    let text = response.text().await.map_err(|e| e.to_string())?;
    parse_riven_trades(&text)
}
