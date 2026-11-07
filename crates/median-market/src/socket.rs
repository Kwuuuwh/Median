use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::error::MarketError;
use crate::model::Order;
use crate::session::bare;

/// URL of the warframe.market WebSocket.
pub const SOCKET: &str = "wss://ws.warframe.market/socket";

/// The subprotocol the server requires; a connection without it is rejected.
const PROTOCOL: &str = "wfm";

const SIGN_IN: &str = "@wfm|cmd/auth/signIn";
const SIGN_OUT: &str = "@wfm|cmd/auth/signOut";
const SET_STATUS: &str = "@wfm|cmd/status/set";
const SUBSCRIBE: &str = "@wfm|cmd/subscribe/newOrders";
const NEW_ORDER: &str = "@wfm|event/subscriptions/newOrder";
const STATUS_SET: &str = "@wfm|event/status/set";

/// The wrapper of every message on the socket.
#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    payload: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

/// A message received on the socket.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    /// A new visible order from a user who is not offline (the order stream).
    Order(Box<Order>),
    /// The server reported the account's status.
    Status(String),
    /// The response to a command.
    Answer {
        route: String,
        ok: bool,
        /// The response payload; on a rejection it holds the reason.
        said: Option<serde_json::Value>,
    },
    /// A route this client does not handle.
    Other(String),
}

type Live = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// The sending half of the socket.
pub struct SocketWriter(SplitSink<Live, Message>);

/// The receiving half of the socket.
pub struct SocketReader(SplitStream<Live>);

/// Opens the socket; `agent` is sent verbatim as `User-Agent`, and the market rejects a socket
/// without one.
pub async fn open(url: &str, agent: &str) -> Result<(SocketWriter, SocketReader), MarketError> {
    let mut request = url
        .into_client_request()
        .map_err(|e| MarketError::Unreachable(e.to_string()))?;
    let headers = request.headers_mut();
    headers.insert("sec-websocket-protocol", HeaderValue::from_static(PROTOCOL));
    headers.insert(
        "user-agent",
        HeaderValue::from_str(agent).map_err(|e| MarketError::Unreachable(e.to_string()))?,
    );

    let (live, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| MarketError::Unreachable(e.to_string()))?;

    let (sink, stream) = live.split();
    Ok((SocketWriter(sink), SocketReader(stream)))
}

impl SocketWriter {
    /// Signs the socket in with the token; setting the status requires a signed-in socket. The
    /// issued header value has a scheme word in front; the payload takes the bare token.
    pub async fn sign_in(&mut self, authorization: &str) -> Result<(), MarketError> {
        self.send(SIGN_IN, serde_json::json!({ "token": bare(authorization) }))
            .await
    }

    pub async fn sign_out(&mut self) -> Result<(), MarketError> {
        self.send(SIGN_OUT, serde_json::json!({})).await
    }

    /// Sets the account's status: `online`, `ingame` or `invisible`.
    pub async fn status(&mut self, status: &str) -> Result<(), MarketError> {
        self.send(SET_STATUS, serde_json::json!({ "status": status }))
            .await
    }

    /// Subscribes to the order stream of new orders on `platform`. Needs no token.
    pub async fn subscribe(&mut self, platform: &str) -> Result<(), MarketError> {
        self.send(SUBSCRIBE, serde_json::json!({ "platform": platform }))
            .await
    }

    async fn send(&mut self, route: &str, payload: serde_json::Value) -> Result<(), MarketError> {
        let envelope = Envelope {
            route: route.to_owned(),
            payload: Some(payload),
            id: None,
        };
        let text =
            serde_json::to_string(&envelope).map_err(|e| MarketError::Malformed(e.to_string()))?;
        self.0
            .send(Message::Text(text.into()))
            .await
            .map_err(|e| MarketError::Unreachable(e.to_string()))
    }
}

impl SocketReader {
    /// The next message received; `None` once the socket closes.
    pub async fn next(&mut self) -> Option<Result<Incoming, MarketError>> {
        loop {
            return match self.0.next().await? {
                Ok(Message::Text(text)) => Some(read(&text)),
                Ok(Message::Close(_)) => None,
                // The protocol uses text frames only.
                Ok(_) => continue,
                Err(e) => Some(Err(MarketError::Unreachable(e.to_string()))),
            };
        }
    }
}

/// Parses one text message into an `Incoming`.
fn read(text: &str) -> Result<Incoming, MarketError> {
    let envelope: Envelope =
        serde_json::from_str(text).map_err(|e| MarketError::Malformed(format!("socket: {e}")))?;

    match envelope.route.as_str() {
        NEW_ORDER => {
            let payload = envelope
                .payload
                .ok_or_else(|| MarketError::Malformed("newOrder carried nothing".to_owned()))?;
            let order = serde_json::from_value(payload)
                .map_err(|e| MarketError::Malformed(format!("newOrder: {e}")))?;
            Ok(Incoming::Order(Box::new(order)))
        }
        // The event format is undocumented: the status is either at the top of the payload or
        // one level in; in any other shape it is empty, and the caller ignores it.
        STATUS_SET => Ok(Incoming::Status(
            envelope
                .payload
                .as_ref()
                .and_then(|payload| match payload.get("status") {
                    Some(serde_json::Value::Object(inner)) => inner.get("status"),
                    found => found,
                })
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        )),
        route if route.ends_with(":ok") => Ok(Incoming::Answer {
            route: route.trim_end_matches(":ok").to_owned(),
            ok: true,
            said: envelope.payload,
        }),
        route if route.ends_with(":error") => Ok(Incoming::Answer {
            route: route.trim_end_matches(":error").to_owned(),
            ok: false,
            said: envelope.payload,
        }),
        route => Ok(Incoming::Other(route.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheme_word_does_not_reach_the_payload() {
        assert_eq!(bare("JWT eyJhbGciOi"), "eyJhbGciOi");
        assert_eq!(bare("eyJhbGciOi"), "eyJhbGciOi");
    }

    #[test]
    fn a_status_event_carries_the_word_the_server_used() {
        let incoming = read(r#"{"route":"@wfm|event/status/set","payload":{"status":"ingame"}}"#);

        assert_eq!(incoming, Ok(Incoming::Status("ingame".to_owned())));
    }

    #[test]
    fn an_answer_is_told_apart_from_a_refusal() {
        assert_eq!(
            read(r#"{"route":"@wfm|cmd/status/set:ok","payload":{}}"#),
            Ok(Incoming::Answer {
                route: "@wfm|cmd/status/set".to_owned(),
                ok: true,
                said: Some(serde_json::json!({}))
            })
        );
        assert_eq!(
            read(r#"{"route":"@wfm|cmd/status/set:error","payload":{}}"#),
            Ok(Incoming::Answer {
                route: "@wfm|cmd/status/set".to_owned(),
                ok: false,
                said: Some(serde_json::json!({}))
            })
        );
    }

    #[test]
    fn a_route_nothing_is_done_about_is_kept_by_name() {
        assert_eq!(
            read(r#"{"route":"@wfm|event/reports/online","payload":{"connections":52040}}"#),
            Ok(Incoming::Other("@wfm|event/reports/online".to_owned()))
        );
    }

    #[test]
    fn a_message_that_is_not_an_envelope_is_malformed() {
        assert!(matches!(read("nonsense"), Err(MarketError::Malformed(_))));
    }
}
