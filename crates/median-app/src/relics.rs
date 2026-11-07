use std::collections::{BTreeMap, HashMap};

use median_core::{Prize, payback};
use median_ipc::{Odds, Relic, Slot, Step};

use crate::catalog::{self, Catalog};
use crate::market::{self, Store};
use crate::state_sink::SharedHoldings;

/// Refinements in display order, with the Void Traces (`echoes`) each costs to refine an intact
/// relic to.
const STEPS: [(&str, u32); 4] = [
    ("intact", 0),
    ("exceptional", 25),
    ("flawless", 50),
    ("radiant", 100),
];

/// Relics the account owns, with the expected value of each refinement for a squad of `squad`
/// players who all open the same relic. Refinements the account owns none of are included, so
/// the page can show what refining would be worth.
pub async fn held(
    catalog: &Catalog,
    market_cache: &Store,
    holdings: &SharedHoldings,
    lang: &str,
    squad: u32,
) -> Result<Vec<Relic>, String> {
    let counts = holdings.counted();
    let mut keys: Vec<String> = counts.keys().cloned().collect();
    keys.sort();
    priced(catalog, market_cache, &counts, keys, lang, squad).await
}

/// Relics of `keys`, with the expected value of each refinement for a squad of `squad`;
/// `counts` gives how many of each refinement the account owns. Every reward is valued at the
/// median price of its closed trades.
pub async fn priced(
    catalog: &Catalog,
    market_cache: &Store,
    counts: &HashMap<String, u32>,
    keys: Vec<String>,
    lang: &str,
    squad: u32,
) -> Result<Vec<Relic>, String> {
    let tongue = lang.to_owned();
    let (family, tables) = catalog
        .read(move |connection| {
            let bases = catalog::bases(connection, &keys)?;
            let family = catalog::family(connection, &tongue, &bases)?;
            let grades: Vec<String> = family.iter().map(|grade| grade.item.clone()).collect();
            Ok((family, catalog::slots(connection, &tongue, &grades)?))
        })
        .await
        .map_err(|e| e.to_string())?;

    let prices = quoted(catalog, market_cache, &tables).await?;
    Ok(assemble(family, tables, &prices, counts, squad))
}

/// Median closed-trade price of every item the relics can drop; items without closed trades in
/// the local market cache are omitted.
async fn quoted(
    catalog: &Catalog,
    market_cache: &Store,
    tables: &[catalog::Slot],
) -> Result<HashMap<String, u32>, String> {
    let mut items: Vec<String> = tables.iter().map(|slot| slot.item.clone()).collect();
    items.sort();
    items.dedup();

    Ok(market::quoted(catalog, market_cache, &items)
        .await?
        .into_iter()
        .filter_map(|quote| Some((quote.item, quote.platinum?)))
        .collect())
}

fn assemble(
    family: Vec<catalog::Grade>,
    tables: Vec<catalog::Slot>,
    prices: &HashMap<String, u32>,
    counts: &HashMap<String, u32>,
    squad: u32,
) -> Vec<Relic> {
    let mut drops: HashMap<String, Vec<catalog::Slot>> = HashMap::new();
    for slot in tables {
        drops.entry(slot.relic.clone()).or_default().push(slot);
    }

    let mut families: BTreeMap<String, Vec<catalog::Grade>> = BTreeMap::new();
    for grade in family {
        families.entry(grade.base.clone()).or_default().push(grade);
    }

    let mut relics: Vec<Relic> = families
        .into_values()
        .filter_map(|grades| one(&grades, &drops, prices, counts, squad))
        .collect();
    relics.sort_by(|one, other| one.name.cmp(&other.name));
    relics
}

/// Builds one relic: its refinements in order, its drops and the expected value of every
/// refinement; `None` when the catalog has no drop table for it.
fn one(
    grades: &[catalog::Grade],
    drops: &HashMap<String, Vec<catalog::Slot>>,
    prices: &HashMap<String, u32>,
    counts: &HashMap<String, u32>,
    squad: u32,
) -> Option<Relic> {
    let ordered: Vec<&catalog::Grade> = STEPS
        .iter()
        .filter_map(|(refinement, _)| grades.iter().find(|grade| grade.refinement == *refinement))
        .collect();
    let named = *ordered.first()?;

    let mut holds = drops.get(&named.item).cloned().unwrap_or_default();
    holds.sort_by(|one, other| {
        rank(&one.rarity)
            .cmp(&rank(&other.rarity))
            .then_with(|| one.name.cmp(&other.name))
    });
    if holds.is_empty() {
        return None;
    }

    let mut steps = Vec::with_capacity(ordered.len());
    let mut odds: Vec<Vec<Odds>> = vec![Vec::new(); holds.len()];

    for (grade, (_, echoes)) in ordered.iter().zip(STEPS) {
        let table = drops.get(&grade.item);
        let prizes: Vec<Prize> = holds
            .iter()
            .map(|held| {
                let row = table.and_then(|rows| rows.iter().find(|row| row.item == held.item));
                Prize {
                    chance: row.map_or(0, |row| row.chance),
                    count: row.map_or(held.count, |row| row.count),
                    platinum: prices.get(&held.item).map(|&plat| f64::from(plat)),
                    ducats: held.ducats,
                }
            })
            .collect();

        let paid = payback(&prizes, squad);
        for (at, taken) in paid.taken.iter().enumerate() {
            odds[at].push(Odds {
                chance: prizes[at].chance,
                platinum: median_ipc::permille(taken.platinum),
                ducats: median_ipc::permille(taken.ducats),
            });
        }

        steps.push(Step {
            item: grade.item.clone(),
            refinement: grade.refinement.clone(),
            count: counts.get(&grade.item).copied().unwrap_or(0),
            platinum: median_ipc::tenths(paid.platinum),
            ducats: median_ipc::tenths(paid.ducats),
            echoes,
        });
    }

    Some(Relic {
        base: named.base.clone(),
        name: named.name.clone(),
        image: named.image.clone(),
        era: named.era.clone(),
        vaulted: named.vaulted,
        steps,
        holds: holds
            .into_iter()
            .zip(odds)
            .map(|(held, odds)| Slot {
                platinum: prices.get(&held.item).copied(),
                own: counts.get(&held.item).copied().unwrap_or(0),
                item: held.item,
                name: held.name,
                image: held.image,
                rarity: held.rarity,
                count: held.count,
                ducats: held.ducats,
                odds,
            })
            .collect(),
    })
}

/// Sort key of a rarity in the drop table: common, uncommon, then everything else (rare).
fn rank(rarity: &str) -> u8 {
    match rarity {
        "common" => 0,
        "uncommon" => 1,
        _ => 2,
    }
}
