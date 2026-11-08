use median_core::{DESCRIBED_FOR, seconds};

use crate::market::gateway::{Gateway, Priority};
use crate::market::store::{ITEM, Store};

/// Whether an order on this item may set a lot size (`bulk_tradable`); read from the market cache,
/// or fetched from the market once when the cache has no value.
pub(crate) async fn lots(
    gateway: &Gateway,
    store: &Store,
    priority: Priority,
    slug: &str,
) -> Result<bool, String> {
    if let Some(known) = store.lots_allowed(slug).await.map_err(|e| e.to_string())? {
        return Ok(known);
    }

    let meta = gateway
        .call(priority, |pass| gateway.client().item(pass, slug))
        .await
        .map_err(|e| format!("the market did not describe {slug}: {e}"))?;
    let allowed = meta.bulk_tradable.unwrap_or(false);
    store.keep_item(&meta).await.map_err(|e| e.to_string())?;
    store.stamp(slug, ITEM).await.map_err(|e| e.to_string())?;
    Ok(allowed)
}

/// Whether the market cache's description of the item is younger than `DESCRIBED_FOR`.
pub(crate) async fn is_described(store: &Store, slug: &str) -> bool {
    store
        .stamped(slug, ITEM)
        .await
        .ok()
        .flatten()
        .is_some_and(|at| seconds() - at < DESCRIBED_FOR)
}

/// Fetches the item description from the market and saves it, unless the cached one is younger
/// than `DESCRIBED_FOR`.
pub(crate) async fn describe(
    gateway: &Gateway,
    store: &Store,
    priority: Priority,
    slug: &str,
) -> Result<(), String> {
    if is_described(store, slug).await {
        return Ok(());
    }
    let meta = gateway
        .call(priority, |pass| gateway.client().item(pass, slug))
        .await
        .map_err(|e| format!("the market did not describe {slug}: {e}"))?;
    store.keep_item(&meta).await.map_err(|e| e.to_string())?;
    store.stamp(slug, ITEM).await.map_err(|e| e.to_string())
}
