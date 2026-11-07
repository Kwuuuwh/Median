use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use tracing::debug;

use crate::error::MarketError;
use crate::gate::Gate;
use crate::gate::Pass;

/// Timeout of one request; on expiry the market counts as unreachable.
const TIMEOUT: Duration = Duration::from_secs(20);

/// Back-off after HTTP 429 when the response has no `Retry-After` header.
const DEFAULT_BACKOFF: Duration = Duration::from_secs(30);

/// Base URLs of the warframe.market v1 and v2 APIs.
#[derive(Debug, Clone)]
pub struct Host {
    pub v1: String,
    pub v2: String,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            v1: "https://api.warframe.market/v1".into(),
            v2: "https://api.warframe.market/v2".into(),
        }
    }
}

/// HTTP client for warframe.market; every call passes through the gate. Calls that need the
/// account's token are made through `Session`.
pub struct Client {
    http: reqwest::Client,
    gate: Arc<Gate>,
    host: Host,
}

impl Client {
    /// Creates a client; `agent` is sent verbatim as the `User-Agent` header.
    pub fn new(gate: Arc<Gate>, host: Host, agent: &str) -> Result<Self, MarketError> {
        let http = reqwest::Client::builder()
            .user_agent(agent.to_owned())
            .timeout(TIMEOUT)
            .build()
            .map_err(|e| MarketError::Unreachable(e.to_string()))?;
        Ok(Self { http, gate, host })
    }

    /// The gate every call of this client passes through.
    pub fn gate(&self) -> &Arc<Gate> {
        &self.gate
    }

    pub(crate) fn v1(&self, path: &str) -> String {
        format!("{}{path}", self.host.v1)
    }

    pub(crate) fn v2(&self, path: &str) -> String {
        format!("{}{path}", self.host.v2)
    }

    /// Builds a request with the headers every call sends; `authorization` is sent verbatim.
    fn request(
        &self,
        method: reqwest::Method,
        url: &str,
        authorization: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let request = self
            .http
            .request(method, url)
            .header("accept", "application/json")
            .header("platform", "pc")
            // Cross-platform players appear in the order books the site shows a PC player;
            // without this header the market omits them.
            .header("crossplay", "true")
            .header("language", "en");
        match authorization {
            Some(value) => request.header(reqwest::header::AUTHORIZATION, value),
            None => request,
        }
    }

    /// GETs one JSON response through the gate; `subject` names the call in errors.
    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        pass: Pass,
        url: String,
        subject: &str,
    ) -> Result<T, MarketError> {
        self.json(pass, reqwest::Method::GET, url, None, None, subject)
            .await
    }

    /// Sends one call on `pass` and parses the JSON response; `body: None` sends no body.
    pub(crate) async fn json<T: DeserializeOwned>(
        &self,
        pass: Pass,
        method: reqwest::Method,
        url: String,
        authorization: Option<&str>,
        body: Option<serde_json::Value>,
        subject: &str,
    ) -> Result<T, MarketError> {
        let bytes = self
            .gate
            .spend(pass, || async {
                let mut request = self.request(method, &url, authorization);
                if let Some(body) = body {
                    request = request.json(&body);
                }
                let response = request
                    .send()
                    .await
                    .map_err(|e| MarketError::Unreachable(e.to_string()))?;
                let bytes = answer(response, subject).await?;
                debug!(bytes = bytes.len(), %url, "market response received");
                Ok(bytes)
            })
            .await?;

        serde_json::from_slice(&bytes)
            .map_err(|e| MarketError::Malformed(format!("{subject}: {e}")))
    }

    /// POSTs one call and returns the value of response header `name` with the body bytes;
    /// `Err(Malformed)` when the header is missing.
    pub(crate) async fn header(
        &self,
        pass: Pass,
        url: String,
        authorization: Option<&str>,
        body: serde_json::Value,
        name: &str,
        subject: &str,
    ) -> Result<(String, Vec<u8>), MarketError> {
        self.gate
            .spend(pass, || async {
                let response = self
                    .request(reqwest::Method::POST, &url, authorization)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| MarketError::Unreachable(e.to_string()))?;

                let found = response
                    .headers()
                    .get(name)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                let bytes = answer(response, subject).await?;

                match found {
                    Some(value) => Ok((value, bytes)),
                    None => Err(MarketError::Malformed(format!(
                        "{subject}: the answer carried no {name} header"
                    ))),
                }
            })
            .await
    }
}

/// Reads a response body; a non-success status becomes the matching `MarketError`.
async fn answer(response: reqwest::Response, subject: &str) -> Result<Vec<u8>, MarketError> {
    let status = response.status();
    let retry_after = retry_after(&response);
    let bytes = response
        .bytes()
        .await
        .map_err(|e| MarketError::Unreachable(e.to_string()))?;

    if !status.is_success() {
        return Err(refusal(status.as_u16(), &bytes, subject, retry_after));
    }
    Ok(bytes.to_vec())
}

/// Maps an HTTP error status, and a body that mentions `deprecated`, to a `MarketError`.
fn refusal(status: u16, body: &[u8], subject: &str, retry_after: Option<Duration>) -> MarketError {
    let says_deprecated = String::from_utf8_lossy(body)
        .to_lowercase()
        .contains("deprecated");
    match status {
        401 => MarketError::Unauthorized,
        403 if says_deprecated => MarketError::Retired(subject.to_owned()),
        403 if body.is_empty() => MarketError::Forbidden(subject.to_owned()),
        403 => MarketError::Forbidden(format!("{subject} ({})", said(body))),
        404 if says_deprecated => MarketError::Retired(subject.to_owned()),
        404 => MarketError::NotListed(subject.to_owned()),
        410 => MarketError::Retired(subject.to_owned()),
        429 => MarketError::RateLimited {
            after: retry_after.unwrap_or(DEFAULT_BACKOFF),
        },
        509 => MarketError::TooManyConnections,
        400 => MarketError::Refused(said(body)),
        500..=599 => MarketError::Unreachable(format!("market answered {status}")),
        _ => MarketError::Unreachable(format!("market answered {status}")),
    }
}

/// The `error` field of an error response, or else the raw body, cut to 200 characters.
fn said(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let said = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|answer| answer.get("error").cloned())
        .map_or_else(|| text.to_string(), |error| error.to_string());
    said.chars().take(200).collect()
}

/// The `Retry-After` header in seconds; `None` when it is absent or not a number.
fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_item_is_about_the_item_not_the_market() {
        let error = refusal(404, b"{}", "excalibur_prime", None);
        assert_eq!(error, MarketError::NotListed("excalibur_prime".into()));
        assert!(!error.counts_against_the_source());
    }

    #[test]
    fn a_deprecated_route_is_retired_not_forbidden() {
        let error = refusal(403, b"Deprecated", "v1 orders", None);
        assert_eq!(error, MarketError::Retired("v1 orders".into()));
    }

    #[test]
    fn an_expired_session_is_told_apart_from_a_refused_right() {
        assert_eq!(
            refusal(401, b"", "my orders", None),
            MarketError::Unauthorized
        );
        assert_eq!(
            refusal(403, b"", "my orders", None),
            MarketError::Forbidden("my orders".into())
        );
        assert_eq!(
            refusal(
                403,
                br#"{"error":"app.order.forbidden"}"#,
                "place order",
                None
            ),
            MarketError::Forbidden(r#"place order ("app.order.forbidden")"#.into())
        );
    }

    #[test]
    fn an_expired_session_does_not_count_against_the_market_but_a_refusal_does() {
        assert!(!refusal(401, b"", "my orders", None).counts_against_the_source());
        assert!(refusal(403, b"nope", "items", None).counts_against_the_source());
    }

    #[test]
    fn the_rate_limit_becomes_a_pause() {
        assert_eq!(
            refusal(429, b"", "anything", Some(Duration::from_secs(5))),
            MarketError::RateLimited {
                after: Duration::from_secs(5)
            }
        );
        assert_eq!(
            refusal(429, b"", "anything", None),
            MarketError::RateLimited {
                after: DEFAULT_BACKOFF
            }
        );
    }

    #[test]
    fn the_connection_ceiling_is_not_a_pause() {
        assert_eq!(
            refusal(509, b"", "anything", None),
            MarketError::TooManyConnections
        );
    }

    #[test]
    fn a_server_fault_is_the_market_being_unreachable() {
        let error = refusal(503, b"", "anything", None);
        assert!(matches!(error, MarketError::Unreachable(_)));
        assert!(error.counts_against_the_source());
    }

    #[test]
    fn paths_are_built_on_the_host_they_were_given() {
        let (gate, _doorman) = Gate::open(
            crate::gate::Budget {
                per_second: 3.0,
                burst: 3.0,
                failures_before_open: 3,
                cooldown: Duration::from_secs(60),
            },
            std::time::Instant::now(),
        );
        let client = Client::new(
            gate,
            Host {
                v1: "http://localhost/v1".into(),
                v2: "http://localhost/v2".into(),
            },
            "median-test/0.1",
        )
        .expect("client builds");
        assert_eq!(
            client.v1("/items/x/statistics"),
            "http://localhost/v1/items/x/statistics"
        );
        assert_eq!(client.v2("/item/x"), "http://localhost/v2/item/x");
    }
}
