use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::agent::gear::Gear;
use crate::market::Priority;

/// Interval between checks of whether the agent is running.
const PAUSE: Duration = Duration::from_secs(30);

/// Evaluates the market's recent orders once each time the agent starts.
///
/// The socket delivers orders only as they are placed, so orders already open when the agent
/// started never arrive on it. The market's recent-orders list covers that gap with one
/// request; every later order arrives on the socket.
pub async fn tend(gear: Arc<Gear>, running: Arc<Mutex<bool>>) -> Result<(), String> {
    let mut caught_up = false;

    loop {
        // Read through `still`, never through a guard held across the pass: the pass itself
        // checks whether the agent is still running.
        if !still(&running).await {
            // While the agent is stopped no socket order reaches it, so the next start needs
            // its own recent-orders pass.
            caught_up = false;
        } else if !caught_up {
            recent(&gear, &running).await;
            caught_up = true;
        }
        sleep(PAUSE).await;
    }
}

/// Whether the agent is still running. A pass that started before the agent was stopped ends
/// early instead of evaluating the rest of the list.
async fn still(running: &Mutex<bool>) -> bool {
    *running.lock().await
}

/// Evaluates every order in the market's recent-orders list, read with one request.
async fn recent(gear: &Gear, running: &Mutex<bool>) {
    let recent = gear
        .gateway
        .call(Priority::Background, |pass| {
            gear.gateway.client().recent(pass)
        })
        .await;
    match recent {
        Ok(orders) => {
            debug!(orders = orders.len(), "recent orders read");
            for order in &orders {
                if !still(running).await {
                    return;
                }
                gear.look(order).await;
            }
        }
        Err(why) => warn!(error = %why, "recent orders read failed"),
    }
}
