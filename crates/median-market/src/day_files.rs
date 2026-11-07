use std::collections::BTreeMap;
use std::time::Duration;

use serde::Deserialize;

use crate::error::MarketError;
use crate::model::{ClosedBucket, Side};

const HOST: &str = "https://relics.run/history";

/// A day file downloads in about a second. The host sometimes stalls mid-body, and waiting on a
/// stall delays prices in the market cache.
const TIMEOUT: Duration = Duration::from_secs(30);

/// One row of a day file: a statistics bucket, its item, and its order type.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Daily {
    pub item_id: String,
    /// `closed` for closed trades, `buy` and `sell` for orders open that day.
    pub order_type: String,
    #[serde(flatten)]
    pub bucket: ClosedBucket,
}

impl Daily {
    pub fn is_closed(&self) -> bool {
        self.order_type == "closed"
    }

    /// Side of the open orders the row summarises; `None` for a closed-trade row.
    pub fn side(&self) -> Option<Side> {
        Side::from_name(&self.order_type)
    }
}

/// Client for the day files relics.run publishes: one file of market history per day.
pub struct DayFiles {
    http: reqwest::Client,
    host: String,
}

impl DayFiles {
    pub fn new(agent: &str) -> Result<Self, MarketError> {
        Self::at(agent, HOST.to_owned())
    }

    pub fn at(agent: &str, host: String) -> Result<Self, MarketError> {
        let http = reqwest::Client::builder()
            .user_agent(agent.to_owned())
            .timeout(TIMEOUT)
            .build()
            .map_err(|e| MarketError::Unreachable(e.to_string()))?;
        Ok(Self { http, host })
    }

    /// Every row of the day file for `date` (`YYYY-MM-DD`); `None` when the host has no file
    /// for that day (HTTP 404).
    pub async fn day(&self, date: &str) -> Result<Option<Vec<Daily>>, MarketError> {
        let url = format!("{}/price_history_{date}.json", self.host);
        let answer = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| MarketError::Unreachable(e.to_string()))?;

        if answer.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !answer.status().is_success() {
            return Err(MarketError::Unreachable(format!(
                "{} answered {}",
                url,
                answer.status()
            )));
        }

        let bytes = answer
            .bytes()
            .await
            .map_err(|e| MarketError::Unreachable(e.to_string()))?;
        let byname: BTreeMap<String, Vec<Daily>> = serde_json::from_slice(&bytes)
            .map_err(|e| MarketError::Malformed(format!("{url}: {e}")))?;

        Ok(Some(byname.into_values().flatten().collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows in the day file format, from 2026-08-23.
    const DAY: &str = r#"{
      "Secura Dual Cestra":[
        {"datetime":"2026-08-23T00:00:00.000+00:00","volume":32,"min_price":20.0,
         "max_price":40.0,"open_price":35.0,"closed_price":40.0,"avg_price":30.0,
         "wa_price":29.656,"median":30.0,"moving_avg":29.5,"donch_top":40.0,"donch_bot":18.0,
         "id":"6a8b8980659ec80011f80519","item_id":"54aae292e7798909064f1575",
         "order_type":"closed"},
        {"datetime":"2026-08-23T00:00:00.000+00:00","volume":245,"min_price":10,
         "max_price":25,"avg_price":17.5,"wa_price":18.11,"median":16.0,"order_type":"buy",
         "moving_avg":26.5,"id":"6a8b8980659ec80011f8051d",
         "item_id":"54aae292e7798909064f1575"}],
      "Neo D5 Relic":[
        {"datetime":"2026-08-23T00:00:00.000+00:00","volume":300,"min_price":6,"max_price":6,
         "avg_price":6.0,"wa_price":6.0,"median":6,"order_type":"buy","subtype":"intact",
         "moving_avg":10.2,"id":"6a8b8d51659ec80011f85abb",
         "item_id":"62d3494075156700ce450df8"}]
    }"#;

    fn rows() -> Vec<Daily> {
        let byname: BTreeMap<String, Vec<Daily>> =
            serde_json::from_str(DAY).expect("a day file parses");
        byname.into_values().flatten().collect()
    }

    #[test]
    fn a_day_carries_every_item_under_its_display_name() {
        let rows = rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().filter(|row| row.is_closed()).count(), 1);
    }

    #[test]
    fn the_item_is_known_by_its_market_id_not_by_the_key() {
        let closed = rows()
            .into_iter()
            .find(Daily::is_closed)
            .expect("one row is closed");

        assert_eq!(closed.item_id, "54aae292e7798909064f1575");
        assert_eq!(closed.bucket.volume, 32);
        assert_eq!(closed.bucket.median, 30.0);
        assert_eq!(closed.bucket.wa_price, Some(29.656));
    }

    #[test]
    fn what_this_file_adds_is_not_taken_for_an_unknown_field() {
        for row in rows() {
            assert!(
                row.bucket.extra.is_empty(),
                "row carried unexpected fields: {:?}",
                row.bucket.extra
            );
        }
    }

    #[test]
    fn a_variant_the_row_names_survives_the_parse() {
        let relic = rows()
            .into_iter()
            .find(|row| row.item_id == "62d3494075156700ce450df8")
            .expect("the relic row is there");

        assert_eq!(relic.bucket.subtype.as_deref(), Some("intact"));
        assert_eq!(relic.order_type, "buy");
    }
}
