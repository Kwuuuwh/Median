use std::collections::BTreeSet;
use std::sync::{Arc, PoisonError, RwLock};

use median_core::seconds;
use median_market::{Order, Side};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::warn;

use crate::market::store::{Observation, Store};
use crate::market::variant::Variant;

/// Items with order-stream activity since their book was last fetched.
#[derive(Clone, Default)]
pub struct Stirred(Arc<RwLock<BTreeSet<String>>>);

impl Stirred {
    pub fn note(&self, slug: &str) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(slug.to_owned());
    }

    pub fn moved(&self, slug: &str) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(slug)
    }

    pub fn settle(&self, slug: &str) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(slug);
    }
}

/// Saves every socket order to the observation log and passes it to the agent for evaluation.
/// The market cache records the order stream whether or not the agent is running.
pub async fn tend(
    store: Arc<Store>,
    carried: Arc<Mutex<Receiver<Order>>>,
    judging: Sender<Order>,
    stirred: Stirred,
) -> Result<(), String> {
    let mut carried = carried.lock().await;

    while let Some(order) = carried.recv().await {
        write_down(&store, &stirred, &order).await;
        // The agent falls behind on a busy stream; when its queue is full this order is dropped
        // instead of blocking the stream, since the newer orders matter more.
        let _ = judging.try_send(order);
    }
    Ok(())
}

/// Saves one order-stream order to the observation log and marks its item as having activity.
async fn write_down(store: &Store, stirred: &Stirred, order: &Order) {
    let Some(id) = order.item_id.clone() else {
        return;
    };
    let slug = match store.slug_of(id).await {
        Ok(Some(slug)) => slug,
        Ok(None) => return,
        Err(why) => {
            warn!(error = %why, "order stream items not resolved");
            return;
        }
    };

    let of = Variant::of_order(order).key();
    let observation = Observation {
        id: order.id.clone(),
        slug: slug.clone(),
        variant: of.clone(),
        selling: order.side == Side::Sell,
        platinum: order.platinum,
        quantity: order.quantity,
        per_trade: order.per_trade,
        reachable: order.user.status.reachable(),
        at: seconds(),
    };
    if let Err(why) = store.keep_observation(observation).await {
        warn!(error = %why, slug, "order stream observation not saved");
        return;
    }
    stirred.note(&slug);
}
