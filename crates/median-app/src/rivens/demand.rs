use std::time::Duration;

use median_core::Demand;
use median_ipc::RivenDemand;

use super::asks::Slices;
use super::trades::Weekly;
use crate::catalog::{self, Catalog};

/// Age within which a seen auction still counts as listed; the sweeps read each followed weapon
/// every six hours.
const LISTED_WITHIN: Duration = Duration::from_secs(24 * 60 * 60);

/// Demand for rivens of `weapon`, rerolled or not, from weekly trade popularity and current
/// auctions; `None` before the weekly trades are loaded or when the weapon's riven family is
/// unknown.
pub async fn demand(
    catalog: &Catalog,
    slices: &Slices,
    weekly: &Weekly,
    weapon: String,
    rerolled: bool,
) -> Result<Option<RivenDemand>, String> {
    Ok(weapon_demand(catalog, slices, weekly, weapon, rerolled)
        .await?
        .map(to_demand))
}

/// Same as `demand`, as the core type.
pub(super) async fn weapon_demand(
    catalog: &Catalog,
    slices: &Slices,
    weekly: &Weekly,
    weapon: String,
    rerolled: bool,
) -> Result<Option<Demand>, String> {
    if !weekly.is_read() {
        return Ok(None);
    }
    let armed = catalog
        .read(move |connection| catalog::armed(connection, &weapon))
        .await
        .map_err(|e| e.to_string())?;
    let Some((family, slug)) = armed.and_then(|armed| Some((armed.family?, armed.slug))) else {
        return Ok(None);
    };
    let lots = match slug {
        Some(slug) => slices.listed(&slug, LISTED_WITHIN).await?,
        None => None,
    };
    let popularity = weekly.popularity(&family, rerolled);
    Ok(Some(median_core::demand(popularity, lots)))
}

fn to_demand(demand: Demand) -> RivenDemand {
    match demand {
        Demand::Lively => RivenDemand::Lively,
        Demand::Slow => RivenDemand::Slow,
        Demand::Dead => RivenDemand::Dead,
        Demand::Unclear => RivenDemand::Unclear,
    }
}
