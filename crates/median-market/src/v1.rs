use serde::Deserialize;

use crate::error::MarketError;
use crate::gate::Pass;
use crate::http::Client;
use crate::model::{ClosedBucket, LiveBucket, Statistics};

/// v1 wraps every response in `{ payload }`.
#[derive(Debug, Deserialize)]
struct Envelope<T> {
    payload: T,
}

#[derive(Debug, Deserialize)]
struct StatisticsPayload {
    statistics_closed: Series<ClosedBucket>,
    statistics_live: Series<LiveBucket>,
}

/// Each series has two resolutions: hourly buckets over 48 hours, daily buckets over 90 days.
#[derive(Debug, Deserialize)]
struct Series<T> {
    #[serde(rename = "48hours", default = "Vec::new")]
    hourly: Vec<T>,
    #[serde(rename = "90days", default = "Vec::new")]
    daily: Vec<T>,
}

impl Client {
    /// Every statistics bucket warframe.market has for one item, 100–200 KB per response. The
    /// market may redirect the slug, so the response can describe a different slug.
    pub async fn statistics(&self, pass: Pass, slug: &str) -> Result<Statistics, MarketError> {
        let url = self.v1(&format!("/items/{slug}/statistics"));
        let answer: Envelope<StatisticsPayload> = self.get(pass, url, slug).await?;
        Ok(Statistics {
            closed_hourly: answer.payload.statistics_closed.hourly,
            closed_daily: answer.payload.statistics_closed.daily,
            live_hourly: answer.payload.statistics_live.hourly,
            live_daily: answer.payload.statistics_live.daily,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Side;

    /// A trimmed response in the real format.
    const ANSWER: &str = r#"{"payload":{
      "statistics_closed":{
        "48hours":[{"datetime":"2026-08-16T17:00:00.000+00:00","volume":4,"min_price":18.0,
          "max_price":20.0,"open_price":20.0,"closed_price":18.0,"avg_price":19.0,
          "wa_price":18.5,"median":18.0,"moving_avg":18.9,"donch_top":20.0,"donch_bot":18.0,
          "id":"6a81fbc4659ec80011d3e6c3"}],
        "90days":[{"datetime":"2026-08-15T00:00:00.000+00:00","volume":71,"min_price":17,
          "max_price":20,"open_price":20,"closed_price":20,"avg_price":18.5,"wa_price":19.085,
          "median":19.0,"moving_avg":18.4,"donch_top":20,"donch_bot":17,
          "id":"6a80ff4c659ec80011d03971","subtype":"intact"}]},
      "statistics_live":{
        "48hours":[{"datetime":"2026-08-16T18:00:00.000+00:00","volume":65,"min_price":15,
          "max_price":27,"avg_price":21.0,"wa_price":20.846,"median":20.0,"order_type":"sell",
          "moving_avg":16.9,"id":"6a8209d4659ec80011d41e80","mod_rank":5}],
        "90days":[{"datetime":"2026-08-15T00:00:00.000+00:00","volume":346,"min_price":13,
          "max_price":29,"avg_price":21.0,"wa_price":21.477,"median":20,"order_type":"buy",
          "id":"6a80ff4c659ec80011d03976"}]}}}"#;

    fn statistics() -> Statistics {
        let parsed: Envelope<StatisticsPayload> =
            serde_json::from_str(ANSWER).expect("statistics parse");
        Statistics {
            closed_hourly: parsed.payload.statistics_closed.hourly,
            closed_daily: parsed.payload.statistics_closed.daily,
            live_hourly: parsed.payload.statistics_live.hourly,
            live_daily: parsed.payload.statistics_live.daily,
        }
    }

    #[test]
    fn all_four_series_are_kept_apart() {
        let stats = statistics();
        assert_eq!(stats.closed_hourly.len(), 1);
        assert_eq!(stats.closed_daily.len(), 1);
        assert_eq!(stats.live_hourly.len(), 1);
        assert_eq!(stats.live_daily.len(), 1);
    }

    #[test]
    fn prices_arrive_as_whole_numbers_and_as_fractions() {
        let stats = statistics();
        // The hourly bucket has 18.0, the daily one 17; both parse as prices.
        assert_eq!(stats.closed_hourly[0].min_price, 18.0);
        assert_eq!(stats.closed_daily[0].min_price, 17.0);
    }

    #[test]
    fn variant_fields_survive_the_parse() {
        let stats = statistics();
        assert_eq!(stats.closed_daily[0].subtype.as_deref(), Some("intact"));
        assert_eq!(stats.live_hourly[0].mod_rank, Some(5));
        assert_eq!(stats.closed_hourly[0].subtype, None);
    }

    #[test]
    fn the_two_sides_of_the_live_series_are_told_apart() {
        let stats = statistics();
        assert_eq!(stats.live_hourly[0].order_type, Side::Sell);
        assert_eq!(stats.live_daily[0].order_type, Side::Buy);
    }

    #[test]
    fn the_market_id_is_carried_but_is_not_a_key() {
        let stats = statistics();
        assert_eq!(
            stats.closed_daily[0].id.as_deref(),
            Some("6a80ff4c659ec80011d03971")
        );
    }

    #[test]
    fn a_missing_series_reads_as_empty_rather_than_failing() {
        let parsed: Envelope<StatisticsPayload> = serde_json::from_str(
            r#"{"payload":{"statistics_closed":{"90days":[]},"statistics_live":{}}}"#,
        )
        .expect("a thin answer still parses");
        assert!(parsed.payload.statistics_closed.hourly.is_empty());
        assert!(parsed.payload.statistics_live.daily.is_empty());
    }
}
