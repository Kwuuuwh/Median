use std::collections::{BTreeSet, HashMap};

use median_core::{DAY, Instant, margins, seconds};
use median_ipc::{Dealt, Figures};

use crate::account;
use crate::catalog::{self, Catalog};

/// Slug suffix of a full set on warframe.market.
const SET: &str = "_set";

/// Group an exchange is counted under: `riven`, `relic`, `arcane` and `mod` keep their catalog
/// group; any other slug ending in `_set` is `set`, whatever group its parts belong to.
fn group(filed: Option<&String>, slug: &str) -> String {
    let filed = filed.map(String::as_str).unwrap_or_default();
    match (filed, slug.ends_with(SET)) {
        ("riven" | "relic" | "arcane" | "mod", _) => filed.to_owned(),
        (_, true) => "set".to_owned(),
        (filed, false) => filed.to_owned(),
    }
}

/// Exchanges of the last `days` days with their margins, newest first; `days == 0` returns all.
pub async fn figures(
    catalog: &Catalog,
    history: &account::Store,
    lang: &str,
    days: u32,
) -> Result<Figures, String> {
    let from = match days {
        0 => 0,
        days => seconds().saturating_sub(Instant::from(days).saturating_mul(DAY)),
    };
    let made = history
        .exchanges_since(0)
        .await
        .map_err(|e| e.to_string())?;
    let margins = margins(&made.iter().map(account::Exchange::deal).collect::<Vec<_>>());

    let shown: Vec<(&account::Exchange, Option<i64>)> = made
        .iter()
        .zip(margins)
        .filter(|(exchange, _)| exchange.at >= from)
        .collect();
    let slugs: BTreeSet<String> = shown
        .iter()
        .map(|(exchange, _)| exchange.slug.clone())
        .collect();
    let lang = lang.to_owned();
    let (named, filed): (HashMap<String, catalog::Named>, HashMap<String, String>) = catalog
        .read(move |connection| {
            let mut found = HashMap::new();
            for slug in slugs {
                if let Some(known) = catalog::named(connection, &lang, &slug)? {
                    found.insert(slug, known);
                }
            }
            let items: Vec<String> = found.values().map(|known| known.item.clone()).collect();
            let filed = catalog::grouped(connection, &items)?.into_iter().collect();
            Ok((found, filed))
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(Figures {
        deals: shown
            .into_iter()
            .map(|(exchange, margin)| {
                let known = named.get(&exchange.slug);
                Dealt {
                    at: exchange.at.max(0) as u32,
                    item: known.map_or_else(|| exchange.item.clone(), |known| known.item.clone()),
                    name: known.map_or_else(|| exchange.slug.clone(), |known| known.name.clone()),
                    image: known.and_then(|known| known.image.clone()),
                    selling: exchange.selling,
                    platinum: exchange.platinum,
                    quantity: exchange.quantity,
                    agent: exchange.agent,
                    group: group(
                        known.and_then(|known| filed.get(&known.item)),
                        &exchange.slug,
                    ),
                    margin: margin.map(|margin| {
                        margin.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
                    }),
                }
            })
            .collect(),
    })
}
