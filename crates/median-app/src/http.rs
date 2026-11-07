/// `User-Agent` sent with every HTTP request and WebSocket connection: `median/r<revision>`.
pub(crate) const USER_AGENT: &str = concat!("median/r", env!("MEDIAN_REVISION"));

/// Builds an HTTP client with the app's user agent for public sources (game data, world state,
/// riven sheets); `Err` holds the builder's error text.
pub(crate) fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())
}
