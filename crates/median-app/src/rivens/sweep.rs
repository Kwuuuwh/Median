use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use median_core::Detail;
use tokio::time::sleep;
use tracing::{info, warn};

use super::asks::Slices;
use crate::catalog::{self, Catalog};
use crate::market::Gateway;
use crate::state_sink::SharedHoldings;

/// Interval between two sweeps of the followed weapons' auctions.
const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// How long a weapon stays followed after the user viewed its rivens.
const LOOKED: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Delay after start before the first sweep, so the window's own requests go first.
const SETTLE: Duration = Duration::from_secs(5 * 60);

/// Pause between two weapons in a sweep: back-to-back searches make warframe.market refuse
/// every request.
const APART: Duration = Duration::from_secs(2 * 60);

/// Every `EVERY`, re-reads the auctions of the weapons of owned rivens and of recently viewed
/// weapons, so that closed auctions are detected.
pub async fn tend(
    catalog: Arc<Catalog>,
    gateway: Gateway,
    slices: Arc<Slices>,
    holdings: SharedHoldings,
) -> Result<(), String> {
    sleep(SETTLE).await;
    loop {
        let mut weapons: BTreeSet<String> = slices.looked_within(LOOKED).into_iter().collect();
        weapons.extend(held_weapons(&catalog, &holdings).await?);
        for weapon in &weapons {
            match slices.sweep(&gateway, weapon).await {
                Ok(found) => info!(weapon, auctions = found, "riven auctions searched"),
                Err(why) => warn!(error = %why, weapon, "riven auction search failed"),
            }
            sleep(APART).await;
        }
        sleep(EVERY).await;
    }
}

/// warframe.market slugs of the weapons of the account's revealed rivens.
async fn held_weapons(
    catalog: &Catalog,
    holdings: &SharedHoldings,
) -> Result<BTreeSet<String>, String> {
    let weapons: BTreeSet<String> = holdings
        .read()
        .inventory
        .holdings
        .iter()
        .filter_map(|holding| match &holding.detail {
            Some(Detail::Riven {
                revealed: Some(riven),
                ..
            }) => Some(riven.weapon.clone()),
            _ => None,
        })
        .collect();
    catalog
        .read(move |connection| {
            let mut slugs = BTreeSet::new();
            for weapon in &weapons {
                if let Some(slug) = catalog::armed(connection, weapon)?.and_then(|armed| armed.slug)
                {
                    slugs.insert(slug);
                }
            }
            Ok(slugs)
        })
        .await
        .map_err(|e| e.to_string())
}
