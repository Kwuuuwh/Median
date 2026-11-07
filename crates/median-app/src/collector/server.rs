use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use median_core::{CollectorEvent, Event, Feeds, seconds};
use median_engine::Input;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tracing::{Instrument, debug, info, info_span, warn};

use crate::account::Reader;
use crate::collector::protocol::{Arrived, Envelope, Kind, PROTOCOL, Reason, feeds_of};

/// Time without any received message after which the collector connection counts as lost.
pub(crate) const SILENCE: Duration = Duration::from_secs(35);

/// Minimum seconds between two `Received` reports to the engine on one connection.
const REPORT_EVERY: median_core::Instant = 5;

type Socket = WebSocketStream<TcpStream>;

/// Serves collector WebSocket connections on `address`; returns only when binding or accepting
/// fails.
pub async fn serve(
    inputs: Sender<Input>,
    dumps: Arc<Reader>,
    address: SocketAddr,
) -> Result<(), String> {
    let listener = TcpListener::bind(address)
        .await
        .map_err(|e| e.to_string())?;
    info!(%address, "collector server listening");
    answer(listener, inputs, dumps, SILENCE).await
}

/// Accepts connections on `listener`; each new connection closes the previous one, so one is
/// active at a time.
async fn answer(
    listener: TcpListener,
    inputs: Sender<Input>,
    dumps: Arc<Reader>,
    silence: Duration,
) -> Result<(), String> {
    let (evict, _) = watch::channel(0u64);
    let mut connections: u64 = 0;

    loop {
        let (stream, peer) = listener.accept().await.map_err(|e| e.to_string())?;
        connections += 1;
        let _ = evict.send(connections);

        let displaced = evict.subscribe();
        let inputs = inputs.clone();
        let dumps = Arc::clone(&dumps);
        let span = info_span!("collector.link", %peer);
        tokio::spawn(
            async move {
                if let Err(reason) = talk(stream, inputs, dumps, displaced, silence).await {
                    warn!(error = %reason, "collector connection failed");
                }
            }
            .instrument(span),
        );
    }
}

/// Handles one connection: handshake, then envelopes until `silence` passes without a message,
/// the collector closes, or a newer connection replaces it.
async fn talk(
    stream: TcpStream,
    inputs: Sender<Input>,
    dumps: Arc<Reader>,
    mut displaced: watch::Receiver<u64>,
    silence: Duration,
) -> Result<(), String> {
    let socket = tokio_tungstenite::accept_hdr_async(stream, no_browser)
        .await
        .map_err(|e| e.to_string())?;
    let (mut writer, mut reader) = socket.split();
    let mut last = Instant::now();
    let mut reported: Option<median_core::Instant> = None;

    loop {
        let arrived = tokio::select! {
            biased;
            _ = displaced.changed() => {
                info!("collector connection replaced by a newer one");
                return Ok(());
            }
            arrived = timeout(silence, reader.next()) => arrived,
        };

        let text = match arrived {
            Err(_) => {
                info!(
                    silence_s = silence.as_secs(),
                    "collector silent, connection closed"
                );
                return tell(&inputs, CollectorEvent::Silent).await;
            }
            Ok(None) | Ok(Some(Ok(Message::Close(_)))) => {
                info!("collector disconnected");
                return tell(&inputs, CollectorEvent::Silent).await;
            }
            Ok(Some(Err(e))) => {
                tell(&inputs, CollectorEvent::Silent).await?;
                return Err(e.to_string());
            }
            Ok(Some(Ok(Message::Text(text)))) => text,
            // The protocol uses text frames only.
            Ok(Some(Ok(_))) => continue,
        };

        let received = read(&text, &mut writer, &mut last, &dumps).await?;
        if let Received::Envelope | Received::Feeds(_) = received {
            // The first envelope is reported at once; later ones after `REPORT_EVERY` seconds.
            let at = seconds();
            if reported.is_none_or(|then| at - then >= REPORT_EVERY) {
                reported = Some(at);
                tell(&inputs, CollectorEvent::Received { at }).await?;
            }
        }
        match received {
            Received::Envelope => {}
            Received::Feeds(feeds) => tell(&inputs, CollectorEvent::Feeds(feeds)).await?,
            Received::Nothing => {}
            Received::OtherProtocol(speaks) => {
                return tell(&inputs, CollectorEvent::Outdated { speaks }).await;
            }
        }
    }
}

/// Outcome of reading one received message.
enum Received {
    /// An envelope of this protocol version, answered where the protocol requires a reply.
    Envelope,
    /// An envelope reporting the state of Overwolf's game-event feeds.
    Feeds(Feeds),
    /// Not a parsable envelope; dropped.
    Nothing,
    /// An envelope of another protocol version; the connection ends.
    OtherProtocol(u32),
}

/// Parses one message, replies where needed, passes event payloads to `account::Reader`, and
/// logs its size and the time since `last`.
async fn read(
    text: &str,
    writer: &mut SplitSink<Socket, Message>,
    last: &mut Instant,
    dumps: &Reader,
) -> Result<Received, String> {
    let Ok(arrived) = serde_json::from_str::<Arrived>(text) else {
        warn!(bytes = text.len(), "collector message unreadable");
        return Ok(Received::Nothing);
    };

    if arrived.v != PROTOCOL {
        warn!(
            collector_protocol = arrived.v,
            expected = PROTOCOL,
            "collector protocol mismatch, connection refused"
        );
        send(writer, Envelope::refusal(Reason::Version)).await?;
        return Ok(Received::OtherProtocol(arrived.v));
    }

    debug!(
        bytes = text.len(),
        kind = arrived.kind.unwrap_or("none"),
        feature = arrived.feature.unwrap_or("none"),
        payload_bytes = arrived.payload.map_or(0, |raw| raw.get().len()),
        since_last_ms = last.elapsed().as_millis() as u64,
        "collector message received"
    );
    *last = Instant::now();

    // An unknown kind still counts as a received envelope.
    match arrived.kind.and_then(Kind::named) {
        Some(Kind::Hello) => send(writer, Envelope::bare(Kind::Welcome)).await?,
        Some(Kind::Ping) => send(writer, Envelope::bare(Kind::Pong)).await?,
        Some(Kind::Event) => {
            if let Some(payload) = arrived.payload {
                dumps.take(payload.get()).await;
            }
        }
        Some(Kind::Diag) => {
            if let Some(payload) = arrived.payload {
                match feeds_of(payload.get()) {
                    Some(Feeds::Refused { reason }) => {
                        warn!(%reason, "game-event feeds refused by Overwolf");
                        return Ok(Received::Feeds(Feeds::Refused { reason }));
                    }
                    Some(feeds) => {
                        info!(feeds = ?feeds, "game-event feeds reported");
                        return Ok(Received::Feeds(feeds));
                    }
                    None => warn!(text = payload.get(), "collector reported a problem"),
                }
            }
        }
        _ => {}
    }
    Ok(Received::Envelope)
}

/// Rejects with 403 a handshake whose origin is `http://` or `https://`, so web pages cannot
/// connect.
#[allow(clippy::result_large_err)]
fn no_browser(request: &Request, response: Response) -> Result<Response, ErrorResponse> {
    let origin = request
        .headers()
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !(origin.starts_with("http://") || origin.starts_with("https://")) {
        return Ok(response);
    }

    warn!(%origin, "collector connection from web page refused");
    let mut refusal = ErrorResponse::new(Some("median does not answer web pages".to_string()));
    *refusal.status_mut() = StatusCode::FORBIDDEN;
    Err(refusal)
}

async fn send(writer: &mut SplitSink<Socket, Message>, envelope: Envelope) -> Result<(), String> {
    let text = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
    writer
        .send(Message::Text(text.into()))
        .await
        .map_err(|e| e.to_string())
}

/// Sends a collector event to the engine; `Err` when the engine input is closed.
async fn tell(inputs: &Sender<Input>, event: CollectorEvent) -> Result<(), String> {
    inputs
        .send(Input::Command(Event::Collector(event)))
        .await
        .map_err(|_| "the engine is gone".to_string())
}

#[cfg(test)]
mod tests {
    use median_core::CollectorEvent;
    use tokio::sync::mpsc::{self, Receiver};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    use super::*;
    use median_core::millis;

    /// Starts a server on a port the OS picks, so tests do not collide.
    async fn start(silence: Duration) -> (SocketAddr, Receiver<Input>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let address = listener.local_addr().expect("the bound address");
        let (tx, rx) = mpsc::channel(64);
        let journal = Arc::new(crate::account::Journal::under(
            &std::env::temp_dir().join("median-collector-tests"),
        ));
        let dumps = Arc::new(Reader::new(tx.clone(), journal));
        tokio::spawn(answer(listener, tx, dumps, silence));
        (address, rx)
    }

    async fn connect(address: SocketAddr) -> Socket {
        let stream = TcpStream::connect(address).await.expect("a connection");
        let request = format!("ws://{address}/")
            .into_client_request()
            .expect("a request");
        let (socket, _) = tokio_tungstenite::client_async(request, stream)
            .await
            .expect("a handshake");
        socket
    }

    /// A bare envelope of `kind` as a text message, as a client sends it.
    fn envelope(kind: Kind) -> Message {
        Message::Text(
            serde_json::to_string(&Envelope::bare(kind))
                .expect("an envelope")
                .into(),
        )
    }

    async fn reply(socket: &mut Socket) -> Envelope {
        let message = socket.next().await.expect("a reply").expect("a message");
        let Message::Text(text) = message else {
            panic!("the server answered with something other than text");
        };
        serde_json::from_str(&text).expect("an envelope")
    }

    /// The next collector event sent to the engine.
    async fn told(rx: &mut Receiver<Input>) -> CollectorEvent {
        loop {
            match rx.recv().await.expect("an input") {
                Input::Command(Event::Collector(event)) => return event,
                _ => continue,
            }
        }
    }

    #[tokio::test]
    async fn a_hello_is_answered_with_welcome() {
        let (address, mut rx) = start(SILENCE).await;
        let mut socket = connect(address).await;

        socket.send(envelope(Kind::Hello)).await.expect("a send");

        assert_eq!(reply(&mut socket).await.kind, Kind::Welcome);
        assert!(matches!(
            told(&mut rx).await,
            CollectorEvent::Received { .. }
        ));
    }

    #[tokio::test]
    async fn an_event_is_taken_before_the_handshake() {
        let (address, mut rx) = start(SILENCE).await;
        let mut socket = connect(address).await;

        socket.send(envelope(Kind::Event)).await.expect("a send");

        assert!(matches!(
            told(&mut rx).await,
            CollectorEvent::Received { .. }
        ));
    }

    #[tokio::test]
    async fn a_ping_is_answered_with_pong() {
        let (address, _rx) = start(SILENCE).await;
        let mut socket = connect(address).await;

        socket.send(envelope(Kind::Ping)).await.expect("a send");

        assert_eq!(reply(&mut socket).await.kind, Kind::Pong);
    }

    #[tokio::test]
    async fn a_second_collector_takes_the_link_over() {
        let (address, _rx) = start(SILENCE).await;
        let mut first = connect(address).await;
        first.send(envelope(Kind::Hello)).await.expect("a send");
        assert_eq!(reply(&mut first).await.kind, Kind::Welcome);

        let mut second = connect(address).await;
        second.send(envelope(Kind::Hello)).await.expect("a send");

        assert_eq!(reply(&mut second).await.kind, Kind::Welcome);
        // The first connection is closed without any protocol message.
        while let Some(Ok(message)) = first.next().await {
            assert!(matches!(message, Message::Close(_)), "{message:?}");
        }
    }

    /// Whether a handshake with this `origin` header is accepted.
    async fn admits(address: SocketAddr, origin: &str) -> bool {
        let stream = TcpStream::connect(address).await.expect("a connection");
        let mut request = format!("ws://{address}/")
            .into_client_request()
            .expect("a request");
        request
            .headers_mut()
            .insert("origin", origin.parse().expect("a header"));

        tokio_tungstenite::client_async(request, stream)
            .await
            .is_ok()
    }

    #[tokio::test]
    async fn a_web_page_is_refused() {
        let (address, _rx) = start(SILENCE).await;

        for origin in ["https://example.test", "http://localhost:5173"] {
            assert!(!admits(address, origin).await, "{origin} was let in");
        }
    }

    #[tokio::test]
    async fn the_page_overwolf_runs_the_collector_in_is_let_in() {
        let (address, _rx) = start(SILENCE).await;

        for origin in [
            "overwolf-extension://ophmdkgfcjapomjdpfobjfbihojchbko",
            "chrome-extension://ophmdkgfcjapomjdpfobjfbihojchbko",
        ] {
            assert!(admits(address, origin).await, "{origin} was refused");
        }
    }

    /// Sends `text` unchanged, so a test can send messages this build never produces.
    async fn say(socket: &mut Socket, text: String) {
        socket
            .send(Message::Text(text.into()))
            .await
            .expect("a send");
    }

    #[tokio::test]
    async fn another_protocol_version_is_told_so() {
        for speaks in [PROTOCOL - 1, PROTOCOL + 1] {
            let (address, mut rx) = start(SILENCE).await;
            let mut socket = connect(address).await;
            let other = Envelope {
                v: speaks,
                ..Envelope::bare(Kind::Hello)
            };

            say(&mut socket, serde_json::to_string(&other).expect("json")).await;

            let refusal = reply(&mut socket).await;
            assert_eq!(refusal.kind, Kind::Error);
            assert_eq!(refusal.reason, Some(Reason::Version));
            assert_eq!(told(&mut rx).await, CollectorEvent::Outdated { speaks });
        }
    }

    #[tokio::test]
    async fn a_kind_this_build_does_not_know_is_still_heard() {
        let (address, mut rx) = start(SILENCE).await;
        let mut socket = connect(address).await;

        say(
            &mut socket,
            format!(r#"{{"v":{PROTOCOL},"kind":"batch","ts":0}}"#),
        )
        .await;

        assert!(matches!(
            told(&mut rx).await,
            CollectorEvent::Received { .. }
        ));
    }

    #[tokio::test]
    async fn a_payload_is_carried_without_being_read() {
        let (address, mut rx) = start(SILENCE).await;
        let mut socket = connect(address).await;
        // Shapes no typed payload would accept: the server must not parse the payload.
        let payload = r#"{"source":"poll","info":[1,{"deep":{"deeper":[null,true]}},"x"]}"#;

        say(
            &mut socket,
            format!(r#"{{"v":{PROTOCOL},"kind":"event","ts":0,"payload":{payload}}}"#),
        )
        .await;

        assert!(matches!(
            told(&mut rx).await,
            CollectorEvent::Received { .. }
        ));
    }

    #[tokio::test]
    async fn a_burst_reports_the_link_live_once() {
        let (address, mut rx) = start(SILENCE).await;
        let mut socket = connect(address).await;

        for _ in 0..3 {
            socket.send(envelope(Kind::Event)).await.expect("a send");
        }
        // The welcome reply shows that every earlier envelope has been processed.
        socket.send(envelope(Kind::Hello)).await.expect("a send");
        assert_eq!(reply(&mut socket).await.kind, Kind::Welcome);

        let mut live = 0;
        while let Ok(Input::Command(Event::Collector(CollectorEvent::Received { .. }))) =
            rx.try_recv()
        {
            live += 1;
        }
        assert_eq!(live, 1, "a burst inside the window reported more than once");
    }

    #[tokio::test]
    async fn silence_takes_the_link_offline() {
        let (address, mut rx) = start(Duration::from_millis(100)).await;
        let _socket = connect(address).await;

        assert_eq!(told(&mut rx).await, CollectorEvent::Silent);
    }

    #[tokio::test]
    async fn an_envelope_stamps_the_instant_it_arrived() {
        let (address, mut rx) = start(SILENCE).await;
        let mut socket = connect(address).await;
        let before = millis() / 1000;

        socket.send(envelope(Kind::Event)).await.expect("a send");

        let CollectorEvent::Received { at } = told(&mut rx).await else {
            panic!("the engine was not told the link is live");
        };
        assert!(at >= before, "{at} is before the send");
    }
}
