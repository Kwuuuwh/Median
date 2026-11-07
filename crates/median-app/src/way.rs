use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

use median_core::{Version, Way, easiest, hardest};
use median_ipc::Card;

use crate::catalog::{self, Catalog, Masterable, Recipe};
use crate::roadmap::ENEMY;

/// Input classes whose own acquisition source counts toward how hard an item is to get;
/// gathered resources do not count.
const KEYED: [&str; 9] = [
    "component",
    "warframe",
    "primary",
    "secondary",
    "melee",
    "archwing-weapon",
    "companion",
    "companion-weapon",
    "modular",
];

/// Catalog data on every masterable item, with names in one language.
pub struct Masterables {
    pub listed: Vec<Masterable>,
    /// Recipe of each item that is built, one level deep.
    pub recipes: HashMap<String, Recipe>,
    /// Acquisition source of each item, taken from its hardest key; `None` while a key has no
    /// known source.
    pub ways: HashMap<String, Option<Way>>,
}

/// Cache of the masterables last read, valid while the catalog version and the language stay
/// the same.
#[derive(Default)]
pub struct MasteryCache(Mutex<Option<Kept>>);

struct Kept {
    lang: String,
    version: Option<Version>,
    masterables: Arc<Masterables>,
}

impl MasteryCache {
    /// Masterables in `lang`, read from the catalog again only when its version or the
    /// language changed.
    pub async fn masterables(
        &self,
        catalog: &Catalog,
        lang: &str,
    ) -> Result<Arc<Masterables>, String> {
        let version = catalog.version();
        if let Some(kept) = &*self.0.lock().unwrap_or_else(PoisonError::into_inner)
            && kept.lang == lang
            && kept.version == version
        {
            return Ok(Arc::clone(&kept.masterables));
        }
        let masterables = Arc::new(read(catalog, lang).await?);
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(Kept {
            lang: lang.to_owned(),
            version,
            masterables: Arc::clone(&masterables),
        });
        Ok(masterables)
    }
}

async fn read(catalog: &Catalog, lang: &str) -> Result<Masterables, String> {
    let asked = lang.to_owned();
    catalog
        .read(move |connection| {
            let listed = catalog::masterable(connection, &asked)?;
            let recipes = catalog::masterable_recipes(connection)?;
            let keyed: HashMap<&str, Vec<String>> = listed
                .iter()
                .map(|item| {
                    (
                        item.item.as_str(),
                        keys(&item.item, recipes.get(&item.item)),
                    )
                })
                .collect();
            let mut wanted: Vec<String> = keyed.values().flatten().cloned().collect();
            wanted.sort();
            wanted.dedup();
            let cards = catalog::origins(connection, &asked, &wanted)?;
            let ways = keyed
                .iter()
                .map(|(item, keys)| {
                    let way = hardest(keys.iter().map(|key| cards.get(key).and_then(way_of)));
                    ((*item).to_owned(), way)
                })
                .collect();
            Ok(Masterables {
                listed,
                recipes,
                ways,
            })
        })
        .await
        .map_err(|e| e.to_string())
}

/// Keys that must be acquired for an item: its blueprint and the blueprints of its `KEYED`
/// inputs, or the item itself when no recipe builds it.
fn keys(item: &str, recipe: Option<&Recipe>) -> Vec<String> {
    let Some(recipe) = recipe else {
        return vec![item.to_owned()];
    };
    std::iter::once(recipe.blueprint.clone())
        .chain(
            recipe
                .parts
                .iter()
                .filter(|part| KEYED.contains(&part.class.as_str()))
                .map(|part| part.blueprint.clone().unwrap_or_else(|| part.item.clone())),
        )
        .collect()
}

/// Acquisition source of the card's item that takes the least effort; `None` when it has none.
pub fn way_of(card: &Card) -> Option<Way> {
    easiest(ways(card))
}

/// Maximum number of enemy drops listed among an item's sources; more enemies add little.
const ENEMIES_KEPT: usize = 6;

/// Every acquisition source of the card's item, easiest first, each place once at its best
/// chance and at most `ENEMIES_KEPT` enemies; the card's own `way` when none is found.
pub fn every_way(card: &Card) -> Vec<median_ipc::Way> {
    let mut found = ways(card);
    found.sort_by(|one, other| {
        one.effort()
            .cmp(&other.effort())
            .then_with(|| other.odds().cmp(&one.odds()))
    });
    let mut seen = HashSet::new();
    let mut enemies = 0;
    found.retain(|way| {
        let (key, enemy) = match way {
            Way::Drop {
                place,
                mission,
                enemy,
                ..
            } => (
                format!("{place}\u{0}{}", mission.as_deref().unwrap_or("")),
                *enemy,
            ),
            other => (format!("{other:?}"), false),
        };
        if !seen.insert(key) || (enemy && enemies == ENEMIES_KEPT) {
            return false;
        }
        enemies += usize::from(enemy);
        true
    });
    if found.is_empty() {
        return card.way.iter().cloned().collect();
    }
    found.iter().map(wire).collect()
}

/// Every acquisition source on the card, except event-only drops. A blueprint researched in a
/// clan lab is sold for credits only after the research, so that credit sale counts as the lab
/// source, not as a separate one.
fn ways(card: &Card) -> Vec<Way> {
    let researched = !card.research.is_empty();
    let sold = card.sold.iter().filter_map(|sale| match sale {
        sale if sale.rotates => Some(Way::Visit {
            vendor: sale.vendor.clone(),
        }),
        sale if sale.currency.is_empty() && sale.cost.is_none() && sale.credits.is_some() => {
            (!researched).then(|| Way::Credits {
                vendor: sale.vendor.clone(),
                credits: sale.credits.unwrap_or(0),
            })
        }
        sale => Some(Way::Vendor {
            vendor: sale.vendor.clone(),
            currency: sale.currency.clone(),
            coin: sale.coin.clone(),
            cost: sale.cost,
            rank: sale.rank,
            area: sale.area.clone(),
        }),
    });
    let labs = card.research.iter().map(|research| Way::Lab {
        lab: research.lab.clone(),
    });
    let relics = (!card.found.is_empty()).then(|| Way::Relics {
        vaulted: card.good.vaulted == Some(true),
    });
    let dropped = card
        .drops
        .iter()
        .filter(|drop| !drop.event)
        .map(|drop| Way::Drop {
            place: drop.place.clone(),
            area: drop.area.clone(),
            enemy: drop.kind == ENEMY,
            chance: drop.chance,
            planet: drop.planet.clone(),
            mission: drop.mission.clone(),
            level_from: drop.level_from,
            level_to: drop.level_to,
            haunts: drop.haunts.clone(),
        });
    sold.chain(labs).chain(relics).chain(dropped).collect()
}

/// Converts an acquisition source to its IPC form.
pub fn wire(way: &Way) -> median_ipc::Way {
    match way.clone() {
        Way::Credits { vendor, credits } => median_ipc::Way::Credits { vendor, credits },
        Way::Vendor {
            vendor,
            currency,
            coin,
            cost,
            rank,
            area,
        } => median_ipc::Way::Vendor {
            vendor,
            currency,
            coin,
            cost,
            rank,
            area,
        },
        Way::Lab { lab } => median_ipc::Way::Lab { lab },
        Way::Relics { vaulted } => median_ipc::Way::Relics { vaulted },
        Way::Drop {
            place,
            area,
            enemy,
            chance,
            planet,
            mission,
            level_from,
            level_to,
            haunts,
        } => median_ipc::Way::Drop {
            place,
            area,
            enemy,
            chance,
            planet,
            mission,
            level_from,
            level_to,
            haunts,
        },
        Way::Visit { vendor } => median_ipc::Way::Visit { vendor },
    }
}

#[cfg(test)]
mod tests {
    use median_ipc::{Drop, Good, Research, Sale};

    use super::*;
    use crate::catalog::Input;

    fn card(sold: Vec<Sale>, drops: Vec<Drop>, research: Vec<Research>) -> Card {
        Card {
            good: Good {
                item: "/Blueprint".into(),
                name: "Blueprint".into(),
                kind: "blueprint".into(),
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
            sold,
            inside: Vec::new(),
            found: Vec::new(),
            drops,
            research,
            recipes: Vec::new(),
            needed_by: Vec::new(),
            way: None,
        }
    }

    fn sale(vendor: &str, currency: &str, cost: Option<u32>, credits: Option<u32>) -> Sale {
        Sale {
            vendor: vendor.into(),
            cost,
            currency: currency.into(),
            coin: None,
            credits,
            rank: None,
            area: None,
            place: None,
            pack: None,
            rotates: false,
        }
    }

    fn drop(place: &str, kind: &str, chance: u32, event: bool) -> Drop {
        Drop {
            kind: kind.into(),
            place: place.into(),
            event,
            planet: None,
            mission: None,
            giver: None,
            level_from: None,
            level_to: None,
            rotation: None,
            stage: None,
            rarity: "common".into(),
            chance,
            count: None,
            refinement: None,
            area: Some("cetus".into()),
            haunts: Vec::new(),
        }
    }

    #[test]
    fn credits_at_a_standing_trader_beat_standing_and_drops() {
        let card = card(
            vec![
                sale("Hok", "Standing", Some(5_000), None),
                sale("Market", "", None, Some(25_000)),
            ],
            vec![drop("Cetus Bounty", "bounty", 2_000, false)],
            Vec::new(),
        );

        assert_eq!(
            way_of(&card),
            Some(Way::Credits {
                vendor: "Market".into(),
                credits: 25_000,
            })
        );
    }

    #[test]
    fn a_blueprint_the_lab_researches_is_the_labs_whatever_credits_it_is_sold_for() {
        let research = Research {
            lab: "Tenno".into(),
            credits: 10_000,
            time: 259_200,
            affinity: 0,
            prereq: None,
        };
        let card = card(
            vec![sale("Market", "", None, Some(15_000))],
            Vec::new(),
            vec![research],
        );

        assert_eq!(
            way_of(&card),
            Some(Way::Lab {
                lab: "Tenno".into()
            })
        );
    }

    #[test]
    fn a_trader_who_comes_and_goes_is_a_visit() {
        let mut baro = sale("Baro", "Ducats", Some(300), Some(100_000));
        baro.rotates = true;

        assert_eq!(
            way_of(&card(vec![baro], Vec::new(), Vec::new())),
            Some(Way::Visit {
                vendor: "Baro".into()
            })
        );
    }

    #[test]
    fn an_event_drop_is_no_way_to_get_anything() {
        let card = card(
            Vec::new(),
            vec![drop("Plague Star", "bounty", 5_000, true)],
            Vec::new(),
        );

        assert_eq!(way_of(&card), None);
    }

    #[test]
    fn an_enemy_drop_says_so() {
        let card = card(
            Vec::new(),
            vec![drop("Grineer Lancer", ENEMY, 30, false)],
            Vec::new(),
        );

        assert!(matches!(way_of(&card), Some(Way::Drop { enemy: true, .. })));
    }

    #[test]
    fn every_way_names_each_place_once_at_its_best_chance_the_easiest_first() {
        let card = card(
            vec![sale("Market", "", None, Some(25_000))],
            vec![
                drop("Cetus", "bounty", 100, false),
                drop("Cetus", "bounty", 250, false),
                drop("Fortuna", "bounty", 150, false),
            ],
            Vec::new(),
        );

        let places: Vec<String> = every_way(&card)
            .into_iter()
            .map(|way| match way {
                median_ipc::Way::Drop { place, chance, .. } => format!("{place} {chance}"),
                median_ipc::Way::Credits { vendor, .. } => vendor,
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(places, vec!["Market", "Cetus 250", "Fortuna 150"]);
    }

    #[test]
    fn every_way_keeps_no_more_than_a_few_enemies() {
        let drops = (0..10)
            .map(|index| drop(&format!("Enemy {index}"), ENEMY, 10 + index, false))
            .collect();

        assert_eq!(
            every_way(&card(Vec::new(), drops, Vec::new())).len(),
            ENEMIES_KEPT
        );
    }

    #[test]
    fn the_keys_of_a_built_item_are_its_blueprint_and_its_components_not_its_resources() {
        let recipe = Recipe {
            blueprint: "GyreBlueprint".into(),
            parts: vec![
                Input {
                    item: "OrokinCell".into(),
                    count: 3,
                    class: "resource".into(),
                    blueprint: Some("OrokinCellBlueprint".into()),
                },
                Input {
                    item: "GyreChassis".into(),
                    count: 1,
                    class: "component".into(),
                    blueprint: Some("GyreChassisBlueprint".into()),
                },
            ],
        };

        assert_eq!(
            keys("Gyre", Some(&recipe)),
            ["GyreBlueprint", "GyreChassisBlueprint"]
        );
        assert_eq!(keys("KuvaBramma", None), ["KuvaBramma"]);
    }
}
