use std::time::Duration;

use thiserror::Error;

/// Why a warframe.market call failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MarketError {
    /// warframe.market could not be reached, or returned a response that cannot be used.
    #[error("market unreachable: {0}")]
    Unreachable(String),

    /// warframe.market rate-limited the client; no request may be sent until `after` elapses.
    #[error("rate limited for {}s", after.as_secs())]
    RateLimited { after: Duration },

    /// The market's limit on concurrent connections is reached; one must close before another
    /// opens.
    #[error("market allows no more connections at once")]
    TooManyConnections,

    /// The call needs a session and there is none (or it expired).
    #[error("market call needs authorization")]
    Unauthorized,

    /// The session is valid, but it does not allow this call.
    #[error("market refused the right to: {0}")]
    Forbidden(String),

    /// The market rejected the request as sent and gave the reason.
    #[error("market refused the request: {0}")]
    Refused(String),

    /// The session's write permission does not cover the item the call names.
    #[error("no right to write about: {0}")]
    NotPermitted(String),

    /// The item is known locally, but the market does not list it.
    #[error("item is not listed on the market: {0}")]
    NotListed(String),

    /// The endpoint for this call is permanently retired.
    #[error("market endpoint retired: {0}")]
    Retired(String),

    /// The response arrived but did not parse.
    #[error("market answer did not parse: {0}")]
    Malformed(String),
}

impl MarketError {
    /// Whether the failure concerns warframe.market as a whole rather than one item.
    pub fn counts_against_the_source(&self) -> bool {
        match self {
            Self::Unreachable(_)
            | Self::RateLimited { .. }
            | Self::TooManyConnections
            | Self::Forbidden(_) => true,
            Self::Unauthorized
            | Self::Refused(_)
            | Self::NotPermitted(_)
            | Self::NotListed(_)
            | Self::Retired(_)
            | Self::Malformed(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_source_wide_failures_count_against_the_source() {
        assert!(MarketError::Unreachable("timeout".into()).counts_against_the_source());
        assert!(
            MarketError::RateLimited {
                after: Duration::from_secs(30)
            }
            .counts_against_the_source()
        );
        assert!(!MarketError::NotListed("excalibur_prime".into()).counts_against_the_source());
        assert!(!MarketError::Retired("v1 statistics".into()).counts_against_the_source());
        assert!(!MarketError::Unauthorized.counts_against_the_source());
        assert!(!MarketError::Malformed("field median".into()).counts_against_the_source());
        assert!(!MarketError::Refused("bad password".into()).counts_against_the_source());
        assert!(MarketError::Forbidden("items".into()).counts_against_the_source());
        assert!(!MarketError::NotPermitted("rubedo".into()).counts_against_the_source());
        assert!(MarketError::TooManyConnections.counts_against_the_source());
    }
}
