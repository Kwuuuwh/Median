use std::collections::{HashMap, HashSet};

use median_core::{
    Breakdown, Inventory, Levelled, Outlook, Reached, Share, Stock, breakdown, outlook, seconds,
    threshold,
};
use median_ipc::{
    Instant, Lack, Masterable, MasteryRank, MasteryShare, RankTest, Stage, Uncounted,
};

use crate::catalog::{self, Catalog, Recipe};
use crate::roadmap::spare;
use crate::state_sink::SharedHoldings;
use crate::way::{self, MasteryCache};

/// The account's mastery rank, its mastery points by source and progress to the next rank,
/// and every masterable item with its stage and missing inputs.
pub async fn standing(
    catalog: &Catalog,
    cache: &MasteryCache,
    holdings: &SharedHoldings,
    lang: &str,
) -> Result<MasteryRank, String> {
    let masterables = cache.masterables(catalog, lang).await?;
    let listed = &masterables.listed;
    let held = holdings.read().inventory;
    let levelled = levelled(&held, listed);
    let counted = breakdown_of(catalog, &held, listed, &levelled).await?;
    let progression = &held.progression;
    let standing = progression
        .rank
        .map(|rank| outlook(rank, &counted, progression.next_test, seconds()));
    let left = spare(catalog, &held, &holdings.counted()).await?;

    let owned: HashSet<&str> = held
        .holdings
        .iter()
        .map(|holding| holding.item.as_str())
        .collect();
    let acquired: HashMap<&str, median_core::Instant> = progression
        .acquired
        .iter()
        .map(|acquired| (acquired.item.as_str(), acquired.at))
        .collect();
    let forged = forged(catalog, &held).await?;
    let in_hand: u64 = listed
        .iter()
        .zip(&levelled)
        .filter(|(_, levelled)| !levelled.is_mastered())
        .filter(|(item, _)| {
            owned.contains(item.item.as_str()) || forged.contains(item.item.as_str())
        })
        .map(|(_, levelled)| levelled.reachable())
        .sum();
    let stages: Vec<Stage> = listed
        .iter()
        .zip(&levelled)
        .map(|(item, levelled)| {
            let built = owned.contains(item.item.as_str())
                || forged.contains(&item.item)
                || left.forging(&item.item) > 0;
            stage(levelled, built, masterables.recipes.get(&item.item), &left)
        })
        .collect();
    let shortfalls: Vec<Vec<(String, u32)>> = listed
        .iter()
        .zip(&stages)
        .map(|(item, stage)| match masterables.recipes.get(&item.item) {
            Some(recipe) if *stage == Stage::Short => shortfall(recipe, &left),
            _ => Vec::new(),
        })
        .collect();
    let goods = input_goods(catalog, lang, &shortfalls).await?;
    let masterable = listed
        .iter()
        .zip(&levelled)
        .zip(stages.into_iter().zip(shortfalls))
        .map(|((item, levelled), (stage, shortfall))| {
            let recipe = masterables.recipes.get(&item.item);
            let (gathered, inputs) = recipe.map_or((0, 0), |recipe| gathered(recipe, &left));
            Masterable {
                item: item.item.clone(),
                name: item.name.clone(),
                image: item.image.clone(),
                class: item.class.clone(),
                kind: item.kind.clone(),
                rank: levelled.rank(),
                cap: item.cap,
                remaining: wire(levelled.remaining()),
                forma: levelled.forma(),
                passes: levelled.passes(),
                required: item.required,
                owned: owned.contains(item.item.as_str()),
                prime: item.prime,
                vaulted: item.vaulted,
                acquired: acquired.get(item.item.as_str()).map(|&at| moment(at)),
                gathered,
                inputs,
                lacking: shortfall
                    .into_iter()
                    .map(|(input, short)| {
                        let good = goods.get(&input);
                        Lack {
                            name: catalog::name_of(good, &input),
                            image: good.and_then(|good| good.image.clone()),
                            item: input,
                            short,
                        }
                    })
                    .collect(),
                stage,
                way: masterables
                    .ways
                    .get(&item.item)
                    .and_then(Option::as_ref)
                    .map(way::wire),
            }
        })
        .collect();

    Ok(ranked(
        progression.rank,
        &counted,
        standing,
        wire(in_hand),
        masterable,
    ))
}

/// Stage of an item on the way to its mastery; `built` is true when it is owned, finished or
/// being built in the foundry.
fn stage(levelled: &Levelled, built: bool, recipe: Option<&Recipe>, spare: &Stock) -> Stage {
    if levelled.is_mastered() {
        return Stage::Mastered;
    }
    if built {
        return Stage::Level;
    }
    match recipe {
        Some(recipe) if spare.available(&recipe.blueprint) > 0 => match gathered(recipe, spare) {
            (held, inputs) if held == inputs => Stage::Build,
            _ => Stage::Short,
        },
        _ => Stage::Obtain,
    }
}

/// Progress of the account on each of `items`: `Built` when owned or finished in the foundry,
/// `Mastered` at its maximum rank; items with neither are omitted.
pub(crate) async fn reached(
    catalog: &Catalog,
    holdings: &SharedHoldings,
    items: &[String],
) -> Result<HashMap<String, Reached>, String> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }
    let listed = catalog
        .read(|connection| catalog::masterable(connection, "en"))
        .await
        .map_err(|e| e.to_string())?;
    let held = holdings.read().inventory;
    let counted = holdings.counted();
    let forged = forged(catalog, &held).await?;
    let mastered: HashSet<&str> = listed
        .iter()
        .zip(levelled(&held, &listed))
        .filter(|(_, levelled)| levelled.is_mastered())
        .map(|(item, _)| item.item.as_str())
        .collect();

    Ok(items
        .iter()
        .filter_map(|item| {
            let reached = match mastered.contains(item.as_str()) {
                true => Reached::Mastered,
                false
                    if counted.get(item).is_some_and(|&count| count > 0)
                        || forged.contains(item) =>
                {
                    Reached::Built
                }
                false => return None,
            };
            Some((item.clone(), reached))
        })
        .collect())
}

/// Items finished in the foundry that the account has not claimed yet.
pub(crate) async fn forged(catalog: &Catalog, held: &Inventory) -> Result<HashSet<String>, String> {
    let now = seconds();
    let blueprints: Vec<String> = held
        .progression
        .crafting
        .iter()
        .flatten()
        .filter(|build| build.ready <= now)
        .map(|build| build.item.clone())
        .collect();
    if blueprints.is_empty() {
        return Ok(HashSet::new());
    }

    let asked = blueprints.clone();
    let builds = catalog
        .read(move |connection| catalog::builds(connection, &asked))
        .await
        .map_err(|e| e.to_string())?;
    Ok(blueprints
        .iter()
        .filter_map(|blueprint| builds.get(blueprint).map(|built| built.item.clone()))
        .collect())
}

/// Levelling state of each item of `listed`, in order: its XP, and its forma count when the
/// account owns a copy.
pub(crate) fn levelled(held: &Inventory, listed: &[catalog::Masterable]) -> Vec<Levelled> {
    let xp: HashMap<&str, u64> = held
        .mastery
        .iter()
        .map(|levelled| (levelled.item.as_str(), levelled.xp))
        .collect();
    let formed: HashMap<&str, u32> = held
        .progression
        .formed
        .iter()
        .map(|formed| (formed.item.as_str(), formed.forma))
        .collect();
    let copies: HashSet<&str> = held
        .holdings
        .iter()
        .map(|holding| holding.item.as_str())
        .collect();
    listed
        .iter()
        .map(|item| {
            let key = item.item.as_str();
            Levelled {
                mastery: item.mastery,
                cap: item.cap,
                xp: xp.get(key).copied().unwrap_or(0),
                copy: copies
                    .contains(key)
                    .then(|| formed.get(key).copied().unwrap_or(0)),
            }
        })
        .collect()
}

/// Mastery points of the account by source, with the sources that could not be counted.
pub(crate) async fn breakdown_of(
    catalog: &Catalog,
    held: &Inventory,
    listed: &[catalog::Masterable],
    levelled: &[Levelled],
) -> Result<Breakdown, String> {
    let nodes = catalog
        .read(catalog::chart)
        .await
        .map_err(|e| e.to_string())?;
    let progression = &held.progression;
    let mut counted = breakdown(
        levelled,
        &nodes,
        &progression.missions,
        &progression.intrinsics,
    );
    if unknown(catalog, listed, &held.mastery).await? {
        counted.uncounted.push(median_core::Uncounted::Items);
    }
    if nodes.is_empty() {
        counted.uncounted.push(median_core::Uncounted::Nodes);
    }
    Ok(counted)
}

/// Returns (inputs of the recipe `spare` covers in full, all inputs), the blueprint included.
fn gathered(recipe: &Recipe, spare: &Stock) -> (u32, u32) {
    let enough = |item: &str, count: u32| spare.available(item) >= count;
    let parts = recipe
        .parts
        .iter()
        .filter(|part| enough(&part.item, part.count))
        .count();
    let held = u32::from(enough(&recipe.blueprint, 1)) + wire(parts as u64);
    (held, 1 + wire(recipe.parts.len() as u64))
}

/// Parts of the recipe `spare` does not cover, with the number of copies missing for each.
fn shortfall(recipe: &Recipe, spare: &Stock) -> Vec<(String, u32)> {
    recipe
        .parts
        .iter()
        .filter_map(|part| {
            let short = part.count.saturating_sub(spare.available(&part.item));
            (short > 0).then(|| (part.item.clone(), short))
        })
        .collect()
}

/// Catalog goods (name and picture) of every input in `shortfalls`, keyed by item.
async fn input_goods(
    catalog: &Catalog,
    lang: &str,
    shortfalls: &[Vec<(String, u32)>],
) -> Result<HashMap<String, median_ipc::Good>, String> {
    let mut inputs: Vec<String> = shortfalls
        .iter()
        .flatten()
        .map(|(input, _)| input.clone())
        .collect();
    inputs.sort();
    inputs.dedup();
    if inputs.is_empty() {
        return Ok(HashMap::new());
    }
    let lang = lang.to_owned();
    let goods = catalog
        .read(move |connection| catalog::goods(connection, &lang, &inputs))
        .await
        .map_err(|e| e.to_string())?;
    Ok(goods
        .into_iter()
        .map(|good| (good.item.clone(), good))
        .collect())
}

/// Narrows a count to the `u32` the IPC types use, saturating at `u32::MAX`.
pub(crate) fn wire(count: u64) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Narrows a timestamp to the `u32` IPC instant: negative becomes 0, overflow `u32::MAX`.
fn moment(at: median_core::Instant) -> Instant {
    u32::try_from(at.max(0)).unwrap_or(u32::MAX)
}

/// Whether the account has mastery XP on an item that is not in the catalog.
async fn unknown(
    catalog: &Catalog,
    listed: &[catalog::Masterable],
    levelled: &[median_core::Mastered],
) -> Result<bool, String> {
    let known: HashSet<&str> = listed.iter().map(|item| item.item.as_str()).collect();
    let strays: Vec<String> = levelled
        .iter()
        .filter(|levelled| !known.contains(levelled.item.as_str()))
        .map(|levelled| levelled.item.clone())
        .collect();
    if strays.is_empty() {
        return Ok(false);
    }
    let found = catalog
        .read(move |connection| catalog::goods(connection, "en", &strays))
        .await
        .map_err(|e| e.to_string())?;
    Ok(found.iter().any(|good| !good.known))
}

fn ranked(
    rank: Option<u32>,
    counted: &Breakdown,
    standing: Option<Outlook>,
    in_hand: u32,
    masterable: Vec<Masterable>,
) -> MasteryRank {
    MasteryRank {
        rank,
        total: wire(counted.total()),
        in_hand,
        floor: rank.map(|rank| wire(threshold(rank))),
        exact: counted.is_exact(),
        next: standing.map(|standing| wire(standing.next)),
        remaining: standing.map(|standing| wire(standing.remaining)),
        test: standing.map(|standing| test(standing.test)),
        undercounted: standing.is_some_and(|standing| standing.undercounted),
        items: share(counted.items),
        nodes: share(counted.nodes),
        steel_path: share(counted.steel_path),
        junctions: share(counted.junctions),
        steel_junctions: share(counted.steel_junctions),
        railjack: share(counted.railjack),
        drifter: share(counted.drifter),
        uncounted: counted
            .uncounted
            .iter()
            .map(|source| match source {
                median_core::Uncounted::Items => Uncounted::Items,
                median_core::Uncounted::Nodes => Uncounted::Nodes,
            })
            .collect(),
        masterable,
    }
}

fn share(share: Share) -> MasteryShare {
    MasteryShare {
        earned: wire(share.earned),
        done: share.done,
        of: share.of,
    }
}

fn test(test: median_core::RankTest) -> RankTest {
    match test {
        median_core::RankTest::Unearned => RankTest::Unearned,
        median_core::RankTest::Open => RankTest::Open,
        median_core::RankTest::Shut(until) => RankTest::Shut {
            until: moment(until),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use median_core::Mastery;

    use super::*;
    use crate::catalog::Input;

    fn braton() -> Recipe {
        let input = |item: &str, count: u32| Input {
            item: item.into(),
            count,
            class: "resource".into(),
            blueprint: None,
        };
        Recipe {
            blueprint: "BratonBlueprint".into(),
            parts: vec![input("Ferrite", 500), input("Morphics", 1)],
        }
    }

    fn fresh() -> Levelled {
        Levelled {
            mastery: Mastery::Wielded,
            cap: 30,
            xp: 0,
            copy: None,
        }
    }

    fn stock(held: &[(&str, u32)]) -> Stock {
        Stock::from(
            held.iter()
                .map(|(item, count)| ((*item).to_owned(), *count))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn a_recipe_counts_the_inputs_held_in_full() {
        let spare = stock(&[("BratonBlueprint", 1), ("Ferrite", 300), ("Morphics", 2)]);

        assert_eq!(gathered(&braton(), &spare), (2, 3));
    }

    #[test]
    fn a_recipe_falls_short_by_the_copies_not_held() {
        let spare = stock(&[("BratonBlueprint", 1), ("Ferrite", 300), ("Morphics", 2)]);

        assert_eq!(
            shortfall(&braton(), &spare),
            vec![("Ferrite".to_owned(), 200)]
        );
    }

    #[test]
    fn the_stage_follows_what_is_held_of_the_recipe() {
        let recipe = braton();
        let stage_with = |held: &[(&str, u32)]| stage(&fresh(), false, Some(&recipe), &stock(held));

        assert_eq!(stage_with(&[("Ferrite", 500)]), Stage::Obtain);
        assert_eq!(stage_with(&[("BratonBlueprint", 1)]), Stage::Short);
        assert_eq!(
            stage_with(&[("BratonBlueprint", 1), ("Ferrite", 500), ("Morphics", 1)]),
            Stage::Build
        );
        assert_eq!(
            stage(&fresh(), true, Some(&recipe), &stock(&[])),
            Stage::Level
        );
        assert_eq!(stage(&fresh(), false, None, &stock(&[])), Stage::Obtain);
    }
}
