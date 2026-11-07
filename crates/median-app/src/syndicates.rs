use std::collections::HashMap;

use median_core::{Allegiance, Holder, Interest, Need, Policy, allegiances, seconds};
use median_ipc::{StandingOffer, Syndicate};

use crate::catalog::{self, Catalog, Stocked};
use crate::market::{self, Needs, Reading, Store, Variant};
use crate::state_sink::SharedHoldings;

/// Market variant key of what a vendor sells: rank 0 for kinds warframe.market trades by rank
/// (mods, arcanes, stances), the empty key for everything else.
fn sold_as(kind: &str) -> String {
    let ranked = kind == "mod" || kind == "arcane" || kind == "stance" || kind.ends_with("-mod");
    match ranked {
        true => Variant {
            rank: Some(0),
            ..Variant::default()
        }
        .key(),
        false => String::new(),
    }
}

/// Every syndicate with known vendors, with the account's standing and the vendors' offers
/// priced from the market cache. Tradable offers are declared as needs to the scheduler so
/// their prices stay current.
pub async fn syndicates(
    catalog: &Catalog,
    market_cache: &Store,
    holdings: &SharedHoldings,
    needs: &Needs,
    policy: &Policy,
    lang: &str,
) -> Result<Vec<Syndicate>, String> {
    let progression = holdings.read().inventory.progression.clone();
    let held = holdings.counted();
    let found = allegiances(&progression.affiliations, &progression.allowances);

    let vendors: Vec<&'static str> = found
        .iter()
        .flat_map(|allegiance| allegiance.vendors.iter().copied())
        .collect();
    let lang_owned = lang.to_owned();
    let stocked: Vec<Stocked> = catalog
        .read(move |connection| catalog::stocked(connection, &lang_owned, &vendors))
        .await
        .map_err(|e| e.to_string())?;

    let traded = |offer: &Stocked| {
        offer
            .slug
            .clone()
            .filter(|_| offer.tradable)
            .map(|slug| (slug, sold_as(&offer.kind)))
    };
    let mut pairs: Vec<(String, String)> = stocked.iter().filter_map(traded).collect();
    pairs.sort();
    pairs.dedup();
    let recent = market_cache
        .recent(pairs.clone())
        .await
        .map_err(|e| e.to_string())?;
    let now = seconds();
    let reading = |offer: &Stocked| -> Option<Reading> {
        traded(offer).map(|pair| market::reading_of(recent.get(&pair), now))
    };

    let sale_days = policy.sale_days;
    needs.declare(
        Holder::Syndicates,
        stocked
            .iter()
            .filter_map(|offer| {
                let (slug, variant) = traded(offer)?;
                Some(Need {
                    slug,
                    variant,
                    interest: Interest::Offered,
                })
            })
            .collect(),
    );

    Ok(found
        .iter()
        .map(|allegiance| Syndicate {
            tag: allegiance.tag.to_owned(),
            met: allegiance.met,
            standing: allegiance
                .standing
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            title: allegiance.title,
            left_today: allegiance.left_today,
            offers: offers_of(allegiance, &stocked, &reading, &held, sale_days),
        })
        .collect())
}

/// Offers of one syndicate's vendors as IPC rows, priced from `reading`, with owned counts.
fn offers_of(
    allegiance: &Allegiance,
    stocked: &[Stocked],
    reading: &dyn Fn(&Stocked) -> Option<Reading>,
    held: &HashMap<String, u32>,
    sale_days: u32,
) -> Vec<StandingOffer> {
    stocked
        .iter()
        .filter(|offer| allegiance.vendors.contains(&offer.vendor.as_str()))
        .map(|offer| {
            let read = reading(offer);
            let prospect = read.map(|read| read.prospect);
            StandingOffer {
                item: offer.item.clone(),
                slug: offer.slug.clone(),
                name: offer.name.clone(),
                image: offer.image.clone(),
                vendor: offer.vendor_name.clone(),
                place: offer.place.clone(),
                cost: offer.cost,
                rank: offer.rank,
                tradable: offer.tradable,
                platinum: prospect.and_then(|prospect| prospect.worth(sale_days)),
                ask: read.and_then(|read| read.ingame),
                per_day: prospect
                    .filter(|prospect| prospect.per_day > 0.0)
                    .map(|prospect| prospect.per_day as f32),
                days: prospect
                    .and_then(|prospect| prospect.days())
                    .map(|days| days as f32),
                owned: held.get(&offer.item).copied().unwrap_or(0),
            }
        })
        .collect()
}
