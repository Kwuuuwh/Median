use std::collections::HashMap;

use median_core::{Holding, Inventory, Outcome, SwapKind, Variant, fold};
use median_ipc::{Haul, HaulLine};

use crate::account;
use crate::catalog::{self, Catalog};
use crate::collection::goods_of;
use crate::market::{self, Store};
use crate::state_sink::SharedHoldings;

/// Prime parts gained since the haul started, named and priced as in the inventory; copies that
/// were bought, received in in-game trades or built from claimed blueprints are not counted.
/// Empty when there is no profile or the haul has no start.
pub async fn haul(
    catalog: &Catalog,
    market_cache: &Store,
    holdings: &SharedHoldings,
    history: &account::Store,
    lang: &str,
) -> Result<Haul, String> {
    let Some(profile) = holdings.read().inventory.profile.clone() else {
        return Ok(Haul::default());
    };
    let haul = history.haul(profile).await.map_err(|e| e.to_string())?;
    let Some(started) = haul.started else {
        return Ok(Haul::default());
    };
    let mut bought: HashMap<String, u32> = HashMap::new();
    for exchange in history
        .exchanges_since(started)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|exchange| !exchange.selling)
    {
        *bought.entry(exchange.item).or_default() += exchange.quantity;
    }
    for (item, count) in swapped(catalog, history, started, lang).await? {
        *bought.entry(item).or_default() += count;
    }

    let blueprints: Vec<String> = haul.claimed.keys().cloned().collect();
    let built = catalog
        .read(move |connection| catalog::builds(connection, &blueprints))
        .await
        .map_err(|e| e.to_string())?;
    let mut forged: HashMap<String, u32> = HashMap::new();
    for (blueprint, count) in &haul.claimed {
        if let Some(made) = built.get(blueprint) {
            *forged.entry(made.item.clone()).or_default() += count;
        }
    }

    let gained: Vec<Holding> = haul
        .gained
        .into_iter()
        .filter_map(|((item, variant), count)| {
            let paid = taken(&mut bought, &item, count);
            let made = taken(&mut forged, &item, count - paid);
            let count = count - paid - made;
            (count > 0).then(|| Holding {
                detail: median_engine::detail(&variant),
                item,
                variant,
                count,
            })
        })
        .collect();
    let lines = lines(catalog, market_cache, lang, gained).await?;
    Ok(Haul {
        started: Some(started.max(0) as u32),
        lines,
    })
}

/// Takes up to `count` copies of `item` off `tally` and returns how many were taken (0 when the
/// item is absent).
fn taken(tally: &mut HashMap<String, u32>, item: &str, count: u32) -> u32 {
    tally.get_mut(item).map_or(0, |left| {
        let taken = (*left).min(count);
        *left -= taken;
        taken
    })
}

/// Count per catalog item received in completed in-game trades since `from`; names the catalog
/// cannot match are skipped.
async fn swapped(
    catalog: &Catalog,
    history: &account::Store,
    from: median_core::Instant,
    lang: &str,
) -> Result<HashMap<String, u32>, String> {
    let taken: Vec<(String, u32)> = history
        .swaps_since(from)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|swap| swap.outcome == Outcome::Done)
        .flat_map(|swap| swap.taken)
        .filter(|part| part.kind == SwapKind::Item)
        .map(|part| (fold(&part.name), part.count))
        .collect();
    let known: HashMap<String, String> =
        catalog::told_by_fold(catalog, lang, taken.iter().map(|(name, _)| name.clone()))
            .await?
            .into_values()
            .filter(|told| !told.item.is_empty())
            .map(|told| (told.fold, told.item))
            .collect();
    let mut swapped: HashMap<String, u32> = HashMap::new();
    for (name, count) in taken {
        if let Some(item) = known.get(&name) {
            *swapped.entry(item.clone()).or_default() += count;
        }
    }
    Ok(swapped)
}

/// Haul rows for the gained items that are prime parts with a ducat value, named by the catalog
/// and priced from the market cache.
async fn lines(
    catalog: &Catalog,
    market_cache: &Store,
    lang: &str,
    gained: Vec<Holding>,
) -> Result<Vec<HaulLine>, String> {
    let named = goods_of(
        catalog,
        lang,
        gained.iter().map(|held| held.item.clone()).collect(),
    )
    .await?;
    let gained: Vec<Holding> = gained
        .into_iter()
        .filter(|held| {
            named
                .get(&held.item)
                .is_some_and(|good| good.prime && good.ducats.is_some())
        })
        .collect();
    let gained = Inventory {
        holdings: gained,
        ..Inventory::default()
    };
    let worth: HashMap<(String, String), Option<u32>> =
        market::prices(catalog, market_cache, &gained)
            .await?
            .into_iter()
            .map(|priced| ((priced.item, priced.variant), priced.worth))
            .collect();
    Ok(gained
        .holdings
        .into_iter()
        .map(|held| {
            let good = named.get(&held.item);
            let Variant(variant) = held.variant;
            HaulLine {
                name: catalog::name_of(good, &held.item),
                image: good.and_then(|good| good.image.clone()),
                known: good.is_some_and(|good| good.known),
                ducats: good.and_then(|good| good.ducats),
                worth: worth
                    .get(&(held.item.clone(), variant.clone()))
                    .copied()
                    .flatten(),
                item: held.item,
                variant,
                count: held.count,
            }
        })
        .collect())
}

/// Restarts the current profile's haul now, with the last inventory read as its baseline; does
/// nothing without a profile.
pub async fn restart(holdings: &SharedHoldings, history: &account::Store) -> Result<(), String> {
    let Some(profile) = holdings.read().inventory.profile.clone() else {
        return Ok(());
    };
    history
        .restart_haul(profile, median_core::seconds())
        .await
        .map_err(|e| e.to_string())
}
