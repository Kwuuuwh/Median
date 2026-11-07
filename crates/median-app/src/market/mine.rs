use std::sync::Arc;
use std::time::Duration;

use median_core::{Event, Holder, Interest, MarketEvent, MembershipEvent, Need};
use median_engine::Input;
use median_market::{MarketError, Mine, Session};
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::account;
use crate::market::gateway::{Gateway, Priority};
use crate::market::needs::Needs;
use crate::market::store::Store;
use crate::market::token::Token;
use crate::market::variant::Variant;

/// Interval between reads of the account's own orders from the market.
const EVERY: Duration = Duration::from_secs(60);

/// Reads the account's own orders from the market and saves all of them to the account store;
/// returns the number of orders. An order the market no longer returns is marked gone, not
/// deleted.
pub async fn read(
    gateway: &Gateway,
    priority: Priority,
    market_cache: &Store,
    history: &account::Store,
    needs: &Needs,
    tx: &Sender<Input>,
    authorization: String,
) -> Result<usize, MarketError> {
    let session = Session::resume(Arc::clone(gateway.client()), authorization, None);
    let mine = gateway
        .call(priority, |pass| session.my_orders(pass))
        .await?;

    let count = mine.len();
    let kept = answered(market_cache, mine).await;
    needs.declare(Holder::Orders, wanted(&kept));
    if let Err(why) = history.keep_all_orders(kept).await {
        return Err(MarketError::Malformed(why.to_string()));
    }

    let _ = tx
        .send(Input::Command(Event::Market(MarketEvent::OrdersRead)))
        .await;
    Ok(count)
}

/// Pairs with an open order of the account, as `Committed` needs.
fn wanted(kept: &[account::Answered]) -> Vec<Need> {
    let mut pairs: Vec<(String, String)> = kept
        .iter()
        .filter_map(|(_, slug, variant)| slug.clone().map(|slug| (slug, variant.clone())))
        .collect();
    pairs.sort();
    pairs.dedup();

    pairs
        .into_iter()
        .map(|(slug, variant)| Need {
            slug,
            variant,
            interest: Interest::Committed,
        })
        .collect()
}

/// Each order with its slug from the market cache (`None` when unknown) and its variant key.
async fn answered(market_cache: &Store, mine: Vec<Mine>) -> Vec<account::Answered> {
    let mut kept = Vec::with_capacity(mine.len());
    for order in mine {
        let slug = match &order.item_id {
            Some(id) => market_cache.slug_of(id.clone()).await.ok().flatten(),
            None => None,
        };
        let variant = Variant::of_mine(&order).key();
        kept.push((order, slug, variant));
    }
    kept
}

/// Records a completed trade on the market at its trade price and updates the account store.
pub async fn close(
    gateway: &Gateway,
    priority: Priority,
    history: &account::Store,
    order: &account::Standing,
    quantity: u32,
    platinum: u32,
) -> Result<(), MarketError> {
    let authorization = Token::read().ok_or(MarketError::Unauthorized)?;
    let session = Session::resume(Arc::clone(gateway.client()), authorization, None);

    // A trade at another price reprices the order for the close and restores it afterwards.
    let repriced = platinum > 0 && platinum != order.platinum;
    if repriced {
        let change = serde_json::json!({ "platinum": platinum });
        gateway
            .call(priority, |pass| session.amend(pass, &order.id, change))
            .await?;
    }

    let closed = gateway
        .call(priority, |pass| session.close(pass, &order.id, quantity))
        .await;
    let left = order.quantity.saturating_sub(quantity);
    if repriced && (closed.is_err() || left > 0) {
        let back = serde_json::json!({ "platinum": order.platinum });
        let restored = gateway
            .call(priority, |pass| session.amend(pass, &order.id, back))
            .await;
        if let Err(why) = restored {
            warn!(error = %why, order_id = order.id, "order price not restored after trade");
        }
    }
    closed?;

    history
        .shrink_order(order.id.clone(), left)
        .await
        .map_err(|why| MarketError::Malformed(why.to_string()))
}

/// Withdraws one of the account's orders on the market and in the account store.
pub async fn withdraw(
    gateway: &Gateway,
    priority: Priority,
    history: &account::Store,
    id: &str,
) -> Result<(), MarketError> {
    let authorization = Token::read().ok_or(MarketError::Unauthorized)?;
    let session = Session::resume(Arc::clone(gateway.client()), authorization, None);
    gateway
        .call(priority, |pass| session.withdraw(pass, id))
        .await?;
    history
        .withdraw_order(id.to_owned())
        .await
        .map_err(|why| MarketError::Malformed(why.to_string()))
}

/// Reads the account's own orders from the market every `EVERY` while a token is stored; a
/// refused token is deleted.
pub async fn tend(
    gateway: Gateway,
    market_cache: Arc<Store>,
    history: Arc<account::Store>,
    needs: Needs,
    tx: Sender<Input>,
) -> Result<(), String> {
    loop {
        if let Some(authorization) = Token::read() {
            let read = read(
                &gateway,
                Priority::Commit,
                &market_cache,
                &history,
                &needs,
                &tx,
                authorization,
            )
            .await;
            match read {
                Ok(count) => debug!(orders = count, "own orders read"),
                Err(MarketError::Unauthorized) => {
                    Token::clear();
                    let _ = tx
                        .send(Input::Command(Event::Membership(MembershipEvent::Refused)))
                        .await;
                }
                Err(why) => warn!(error = %why, "own orders read failed"),
            }
        }
        sleep(EVERY).await;
    }
}
