use median_core::millis;

use median_core::Feeds;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Version of the collector protocol this build speaks.
pub const PROTOCOL: u32 = 2;

/// Loopback port the collector connects to.
pub const PORT: u16 = 42010;

/// Message type of an envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Hello,
    Welcome,
    Event,
    Diag,
    Ping,
    Pong,
    Error,
}

impl Kind {
    /// Kind with the wire name `name`; `None` for a name this protocol version does not define.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "hello" => Some(Kind::Hello),
            "welcome" => Some(Kind::Welcome),
            "event" => Some(Kind::Event),
            "diag" => Some(Kind::Diag),
            "ping" => Some(Kind::Ping),
            "pong" => Some(Kind::Pong),
            "error" => Some(Kind::Error),
            _ => None,
        }
    }
}

/// Reason the server rejected a collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
    Version,
}

/// One message of the collector protocol, as this build writes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    pub ts: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Box<RawValue>>,
}

impl Envelope {
    /// Envelope of `kind` with no payload, stamped with the current time in milliseconds.
    pub fn bare(kind: Kind) -> Self {
        Self {
            v: PROTOCOL,
            kind,
            feature: None,
            reason: None,
            ts: millis(),
            payload: None,
        }
    }

    /// `error` envelope with `reason`, sent to a collector on another protocol version.
    pub fn refusal(reason: Reason) -> Self {
        Self {
            reason: Some(reason),
            ..Self::bare(Kind::Error)
        }
    }
}

/// A received envelope, with every field a newer protocol version could change kept as text.
#[derive(Debug, Deserialize)]
pub struct Arrived<'a> {
    pub v: u32,
    #[serde(default, borrow)]
    pub kind: Option<&'a str>,
    #[serde(default, borrow)]
    pub feature: Option<&'a str>,
    /// The payload as raw JSON text; it is not parsed here.
    #[serde(default, borrow)]
    pub payload: Option<&'a RawValue>,
}

/// Tags of the `diag` messages in which the collector reports Overwolf's game-event feeds; the
/// collector sends them as written here (`apps/collector/src/background.ts`, `log`).
const FEEDS_UP: &str = "the game-event feeds are up";
const FEEDS_NOT_UP: &str = "the game-event feeds are not up";
const FEEDS_REFUSED: &str = "the game-event feeds were refused";

/// Payload of a `diag` message: a fixed tag and an optional detail.
#[derive(Debug, Deserialize)]
struct Diag {
    tag: String,
    #[serde(default)]
    detail: Option<serde_json::Value>,
}

/// What a `diag` payload says about the game-event feeds; `None` for any other diagnostic or an
/// unreadable payload. A refusal without a readable reason gets an empty one.
pub fn feeds_of(payload: &str) -> Option<Feeds> {
    let diag: Diag = serde_json::from_str(payload).ok()?;
    match diag.tag.as_str() {
        FEEDS_UP => Some(Feeds::Up),
        FEEDS_NOT_UP => Some(Feeds::Waiting),
        FEEDS_REFUSED => Some(Feeds::Refused {
            reason: match diag.detail {
                Some(serde_json::Value::String(reason)) => reason,
                Some(other) => other.to_string(),
                None => String::new(),
            },
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_feeds_tags_are_read_and_other_diagnostics_are_not() {
        assert_eq!(
            feeds_of(r#"{"tag":"the game-event feeds are up"}"#),
            Some(Feeds::Up)
        );
        assert_eq!(
            feeds_of(r#"{"tag":"the game-event feeds are not up"}"#),
            Some(Feeds::Waiting)
        );
        assert_eq!(
            feeds_of(r#"{"tag":"the game-event feeds were refused","detail":"not supported"}"#),
            Some(Feeds::Refused {
                reason: "not supported".into()
            })
        );
        assert_eq!(feeds_of(r#"{"tag":"something else"}"#), None);
        assert_eq!(feeds_of("not json"), None);
    }
}
