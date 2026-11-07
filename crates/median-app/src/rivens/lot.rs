use median_core::{ListedStat, LotView, RivenClass, entered, odds};
use median_ipc::{Polarity, Riven, RivenStat};
use median_market::Auction;

use super::appraisal::to_core;
use super::asks::{Armory, Slices, pool_stats};
use super::choices::{REROLL_SAMPLES, REROLL_SEED, priced};
use super::demand::weapon_demand;
use super::trades::Weekly;
use crate::catalog::{self, Catalog};
use crate::market::Gateway;

/// Market view of an auction's riven: the middle price of similar rivens, the demand, and for
/// a riven never rerolled the odds that a reroll gives a pricier riven. `None` without a buyout
/// price, or when the weapon or a stat cannot be matched to the catalog.
pub async fn lot_view(
    catalog: &Catalog,
    gateway: &Gateway,
    slices: &Slices,
    weekly: &Weekly,
    auction: &Auction,
) -> Result<Option<(median_core::Riven, LotView)>, String> {
    let Some(buyout) = auction.buyout_price else {
        return Ok(None);
    };
    let Some(weapon) = weapon_of(catalog, auction).await? else {
        return Ok(None);
    };
    let Some(wire) = riven_of(catalog, weapon.clone(), auction).await? else {
        return Ok(None);
    };
    let demand =
        weapon_demand(catalog, slices, weekly, weapon.clone(), auction.rerolls > 0).await?;
    let Some(armory) = Armory::of(catalog, "en", weapon).await? else {
        return Ok(None);
    };
    let Some((asks, _)) = armory.asks(gateway, slices, &wire).await? else {
        return Ok(None);
    };
    let held_bounds = armory.bounds(weekly, auction.rerolls > 0);
    let rolled_bounds = armory.bounds(weekly, true);

    tokio::task::spawn_blocking(move || {
        let bases = pool_stats(&armory.pool);
        let riven = to_core(&wire);
        let fair = asks.band(&riven, held_bounds).map(|band| band.middle);
        let rolled = match (riven.rerolls, fair) {
            (0, Some(fair)) => odds(
                &riven,
                &bases,
                REROLL_SAMPLES,
                REROLL_SEED,
                fair,
                priced(|rolled| asks.band(rolled, rolled_bounds).map(|band| band.middle)),
            ),
            _ => Vec::new(),
        };
        let view = LotView {
            buyout: f64::from(buyout),
            fair,
            rerolls: riven.rerolls,
            odds: rolled,
            demand,
        };
        Ok(Some((riven, view)))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// English riven family name of an auction's weapon; `None` when its class is not a riven
/// class median prices.
pub async fn family_of(catalog: &Catalog, auction: &Auction) -> Result<Option<String>, String> {
    let slug = auction.weapon.clone();
    let found = catalog
        .read(move |connection| catalog::slug_family(connection, &slug))
        .await
        .map_err(|e| e.to_string())?;
    Ok(found
        .filter(|(_, class)| RivenClass::from_name(class).is_some())
        .map(|(family, _)| family))
}

/// Game key of the weapon an auction's riven is for; `None` when the slug is not in the catalog.
pub(super) async fn weapon_of(
    catalog: &Catalog,
    auction: &Auction,
) -> Result<Option<String>, String> {
    let slug = auction.weapon.clone();
    catalog
        .read(move |connection| catalog::slug_weapon(connection, &slug))
        .await
        .map_err(|e| e.to_string())
}

/// The auctioned riven with its rolls computed back from the shown stat values; `None` when a
/// stat is not in the weapon's pool or the values fit no roll.
pub(super) async fn riven_of(
    catalog: &Catalog,
    weapon: String,
    auction: &Auction,
) -> Result<Option<Riven>, String> {
    let Some(armory) = Armory::of(catalog, "en", weapon.clone()).await? else {
        return Ok(None);
    };
    let Some(stats) = auction
        .stats
        .iter()
        .map(|stat| {
            let pooled = armory
                .pool
                .iter()
                .find(|pooled| pooled.slug.as_deref() == Some(stat.slug.as_str()))?;
            Some(ListedStat {
                tag: pooled.tag.clone(),
                shown: stat.value,
                positive: stat.positive,
            })
        })
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(None);
    };
    let bases = pool_stats(&armory.pool);
    let rank = u8::try_from(auction.rank).unwrap_or(u8::MAX);
    let Some(read) = entered(&stats, &bases, armory.armed.disposition, rank) else {
        return Ok(None);
    };
    let wire = |stat: &median_core::RivenStat| RivenStat {
        tag: stat.tag.clone(),
        roll: stat.roll,
    };
    Ok(Some(Riven {
        weapon,
        mastery: auction.mastery,
        rerolls: auction.rerolls,
        polarity: auction.polarity.as_deref().and_then(polarity_of),
        buffs: read.buffs.iter().map(wire).collect(),
        curses: read.curses.iter().map(wire).collect(),
    }))
}

fn polarity_of(word: &str) -> Option<Polarity> {
    match word {
        "madurai" => Some(Polarity::Madurai),
        "vazarin" => Some(Polarity::Vazarin),
        "naramon" => Some(Polarity::Naramon),
        _ => None,
    }
}
