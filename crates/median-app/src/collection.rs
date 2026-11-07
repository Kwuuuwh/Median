use std::collections::{BTreeMap, HashMap, HashSet};

use median_core::{Assembly, Judged, Stock, judge};
use median_ipc::{Branch, Collectible, Forge, Good, Readiness, Tree};

use crate::catalog::{self, Catalog, Entry};
use crate::market::{self, Store};
use crate::mastery::wire;
use crate::roadmap::spare;
use crate::state_sink::SharedHoldings;

/// Every collectible item with the account's owned count and mastery state, and whether it can
/// be built now, drops from the account's relics, and is allowed at the account's mastery rank.
pub async fn collectibles(
    catalog: &Catalog,
    holdings: &SharedHoldings,
    lang: &str,
) -> Result<Vec<Collectible>, String> {
    let held = holdings.read().inventory;
    let counted = holdings.counted();
    let keys: Vec<String> = counted.keys().cloned().collect();
    let asked = lang.to_string();
    let (entries, trees, rewarded) = catalog
        .read(move |connection| {
            let entries = catalog::entries(connection, &asked)?;
            let mut trees = HashMap::new();
            for entry in &entries {
                if let Some(tree) = catalog::assembly(connection, &entry.item)? {
                    trees.insert(entry.item.clone(), tree);
                }
            }
            let rewarded = catalog::rewards_of(connection, &keys)?;
            Ok((entries, trees, rewarded))
        })
        .await
        .map_err(|e| e.to_string())?;

    let xp: HashMap<&str, u64> = held
        .mastery
        .iter()
        .map(|levelled| (levelled.item.as_str(), levelled.xp))
        .collect();
    let rank = held.progression.rank;
    let credits = held.purse.credits;

    Ok(entries
        .into_iter()
        .map(|entry| {
            let tree = trees.get(&entry.item);
            let owned = counted.get(&entry.item).copied().unwrap_or(0);
            let mastered = entry.mastery.zip(entry.cap).map(|(mastery, cap)| {
                mastery.rank(cap, xp.get(entry.item.as_str()).copied().unwrap_or(0)) == cap
            });
            Collectible {
                ready: tree.map(|tree| owned == 0 && buildable(tree, &counted, credits)),
                relics: rewarded.contains(&entry.item)
                    || tree.is_some_and(|tree| hands_over(tree, &rewarded)),
                reachable: rank.map(|rank| entry.required <= rank),
                mastered,
                owned,
                ..collectible(entry)
            }
        })
        .collect())
}

/// Build tree of `item` checked against the account's spare items, with in-game prices; `None`
/// when no recipe builds it.
pub async fn tree(
    catalog: &Catalog,
    market_cache: &Store,
    holdings: &SharedHoldings,
    lang: &str,
    item: &str,
) -> Result<Option<Tree>, String> {
    let asked = item.to_string();
    let Some(assembly) = catalog
        .read(move |connection| catalog::assembly(connection, &asked))
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };

    let mut keys = Vec::new();
    steps(&assembly, &mut keys);
    let named = goods_of(catalog, lang, keys.clone()).await?;
    let prices: HashMap<String, Option<u32>> = market::quoted(catalog, market_cache, &keys)
        .await?
        .into_iter()
        .map(|quote| (quote.item, quote.ingame))
        .collect();
    let held = holdings.read().inventory;
    let mut left = spare(catalog, &held, &holdings.counted()).await?;
    let judged = judge(&assembly, &mut left);

    Ok(Some(Tree {
        credits: wire(judged.credits()),
        purse: held.purse.credits.map(wire),
        time: wire(judged.time()),
        root: branch(&judged, &named, &prices),
    }))
}

/// Catalog goods of `items` in `lang`, keyed by item; items the catalog lacks are absent.
pub(crate) async fn goods_of(
    catalog: &Catalog,
    lang: &str,
    items: Vec<String>,
) -> Result<HashMap<String, Good>, String> {
    let asked = lang.to_string();
    Ok(catalog
        .read(move |connection| catalog::goods(connection, &asked, &items))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|good| (good.item.clone(), good))
        .collect())
}

fn collectible(entry: Entry) -> Collectible {
    Collectible {
        item: entry.item,
        name: entry.name,
        image: entry.image,
        class: entry.class,
        kind: entry.kind,
        prime: entry.prime,
        vaulted: entry.vaulted,
        tradable: entry.tradable,
        required: entry.required,
        owned: 0,
        mastered: None,
        ready: None,
        reachable: None,
        relics: false,
    }
}

/// Whether the tree can be built from owned items and credits; unknown credits (`None`) do not
/// block it.
fn buildable(tree: &Assembly, counted: &HashMap<String, u32>, credits: Option<u64>) -> bool {
    let mut keys = Vec::new();
    steps(tree, &mut keys);
    let spare: BTreeMap<String, u32> = keys
        .into_iter()
        .filter_map(|key| counted.get(&key).map(|count| (key, *count)))
        .collect();
    let judged = judge(tree, &mut Stock::from(spare));
    judged.readiness == median_core::Readiness::Buildable
        && credits.is_none_or(|credits| credits >= judged.credits())
}

/// Whether any item or blueprint of the tree is in `rewarded`, the drops of the account's relics.
fn hands_over(tree: &Assembly, rewarded: &HashSet<String>) -> bool {
    let mut keys = Vec::new();
    steps(tree, &mut keys);
    keys.iter().any(|key| rewarded.contains(key))
}

/// Pushes every item and blueprint of the tree onto `keys`, top down.
fn steps(tree: &Assembly, keys: &mut Vec<String>) {
    keys.push(tree.item.clone());
    if let Some(craft) = &tree.craft {
        keys.push(craft.blueprint.clone());
        for part in &craft.parts {
            steps(part, keys);
        }
    }
}

fn branch(
    step: &Judged,
    named: &HashMap<String, Good>,
    prices: &HashMap<String, Option<u32>>,
) -> Branch {
    let good = named.get(&step.item);
    Branch {
        item: step.item.clone(),
        name: catalog::name_of(good, &step.item),
        image: good.and_then(|good| good.image.clone()),
        count: step.count,
        have: step.have,
        readiness: readiness(step.readiness),
        tradable: good.and_then(|good| good.tradable) == Some(true),
        price: prices.get(&step.item).copied().flatten(),
        craft: step.craft.as_ref().map(|craft| Forge {
            blueprint: craft.blueprint.clone(),
            name: catalog::name_of(named.get(&craft.blueprint), &craft.blueprint),
            image: named
                .get(&craft.blueprint)
                .and_then(|good| good.image.clone()),
            count: craft.count,
            have: craft.have,
            price: prices.get(&craft.blueprint).copied().flatten(),
            credits: wire(craft.credits),
            time: wire(craft.time),
            parts: craft
                .parts
                .iter()
                .map(|part| branch(part, named, prices))
                .collect(),
        }),
    }
}

fn readiness(readiness: median_core::Readiness) -> Readiness {
    match readiness {
        median_core::Readiness::Held => Readiness::Held,
        median_core::Readiness::Buildable => Readiness::Buildable,
        median_core::Readiness::Short => Readiness::Short,
    }
}

#[cfg(test)]
mod tests {
    use median_core::Craft;

    use super::*;

    fn found(item: &str, count: u32) -> Assembly {
        Assembly {
            item: item.into(),
            count,
            craft: None,
        }
    }

    fn volt() -> Assembly {
        Assembly {
            item: "VoltPrime".into(),
            count: 1,
            craft: Some(Craft {
                blueprint: "VoltPrimeBlueprint".into(),
                consumed: true,
                credits: 25_000,
                time: 259_200,
                output: 1,
                parts: vec![found("VoltPrimeSystems", 1), found("OrokinCell", 1)],
                bought: None,
            }),
        }
    }

    fn holding(items: &[(&str, u32)]) -> HashMap<String, u32> {
        items
            .iter()
            .map(|(item, count)| (item.to_string(), *count))
            .collect()
    }

    #[test]
    fn a_tree_whose_every_step_is_held_can_be_built_when_the_credits_are_there() {
        let counted = holding(&[
            ("VoltPrimeBlueprint", 1),
            ("VoltPrimeSystems", 1),
            ("OrokinCell", 1),
        ]);

        assert!(buildable(&volt(), &counted, Some(25_000)));
        assert!(!buildable(&volt(), &counted, Some(24_999)));
        assert!(buildable(&volt(), &counted, None));
    }

    #[test]
    fn one_step_missing_keeps_the_tree_from_being_built() {
        let counted = holding(&[("VoltPrimeBlueprint", 1), ("OrokinCell", 1)]);

        assert!(!buildable(&volt(), &counted, Some(1_000_000)));
    }

    #[test]
    fn relics_that_hand_over_a_step_of_the_tree_count_for_it() {
        let rewarded = HashSet::from(["VoltPrimeSystems".to_string()]);

        assert!(hands_over(&volt(), &rewarded));
        assert!(!hands_over(&volt(), &HashSet::new()));
    }
}
