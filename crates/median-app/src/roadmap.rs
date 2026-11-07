use std::collections::{BTreeMap, HashMap};

use median_core::{
    Assembly, Bill, Inventory, Judged, Levelled, Line, Stock, at_least_once, billed, buildable,
    reached, startable, supplied, walk, walk_forge, walk_plan,
};
use median_ipc::{Area, Card, Good, Planned, Ready, Requirement, Requirements, Roadmap};
use rusqlite::Connection;

use crate::account;
use crate::catalog::{self, Catalog};
use crate::collection::goods_of;
use crate::market::{self, Store};
use crate::mastery::{breakdown_of, forged, levelled, wire};
use crate::state_sink::SharedHoldings;
use crate::way::MasteryCache;

/// Permille in a whole: the catalog gives drop chances in tenths of a percent.
const PERMILLE: f64 = 1_000.0;

/// Refinement of a relic as it drops, before any refining.
const INTACT: &str = "intact";

/// Maximum number of relic names on one requirement line.
const RELICS_SHOWN: usize = 3;

/// Catalog drop kind of an enemy kill, as opposed to a mission or other place reward.
pub(crate) const ENEMY: &str = "enemy";

/// Chance of a guaranteed drop, in tenths of a percent (100%).
const CERTAIN: u32 = 1_000;

/// Forma item; on an item ranked past 30, each forma on a copy unlocks two more ranks.
const FORMA: &str = "/Lotus/Types/Items/MiscItems/Forma";

/// Roadmap of the planned items not yet mastered: their combined requirements, the mastery they
/// add, builds that can start now, foundry lines and the areas that provide missing inputs.
pub async fn roadmap(
    catalog: &Catalog,
    cache: &MasteryCache,
    market_cache: &Store,
    holdings: &SharedHoldings,
    history: &account::Store,
    lang: &str,
) -> Result<Roadmap, String> {
    let held = holdings.read().inventory;
    let masterables = cache.masterables(catalog, lang).await?;
    let masterable = &masterables.listed;
    let levelled = levelled(&held, masterable);
    let listed = history
        .roadmap(held.profile.clone())
        .await
        .map_err(|e| e.to_string())?;
    let standing: HashMap<&str, &Levelled> = masterable
        .iter()
        .zip(&levelled)
        .map(|(item, levelled)| (item.item.as_str(), levelled))
        .collect();
    let mastered = |item: &str| {
        standing
            .get(item)
            .is_some_and(|levelled| levelled.is_mastered())
    };
    let aims: Vec<String> = listed
        .iter()
        .filter(|item| !mastered(item))
        .cloned()
        .collect();
    let gain: u64 = aims
        .iter()
        .filter_map(|item| standing.get(item.as_str()))
        .map(|levelled| levelled.remaining())
        .sum();
    let reaches = match held.progression.rank {
        Some(_) => {
            let counted = breakdown_of(catalog, &held, masterable, &levelled).await?;
            Some(reached(counted.total() + gain))
        }
        None => None,
    };
    let choices = history
        .part_choices(held.profile.clone())
        .await
        .map_err(|e| e.to_string())?;
    let counted = holdings.counted();
    let forma: u32 = aims
        .iter()
        .filter_map(|item| standing.get(item.as_str()))
        .map(|levelled| levelled.forma())
        .sum();

    let mut stock = spare(catalog, &held, &counted).await?;
    let asked = aims.clone();
    let (trees, fitted) = catalog
        .read(move |connection| {
            let trees = asked
                .iter()
                .map(|item| tree(connection, item, 1))
                .collect::<Result<Vec<Assembly>, String>>()?;
            let fitted = match forma {
                0 => None,
                count => Some(tree(connection, FORMA, count)?),
            };
            Ok((trees, fitted))
        })
        .await
        .map_err(|e| e.to_string())?;
    let before = stock.clone();
    let walks: Vec<Judged> = walk_plan(&trees, &mut stock);
    let mut bills: Vec<Bill> = walks.iter().map(billed).collect();
    if let Some(tree) = &fitted {
        bills.push(supplied(&walk(tree, &mut stock)));
    }
    let tops: Vec<Assembly> = trees.iter().chain(&fitted).cloned().collect();
    let forgeable = walk_forge(&tops, &before);
    let started = startable(&forgeable);
    let builds = buildable(&forgeable);
    let mut total = Bill::default();
    for billed in &bills {
        total.add(billed);
    }

    let mut lines: Vec<String> = total.lines.keys().chain(builds.keys()).cloned().collect();
    lines.sort();
    lines.dedup();
    let named = goods_of(
        catalog,
        lang,
        listed
            .iter()
            .chain(&lines)
            .chain(started.keys())
            .cloned()
            .collect(),
    )
    .await?;
    let asked = (lang.to_string(), lines.clone());
    let (cards, areas) = catalog
        .read(move |connection| {
            let (lang, lines) = asked;
            Ok((
                catalog::origins(connection, &lang, &lines)?,
                catalog::areas(connection, &lang)?,
            ))
        })
        .await
        .map_err(|e| e.to_string())?;
    let prices: HashMap<String, Option<u32>> = market::quoted(catalog, market_cache, &lines)
        .await?
        .into_iter()
        .map(|quote| (quote.item, quote.ingame))
        .collect();
    let facts = Facts {
        named: &named,
        cards: &cards,
        prices: &prices,
        counted: &counted,
        purse: held.purse.credits.map(wire),
    };

    let planned: Vec<Planned> = aims
        .iter()
        .map(|item| {
            let good = named.get(item);
            Planned {
                item: item.clone(),
                name: catalog::name_of(good, item),
                image: good.and_then(|good| good.image.clone()),
                owned: counted.contains_key(item),
                remaining: standing
                    .get(item.as_str())
                    .map_or(0, |levelled| wire(levelled.remaining())),
            }
        })
        .collect();
    let ready = started
        .iter()
        .map(|(item, count)| {
            let good = named.get(item);
            Ready {
                item: item.clone(),
                name: catalog::name_of(good, item),
                image: good.and_then(|good| good.image.clone()),
                count: *count,
            }
        })
        .collect();
    let every = |part: &str| {
        let made: Vec<bool> = choices
            .iter()
            .filter(|choice| choice.part == part)
            .map(|choice| choice.buy)
            .collect();
        (!made.is_empty()).then(|| made.iter().all(|&buy| buy))
    };
    let drawn = total.lines.clone();
    let total = wired(&total, &facts, &before, every);
    let finished = forged(catalog, &held).await?;
    let claimed = finished
        .iter()
        .filter(|item| !builds.contains_key(*item) && !standing.contains_key(item.as_str()))
        .filter_map(|item| {
            let line = *drawn.get(item)?;
            Some(requirement(
                item,
                Line {
                    built: true,
                    ..line
                },
                &facts,
                &before,
                None,
            ))
        });
    let forge = builds
        .iter()
        .map(|(item, count)| {
            let have = before.held(item);
            let forging = before.forging(item);
            let line = Line {
                count: have + forging + count,
                have,
                forging,
                kept: false,
                built: true,
            };
            requirement(item, line, &facts, &before, None)
        })
        .chain(claimed)
        .collect();
    let areas = giving(areas, &total.lines);

    Ok(Roadmap {
        planned,
        total,
        gain: wire(gain),
        reaches,
        ready,
        forge,
        areas,
    })
}

/// Areas that provide at least one missing requirement, the one providing most first, then by
/// name.
fn giving(areas: Vec<Area>, lines: &[Requirement]) -> Vec<Area> {
    let short: Vec<&Requirement> = lines.iter().filter(|line| !is_covered(line)).collect();
    let mut given: Vec<(usize, Area)> = areas
        .into_iter()
        .map(|area| {
            let lines = short
                .iter()
                .filter(|line| line.areas.contains(&area.key))
                .count();
            (lines, area)
        })
        .filter(|(lines, _)| *lines > 0)
        .collect();
    given.sort_by(|(one, first), (other, second)| {
        other.cmp(one).then_with(|| first.name.cmp(&second.name))
    });
    given.into_iter().map(|(_, area)| area).collect()
}

/// Keys of the areas where a non-event drop or a vendor provides the item, without duplicates.
fn areas_of(card: &Card) -> Vec<String> {
    let mut areas: Vec<String> = card
        .drops
        .iter()
        .filter(|drop| !drop.event)
        .filter_map(|drop| drop.area.clone())
        .chain(card.sold.iter().filter_map(|sale| sale.area.clone()))
        .collect();
    areas.sort();
    areas.dedup();
    areas
}

/// Stock available for builds: the account's items plus the items the foundry is building.
pub(crate) async fn spare(
    catalog: &Catalog,
    held: &Inventory,
    counted: &HashMap<String, u32>,
) -> Result<Stock, String> {
    let holdings: BTreeMap<String, u32> = counted
        .iter()
        .map(|(item, count)| (item.clone(), *count))
        .collect();

    let blueprints: Vec<String> = held
        .progression
        .crafting
        .iter()
        .flatten()
        .map(|build| build.item.clone())
        .collect();
    let asked = blueprints.clone();
    let builds = catalog
        .read(move |connection| catalog::builds(connection, &asked))
        .await
        .map_err(|e| e.to_string())?;
    let mut forging: BTreeMap<String, u32> = BTreeMap::new();
    for blueprint in &blueprints {
        if let Some(built) = builds.get(blueprint) {
            *forging.entry(built.item.clone()).or_default() += 1;
        }
    }
    Ok(Stock::new(holdings, forging))
}

/// Lookup data used to build the requirement lines of a bill.
struct Facts<'a> {
    named: &'a HashMap<String, Good>,
    cards: &'a HashMap<String, Card>,
    prices: &'a HashMap<String, Option<u32>>,
    counted: &'a HashMap<String, u32>,
    purse: Option<u32>,
}

/// Converts a bill to its IPC form: lines still short first, then covered lines, each group by
/// name, with the platinum cost of the lines to buy.
fn wired(
    billed: &Bill,
    facts: &Facts<'_>,
    before: &Stock,
    choice: impl Fn(&str) -> Option<bool>,
) -> Requirements {
    let mut lines: Vec<Requirement> = billed
        .lines
        .iter()
        .map(|(item, line)| requirement(item, *line, facts, before, choice(item)))
        .collect();
    lines.sort_by(|one, other| {
        is_covered(one)
            .cmp(&is_covered(other))
            .then_with(|| one.name.cmp(&other.name))
    });
    let buying = lines.iter().filter(|line| line.buy && !is_covered(line));
    let to_buy = buying
        .clone()
        .filter_map(|line| line.price.map(|price| price * short(line)))
        .sum();
    let unpriced = buying.filter(|line| line.price.is_none()).count() as u32;

    Requirements {
        lines,
        credits: wire(billed.credits),
        purse: facts.purse,
        time: wire(billed.time),
        to_buy,
        unpriced,
    }
}

fn is_covered(line: &Requirement) -> bool {
    short(line) == 0
}

/// Copies that neither the account's items nor the foundry cover.
fn short(line: &Requirement) -> u32 {
    line.count.saturating_sub(line.have + line.forging)
}

fn requirement(
    item: &str,
    line: Line,
    facts: &Facts<'_>,
    before: &Stock,
    choice: Option<bool>,
) -> Requirement {
    let good = facts.named.get(item);
    let tradable = good.and_then(|good| good.tradable) == Some(true);
    let vaulted = good.and_then(|good| good.vaulted) == Some(true);
    let card = facts.cards.get(item);

    let owned: Vec<(f64, u32)> = card
        .iter()
        .flat_map(|card| &card.found)
        .filter_map(|relic| {
            let held = facts.counted.get(&relic.item).copied().unwrap_or(0);
            (held > 0).then(|| (f64::from(relic.chance) / PERMILLE, held))
        })
        .collect();
    let relics = match vaulted {
        true => 0,
        false => card.map_or(0, |card| {
            card.found
                .iter()
                .filter(|relic| {
                    relic
                        .refinement
                        .as_deref()
                        .is_none_or(|step| step == INTACT)
                })
                .count() as u32
        }),
    };
    let owned_relics = owned.iter().map(|&(_, held)| held).sum();
    let way = card.and_then(|card| card.way.clone());
    let farmable = owned_relics > 0 || relics > 0 || way.is_some();
    let short = line
        .count
        .saturating_sub(before.held(item) + before.forging(item));

    Requirement {
        item: item.to_string(),
        name: catalog::name_of(good, item),
        image: good.and_then(|good| good.image.clone()),
        kind: good.map_or_else(String::new, |good| good.kind.clone()),
        count: line.count,
        have: line.have,
        forging: line.forging,
        built: line.built,
        tradable,
        slug: good.and_then(|good| good.slug.clone()),
        price: facts.prices.get(item).copied().flatten(),
        buy: choice.unwrap_or_else(|| bought_by_default(farmable, tradable)),
        owned_relics,
        owned_chance: median_ipc::permille(at_least_once(&owned)),
        relics,
        relic_names: relic_names(card, facts.counted, vaulted),
        vaulted,
        way,
        ways: card.map(crate::way::every_way).unwrap_or_default(),
        runs: match short {
            0 => None,
            short => card.and_then(|card| runs(card, short)),
        },
        areas: card.map(areas_of).unwrap_or_default(),
    }
}

/// One run that yields a drop: place, mission, rotation and stage.
type Run<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Option<&'a str>);

/// Runs of the best-yielding place needed for `short` copies, counting only guaranteed drops.
/// `None` when no place drops the item every run, and for any item enemies drop: that is farmed
/// by kills, which a run count does not describe.
fn runs(card: &Card, short: u32) -> Option<u32> {
    if card.drops.iter().any(|drop| drop.kind == ENEMY) {
        return None;
    }
    let mut yields: HashMap<Run<'_>, f64> = HashMap::new();
    for drop in card
        .drops
        .iter()
        .filter(|drop| !drop.event && drop.chance >= CERTAIN)
    {
        let run = (
            drop.place.as_str(),
            drop.mission.as_deref(),
            drop.rotation.as_deref(),
            drop.stage.as_deref(),
        );
        *yields.entry(run).or_default() +=
            f64::from(drop.chance) / PERMILLE * f64::from(drop.count.unwrap_or(1));
    }
    let richest = yields.into_values().fold(0.0, f64::max);
    (richest > 0.0).then(|| (f64::from(short) / richest).ceil() as u32)
}

/// Build tree for `count` copies of the item, or a leaf without a craft when no recipe builds it.
fn tree(connection: &Connection, item: &str, count: u32) -> Result<Assembly, String> {
    let found = catalog::assembly(connection, item)?;
    Ok(found.map_or_else(
        || Assembly {
            item: item.to_string(),
            count,
            craft: None,
        },
        |tree| Assembly { count, ..tree },
    ))
}

/// Names of up to `RELICS_SHOWN` relics that drop the item, the most-owned first; a relic the
/// account does not own is listed only when the item is not vaulted and the relic is intact.
fn relic_names(card: Option<&Card>, counted: &HashMap<String, u32>, vaulted: bool) -> Vec<String> {
    let mut found: Vec<(u32, &str)> = card
        .iter()
        .flat_map(|card| &card.found)
        .filter_map(|relic| {
            let held = counted.get(&relic.item).copied().unwrap_or(0);
            let dropping = !vaulted
                && relic
                    .refinement
                    .as_deref()
                    .is_none_or(|step| step == INTACT);
            (held > 0 || dropping).then_some((held, relic.name.as_str()))
        })
        .collect();
    found.sort_by_key(|&(held, _)| std::cmp::Reverse(held));

    let mut names: Vec<String> = Vec::new();
    for (_, name) in found {
        if names.len() == RELICS_SHOWN {
            break;
        }
        if !names.iter().any(|kept| kept == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// Default buy choice of a line until the player sets one: buy it when it is tradable and has
/// no farming source.
fn bought_by_default(farmable: bool, tradable: bool) -> bool {
    !farmable && tradable
}

#[cfg(test)]
mod tests {
    use super::*;
    use median_ipc::Drop;

    #[test]
    fn what_nothing_farms_and_the_market_trades_is_bought_until_said_otherwise() {
        assert!(bought_by_default(false, true));
        assert!(!bought_by_default(true, true));
        assert!(!bought_by_default(false, false));
    }

    fn drop(place: &str, kind: &str, chance: u32, count: Option<u32>) -> Drop {
        Drop {
            kind: kind.to_string(),
            place: place.to_string(),
            event: false,
            planet: None,
            mission: None,
            giver: None,
            level_from: None,
            level_to: None,
            rotation: None,
            stage: None,
            rarity: "common".to_string(),
            chance,
            count,
            refinement: None,
            area: None,
            haunts: Vec::new(),
        }
    }

    fn card(drops: Vec<Drop>) -> Card {
        Card {
            good: Good {
                item: "/Clamp".to_string(),
                name: "Pathos Clamp".to_string(),
                kind: "resource".to_string(),
                image: None,
                known: true,
                slug: None,
                tradable: Some(false),
                vaulted: None,
                prime: false,
                ducats: None,
                makes: None,
                mastered: None,
                collection: None,
            },
            sold: Vec::new(),
            inside: Vec::new(),
            found: Vec::new(),
            drops,
            research: Vec::new(),
            recipes: Vec::new(),
            needed_by: Vec::new(),
            way: None,
        }
    }

    #[test]
    fn runs_are_counted_from_what_a_place_hands_over_every_time() {
        let clamps = card(vec![
            drop("Duviri Full Experience", "transient", CERTAIN, Some(10)),
            drop(
                "Duviri Full Experience (Steel Path)",
                "transient",
                CERTAIN,
                Some(15),
            ),
            drop("Duviri Circuit", "transient", 300, Some(40)),
        ]);

        assert_eq!(runs(&clamps, 216), Some(15));
    }

    #[test]
    fn a_chance_drop_gives_no_count_of_runs() {
        let plants = card(vec![drop("Duviri Circuit", "transient", 300, Some(40))]);

        assert_eq!(runs(&plants, 30), None);
    }

    #[test]
    fn what_enemies_drop_is_farmed_by_killing_and_counts_no_runs() {
        let ferrite = card(vec![
            drop("Earth/Cambria (Excavation)", "node", CERTAIN, Some(300)),
            drop("Grineer Lancer", ENEMY, 500, Some(50)),
        ]);

        assert_eq!(runs(&ferrite, 1_000), None);
    }
}
