use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use median_core::seconds;
use median_market::Auction;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::{debug, warn};

use super::gateway::{Gateway, Priority};
use super::store::Store;

/// Interval between reads of the market's newest-auctions feed.
const EVERY: Duration = Duration::from_secs(20);

/// Most auctions kept waiting; past this the oldest are dropped.
const MOST_WAITING: usize = 500;

/// New auctions from the feed, waiting to be evaluated.
#[derive(Default)]
pub struct Fresh {
    waiting: Mutex<Vec<Auction>>,
    arrived: Notify,
}

impl Fresh {
    fn add(&self, auctions: Vec<Auction>) {
        if auctions.is_empty() {
            return;
        }
        let mut waiting = self.waiting.lock().unwrap_or_else(PoisonError::into_inner);
        waiting.extend(auctions);
        let over = waiting.len().saturating_sub(MOST_WAITING);
        waiting.drain(..over);
        self.arrived.notify_one();
    }

    /// Takes all waiting auctions, waiting until there is at least one.
    pub async fn take(&self) -> Vec<Auction> {
        loop {
            let taken =
                std::mem::take(&mut *self.waiting.lock().unwrap_or_else(PoisonError::into_inner));
            if !taken.is_empty() {
                return taken;
            }
            self.arrived.notified().await;
        }
    }
}

/// Reads the market's newest riven auctions every `EVERY`, saves them in the market cache, and
/// adds the ones not seen in the previous read to `fresh`.
pub async fn tend(gateway: Gateway, store: Arc<Store>, fresh: Arc<Fresh>) -> Result<(), String> {
    let mut before: HashSet<String> = HashSet::new();
    loop {
        let client = Arc::clone(gateway.client());
        let read = gateway
            .call_auction(Priority::Background, |pass| async move {
                client.recent_auctions(pass).await
            })
            .await;
        match read {
            Ok(auctions) => {
                let ids: HashSet<String> =
                    auctions.iter().map(|auction| auction.id.clone()).collect();
                let overlap = ids.intersection(&before).count();
                let read = auctions.len();
                let unseen: Vec<Auction> = auctions
                    .iter()
                    .filter(|auction| !before.contains(&auction.id))
                    .cloned()
                    .collect();
                let new = store
                    .keep_auctions(auctions, seconds())
                    .await
                    .map_err(|e| e.to_string())?;
                if !before.is_empty() && overlap == 0 {
                    warn!(auctions = read, "auction feed gap, some auctions missed");
                } else {
                    debug!(auctions = read, new, overlap, "auction feed read");
                }
                fresh.add(unseen);
                before = ids;
            }
            Err(why) => warn!(error = %why, "auction feed read failed"),
        }
        sleep(EVERY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_auctions_wait_until_taken_and_the_oldest_give_way() {
        let fresh = Fresh::default();
        let auction = |id: usize| Auction {
            id: id.to_string(),
            starting_price: 100,
            buyout_price: Some(100),
            top_bid: None,
            is_direct_sell: true,
            visible: true,
            closed: false,
            created: String::new(),
            updated: String::new(),
            note: String::new(),
            owner: None,
            weapon: "torid".into(),
            name: "crita-visican".into(),
            mastery: 8,
            rerolls: 0,
            rank: 8,
            polarity: None,
            stats: Vec::new(),
        };
        fresh.add((0..MOST_WAITING + 2).map(auction).collect());

        let taken = fresh.take().await;

        assert_eq!(taken.len(), MOST_WAITING);
        assert_eq!(taken[0].id, "2");
    }
}
