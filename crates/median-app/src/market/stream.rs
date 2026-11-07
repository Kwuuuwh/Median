use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use median_core::{Event, MembershipEvent, Status};

use median_engine::Input;
use median_market::{Incoming, Order, SOCKET, SocketReader, SocketWriter, open_socket};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, info, warn};

use crate::market::Token;

/// How long to wait before opening the socket again after it closed.
const REOPEN: Duration = Duration::from_secs(10);

static LOST: AtomicU64 = AtomicU64::new(0);

/// Capacity of the socket request queue.
const WAITING: usize = 16;

/// Platform median trades on. Without it in the subscription, crossplay adds console orders to
/// the order stream.
const PLATFORM: &str = "pc";

/// A request to the open market socket.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Sign in with this token.
    SignIn(String),
    SignOut,
    Status(Status),
}

/// Handle for sending requests to the market socket.
#[derive(Clone)]
pub struct Stream(Sender<Ask>);

impl Stream {
    /// Queues a request to the socket; drops it when the queue is full.
    pub fn ask(&self, ask: Ask) {
        if self.0.try_send(ask).is_err() {
            warn!("market socket request dropped, queue full");
        }
    }
}

/// Creates the socket handle and the queue the socket task reads from.
pub fn channel() -> (Stream, Receiver<Ask>) {
    let (tx, rx) = tokio::sync::mpsc::channel(WAITING);
    (Stream(tx), rx)
}

/// Keeps the market socket open, reopening it `REOPEN` after every close.
pub async fn tend(
    tx: Sender<Input>,
    asks: Arc<Mutex<Receiver<Ask>>>,
    held: Arc<Mutex<Option<String>>>,
    flow: Sender<Order>,
) -> Result<(), String> {
    let mut asks = asks.lock().await;
    // The last requested status, kept across reconnects: the market drops it with the socket,
    // and the account would then show out of game while the app state is in game.
    let mut asked: Option<Status> = None;
    loop {
        match open_socket(SOCKET, crate::http::USER_AGENT).await {
            Ok((mut writer, mut reader)) => {
                info!("market socket connected");
                let token = held.lock().await.clone();
                let signed = match token {
                    Some(token) => match writer.sign_in(&token).await {
                        Ok(()) => true,
                        Err(e) => {
                            warn!(error = %e, "market socket token send failed");
                            false
                        }
                    },
                    None => false,
                };
                // The market keeps nothing across a reconnect, so the subscription is sent again.
                if let Err(e) = writer.subscribe(PLATFORM).await {
                    warn!(error = %e, "market socket subscription failed");
                }
                if signed && let Some(status) = asked {
                    match writer.status(status.as_code()).await {
                        Ok(()) => info!(status = status.as_code(), "market status sent again"),
                        Err(e) => warn!(error = %e, "market status send failed"),
                    }
                }
                serve(
                    &tx,
                    &mut asks,
                    &held,
                    &flow,
                    &mut writer,
                    &mut reader,
                    &mut asked,
                )
                .await;
                info!("market socket closed");
            }
            Err(e) => warn!(error = %e, "market socket connect failed"),
        }
        tokio::time::sleep(REOPEN).await;
    }
}

/// Serves the open socket until it closes. `asked` is the status of the last status request:
/// the app state changes on the market's confirmation of that command, because the format of
/// the market's own status event is not documented.
async fn serve(
    tx: &Sender<Input>,
    asks: &mut Receiver<Ask>,
    held: &Arc<Mutex<Option<String>>>,
    flow: &Sender<Order>,
    writer: &mut SocketWriter,
    reader: &mut SocketReader,
    asked: &mut Option<Status>,
) {
    loop {
        tokio::select! {
            ask = asks.recv() => {
                let Some(ask) = ask else { return };
                match &ask {
                    Ask::Status(status) => *asked = Some(*status),
                    // After sign-out there is no status to restore.
                    Ask::SignOut => *asked = None,
                    Ask::SignIn(_) => {}
                }
                if !obey(ask, held, writer).await {
                    return;
                }
            }
            incoming = reader.next() => {
                match incoming {
                    Some(Ok(incoming)) => tell(tx, flow, incoming, *asked).await,
                    Some(Err(e)) => warn!(error = %e, "market socket message unreadable"),
                    None => return,
                }
            }
        }
    }
}

/// Sends one request to the socket; returns whether the socket is still usable.
async fn obey(ask: Ask, held: &Arc<Mutex<Option<String>>>, writer: &mut SocketWriter) -> bool {
    let sent = match &ask {
        Ask::SignIn(token) => {
            *held.lock().await = Some(token.clone());
            writer.sign_in(token).await
        }
        Ask::SignOut => {
            *held.lock().await = None;
            writer.sign_out().await
        }
        Ask::Status(status) => writer.status(status.as_code()).await,
    };

    match sent {
        Ok(()) => true,
        Err(e) => {
            warn!(error = %e, "market socket request failed");
            false
        }
    }
}

/// Passes one socket message on: orders to `flow`, status changes to the engine. `asked` is
/// the status of the last status request.
async fn tell(tx: &Sender<Input>, flow: &Sender<Order>, incoming: Incoming, asked: Option<Status>) {
    let event = match incoming {
        Incoming::Status(code) => match Status::parse(&code) {
            Some(status) => MembershipEvent::StatusSet(status),
            None => {
                warn!(code = ?code, "market sent unknown status");
                return;
            }
        },
        // The market accepted the command, so the account now has the requested status.
        Incoming::Answer {
            route,
            ok: true,
            said,
        } if route.ends_with("/status/set") => {
            debug!(reply = ?said, "market status accepted");
            match asked {
                Some(status) => MembershipEvent::StatusSet(status),
                None => return,
            }
        }
        // A refused sign-in means the token is invalid; it is the only refusal that changes
        // app state.
        Incoming::Answer {
            route,
            ok: false,
            said,
        } if route.ends_with("/auth/signIn") => {
            warn!(reply = ?said, "market socket token refused");
            Token::clear();
            MembershipEvent::Refused
        }
        Incoming::Answer {
            route,
            ok: false,
            said,
        } => {
            warn!(reply = ?said, route, "market socket request refused");
            return;
        }
        // When the agent falls behind, the order is dropped: newer orders matter more than the
        // ones lost.
        Incoming::Order(order) => {
            if flow.try_send(*order).is_err() {
                let lost = LOST.fetch_add(1, Ordering::Relaxed) + 1;
                warn!(skipped = lost, "order stream queue full, orders skipped");
            }
            return;
        }
        Incoming::Answer { .. } => return,
        // Other messages are not used; `RUST_LOG=median_app=debug` logs their routes for
        // inspection.
        Incoming::Other(route) => {
            debug!(route, "market socket message ignored");
            return;
        }
    };

    let _ = tx.send(Input::Command(Event::Membership(event))).await;
}
