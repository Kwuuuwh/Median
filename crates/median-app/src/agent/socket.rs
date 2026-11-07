use std::sync::Arc;

use median_market::Order;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Receiver;

use crate::agent::gear::Gear;

/// Passes every order from the market socket to the agent while the agent runs; runs for the
/// life of the application. An order arrives here within a second of being placed.
pub async fn tend(
    gear: Arc<Gear>,
    orders: Arc<Mutex<Receiver<Order>>>,
    running: Arc<Mutex<bool>>,
) -> Result<(), String> {
    let mut orders = orders.lock().await;

    while let Some(order) = orders.recv().await {
        if !*running.lock().await {
            continue;
        }
        gear.look(&order).await;
    }
    Ok(())
}
