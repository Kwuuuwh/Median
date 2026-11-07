use std::collections::{HashMap, HashSet};

use median_core::{Assembly, Craft, Mastery, Node, Purchase};
use rusqlite::{Connection, OptionalExtension, params_from_iter};

/// Catalog class of resources; a resource's recipe is expanded only when it is not farmed.
const GATHERED: &str = "resource";

/// Number of drop sources (locations plus enemies) from which a resource counts as farmed and
/// its recipe is ignored.
const FARMED: usize = 10;

/// Maximum depth to which a recipe tree is expanded; the game's trees end at three levels.
const DEPTH: u32 = 6;

/// Classes whose items the collection lists; the components that build them are not listed.
const COLLECTED: [&str; 9] = [
    "warframe",
    "primary",
    "secondary",
    "melee",
    "archwing-weapon",
    "companion",
    "companion-weapon",
    "mod",
    "arcane",
];

/// Mod kind of focus school ways: the catalog stores them as mods, but a player does not own
/// them as mods.
const FOCUS: &str = "focus-way";

/// Catalog data of an item that gives mastery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Masterable {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    pub class: String,
    pub kind: String,
    pub mastery: Mastery,
    pub cap: u32,
    /// Mastery rank required to build the item's blueprint.
    pub required: u32,
    pub prime: bool,
    pub vaulted: bool,
}

/// Catalog data of one item the collection lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    pub class: String,
    pub kind: String,
    pub prime: bool,
    pub vaulted: bool,
    pub tradable: bool,
    pub mastery: Option<Mastery>,
    pub cap: Option<u32>,
    pub required: u32,
}

/// Every in-scope item that gives mastery, named in `lang` with English as fallback; fails on a
/// mastery kind this build cannot parse or a missing rank cap.
pub fn masterable(connection: &Connection, lang: &str) -> Result<Vec<Masterable>, String> {
    let mut statement = connection
        .prepare(
            "SELECT items.unique_name,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    icons.image_id,
                    items.class,
                    items.mastery,
                    items.max_level_cap,
                    COALESCE(items.mastery_req, 0),
                    items.prime,
                    COALESCE(items.vaulted, 0),
                    items.kind
               FROM items
               LEFT JOIN item_icons AS icons
                 ON icons.unique_name = items.unique_name AND icons.lang = ?1
              WHERE items.mastery IS NOT NULL AND items.in_scope = 1
              ORDER BY items.unique_name",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map([lang], |row| {
            Ok((
                Masterable {
                    item: row.get(0)?,
                    name: row.get(1)?,
                    image: row.get(2)?,
                    class: row.get(3)?,
                    kind: row.get(9)?,
                    mastery: Mastery::Wielded,
                    cap: 0,
                    required: row.get(6)?,
                    prime: row.get(7)?,
                    vaulted: row.get(8)?,
                },
                row.get::<_, String>(4)?,
                row.get::<_, Option<u32>>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut found = Vec::new();
    for row in rows {
        let (item, kind, cap) = row.map_err(|e| e.to_string())?;
        let mastery = Mastery::parse(&kind).ok_or_else(|| {
            format!(
                "{} gives a kind of mastery this build does not know: {kind}",
                item.item
            )
        })?;
        let cap = cap.ok_or_else(|| format!("{} gives mastery up to no rank", item.item))?;
        found.push(Masterable {
            mastery,
            cap,
            ..item
        });
    }
    Ok(found)
}

/// Recipe of a masterable item, one level deep: its blueprint and direct inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub blueprint: String,
    pub parts: Vec<Input>,
}

/// One input of a recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub item: String,
    pub count: u32,
    pub class: String,
    /// Blueprint that builds the input; `None` when no recipe builds it.
    pub blueprint: Option<String>,
}

/// Recipe of every masterable item that has one, keyed by the item's unique name.
pub fn masterable_recipes(connection: &Connection) -> Result<HashMap<String, Recipe>, String> {
    let mut statement = connection
        .prepare(
            "SELECT recipes.result, recipes.blueprint, requires.item, requires.count,
                    COALESCE(parts.class, ''),
                    (SELECT MIN(own.blueprint) FROM recipes AS own WHERE own.result = requires.item)
               FROM items
               JOIN recipes ON recipes.result = items.unique_name
               LEFT JOIN recipe_requires AS requires ON requires.blueprint = recipes.blueprint
               LEFT JOIN items AS parts ON parts.unique_name = requires.item
              WHERE items.mastery IS NOT NULL AND items.in_scope = 1
              ORDER BY recipes.result, recipes.blueprint, requires.count DESC, requires.item",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<u32>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut found: HashMap<String, Recipe> = HashMap::new();
    for row in rows {
        let (result, blueprint, part, count, class, built) = row.map_err(|e| e.to_string())?;
        let recipe = found.entry(result).or_insert_with(|| Recipe {
            blueprint: blueprint.clone(),
            parts: Vec::new(),
        });
        // When two recipes build the item, only the first by blueprint is used, as in `assembly`.
        if recipe.blueprint != blueprint {
            continue;
        }
        if let Some(item) = part {
            recipe.parts.push(Input {
                item,
                count: count.unwrap_or(1),
                class,
                blueprint: built,
            });
        }
    }
    Ok(found)
}

/// Every item the collection lists: in-scope items of the `COLLECTED` classes except focus ways,
/// and the modular-weapon parts that give mastery.
pub fn entries(connection: &Connection, lang: &str) -> Result<Vec<Entry>, String> {
    let classes = COLLECTED
        .iter()
        .map(|class| format!("'{class}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut statement = connection
        .prepare(&format!(
            "SELECT items.unique_name,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    icons.image_id,
                    items.class,
                    items.kind,
                    items.prime,
                    COALESCE(items.vaulted, 0),
                    COALESCE(items.tradable, 0),
                    items.mastery,
                    items.max_level_cap,
                    COALESCE(items.mastery_req, 0)
               FROM items
               LEFT JOIN item_icons AS icons
                 ON icons.unique_name = items.unique_name AND icons.lang = ?1
              WHERE items.in_scope = 1
                AND ((items.class IN ({classes}) AND items.kind <> '{FOCUS}')
                     OR (items.class = 'modular' AND items.mastery IS NOT NULL))
              ORDER BY items.unique_name"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map([lang], |row| {
            Ok(Entry {
                item: row.get(0)?,
                name: row.get(1)?,
                image: row.get(2)?,
                class: row.get(3)?,
                kind: row.get(4)?,
                prime: row.get(5)?,
                vaulted: row.get(6)?,
                tradable: row.get(7)?,
                mastery: row
                    .get::<_, Option<String>>(8)?
                    .as_deref()
                    .and_then(Mastery::parse),
                cap: row.get(9)?,
                required: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Entry>>>()
        .map_err(|e| e.to_string())
}

/// Reward keys of every item the given relics can drop; empty when `relics` is empty.
pub fn rewards_of(connection: &Connection, relics: &[String]) -> Result<HashSet<String>, String> {
    if relics.is_empty() {
        return Ok(HashSet::new());
    }
    let places = vec!["?"; relics.len()].join(", ");
    let mut statement = connection
        .prepare(&format!(
            "SELECT DISTINCT reward FROM relic_rewards WHERE relic IN ({places})"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(relics.iter()), |row| {
            row.get::<_, String>(0)
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<HashSet<String>>>()
        .map_err(|e| e.to_string())
}

/// Star chart nodes whose first completion gives mastery, junctions included.
pub fn chart(connection: &Connection) -> Result<Vec<Node>, String> {
    let mut statement = connection
        .prepare("SELECT node, mastery_xp FROM regions WHERE mastery_xp > 0 ORDER BY node")
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map([], |row| {
            Ok(Node {
                tag: row.get(0)?,
                points: row.get::<_, i64>(1)?.max(0) as u64,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Node>>>()
        .map_err(|e| e.to_string())
}

/// Build tree of `item`, expanded until a part has no recipe or is a farmed resource; `None`
/// when no recipe builds `item`.
pub fn assembly(connection: &Connection, item: &str) -> Result<Option<Assembly>, String> {
    Ok(craft(connection, item, 0)?.map(|craft| Assembly {
        item: item.to_string(),
        count: 1,
        craft: Some(craft),
    }))
}

/// Recipe of `item` with its parts expanded recursively up to `DEPTH`; `None` when no recipe
/// builds `item`.
fn craft(connection: &Connection, item: &str, depth: u32) -> Result<Option<Craft>, String> {
    let recipe = connection
        .prepare_cached(
            "SELECT blueprint, COALESCE(consumed, 1), COALESCE(build_price, 0),
                    COALESCE(build_time, 0), COALESCE(output, 1)
               FROM recipes
              WHERE result = ?1
              ORDER BY blueprint
              LIMIT 1",
        )
        .and_then(|mut statement| {
            statement
                .query_row([item], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, i64>(2)?.max(0) as u64,
                        row.get::<_, i64>(3)?.max(0) as u64,
                        row.get::<_, i64>(4)?.clamp(1, i64::from(u32::MAX)) as u32,
                    ))
                })
                .optional()
        })
        .map_err(|e| e.to_string())?;
    let Some((blueprint, consumed, credits, time, output)) = recipe else {
        return Ok(None);
    };

    let mut statement = connection
        .prepare_cached(
            "SELECT requires.item, requires.count, COALESCE(items.class, '')
               FROM recipe_requires AS requires
               LEFT JOIN items ON items.unique_name = requires.item
              WHERE requires.blueprint = ?1
              ORDER BY requires.count DESC, requires.item",
        )
        .map_err(|e| e.to_string())?;
    let asked = statement
        .query_map([&blueprint], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    let mut parts = Vec::with_capacity(asked.len());
    for (part, count, class) in asked {
        let built = match depth < DEPTH && (class != GATHERED || !is_farmed(connection, &part)?) {
            true => craft(connection, &part, depth + 1)?,
            false => None,
        };
        parts.push(Assembly {
            item: part,
            count,
            craft: built,
        });
    }

    Ok(Some(Craft {
        bought: bought(connection, &blueprint)?,
        blueprint,
        consumed,
        credits,
        time,
        output,
        parts,
    }))
}

/// Permanent offer of the blueprint at a non-rotating vendor, paid with an item or with credits
/// only; credit-only offers come first, then the lowest cost.
fn bought(connection: &Connection, blueprint: &str) -> Result<Option<Purchase>, String> {
    connection
        .prepare_cached(
            "SELECT offers.pays, COALESCE(offers.cost, 0), COALESCE(offers.credits, 0),
                    offers.count
               FROM vendor_offers AS offers
               JOIN vendors ON vendors.key = offers.vendor
              WHERE offers.item = ?1 AND offers.always = 1 AND vendors.rotates = 0
                AND (offers.pays IS NOT NULL
                     OR (offers.cost IS NULL AND offers.credits IS NOT NULL))
              ORDER BY offers.pays IS NOT NULL, COALESCE(offers.cost, 0), offers.vendor
              LIMIT 1",
        )
        .and_then(|mut statement| {
            statement
                .query_row([blueprint], |row| {
                    let cost = row.get::<_, i64>(1)?.clamp(0, i64::from(u32::MAX)) as u32;
                    Ok(Purchase {
                        pays: row.get::<_, Option<String>>(0)?.map(|pays| (pays, cost)),
                        credits: row.get::<_, i64>(2)?.max(0) as u64,
                        count: row.get::<_, i64>(3)?.clamp(1, i64::from(u32::MAX)) as u32,
                    })
                })
                .optional()
        })
        .map_err(|e| e.to_string())
}

/// Whether at least `FARMED` locations and enemies drop the item, so it counts as farmed rather
/// than built.
fn is_farmed(connection: &Connection, item: &str) -> Result<bool, String> {
    let sources: i64 = connection
        .prepare_cached(
            "SELECT (SELECT COUNT(*) FROM item_drops WHERE item = ?1)
                  + (SELECT COUNT(*) FROM enemy_drops WHERE item = ?1)",
        )
        .and_then(|mut statement| statement.query_row([item], |row| row.get(0)))
        .map_err(|e| e.to_string())?;
    Ok(sources >= FARMED as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::builds;

    /// Catalog with only the tables the mastery, collection and build queries read.
    fn catalog() -> Connection {
        let connection = Connection::open_in_memory().expect("a catalog in memory");
        connection
            .execute_batch(
                "CREATE TABLE items (
                     unique_name TEXT PRIMARY KEY, name_en TEXT NOT NULL, name_ru TEXT,
                     class TEXT NOT NULL, kind TEXT NOT NULL, prime INTEGER NOT NULL,
                     vaulted INTEGER, tradable INTEGER, in_scope INTEGER NOT NULL,
                     mastery TEXT, mastery_req INTEGER, max_level_cap INTEGER
                 );
                 CREATE TABLE item_icons (
                     unique_name TEXT NOT NULL, lang TEXT NOT NULL, image_id TEXT NOT NULL
                 );
                 CREATE TABLE sets (slug TEXT PRIMARY KEY, built TEXT);
                 CREATE TABLE recipes (
                     blueprint TEXT PRIMARY KEY, result TEXT NOT NULL, build_price INTEGER,
                     build_time INTEGER, consumed INTEGER, output INTEGER
                 );
                 CREATE TABLE recipe_requires (
                     blueprint TEXT NOT NULL, item TEXT NOT NULL, count INTEGER NOT NULL
                 );
                 CREATE TABLE item_drops (item TEXT NOT NULL);
                 CREATE TABLE enemy_drops (item TEXT NOT NULL);
                 CREATE TABLE relic_rewards (relic TEXT NOT NULL, reward TEXT NOT NULL);
                 CREATE TABLE regions (node TEXT PRIMARY KEY, mastery_xp INTEGER NOT NULL);
                 CREATE TABLE vendors (key TEXT PRIMARY KEY, rotates INTEGER NOT NULL);
                 CREATE TABLE vendor_offers (
                     vendor TEXT NOT NULL, item TEXT NOT NULL, cost INTEGER, pays TEXT,
                     credits INTEGER, count INTEGER NOT NULL, always INTEGER NOT NULL
                 );

                 INSERT INTO items VALUES
                   ('VoltPrime', 'Volt Prime', 'Вольт Прайм', 'warframe', 'warframe', 1, 1, 0,
                    1, 'carried', 0, 30),
                   ('VoltPrimeSystems', 'Volt Prime Systems', NULL, 'component',
                    'warframe-component', 1, 1, 0, 1, NULL, NULL, NULL),
                   ('VoltPrimeChassis', 'Volt Prime Chassis', NULL, 'component',
                    'warframe-component', 1, 1, 0, 1, NULL, NULL, NULL),
                   ('OrokinCell', 'Orokin Cell', NULL, 'resource', 'resource', 0, NULL, 0,
                    1, NULL, NULL, NULL),
                   ('KuvaKraken', 'Kuva Kraken', 'Кува Кракен', 'secondary', 'secondary', 0,
                    NULL, 0, 1, 'wielded', 13, 40),
                   ('ExaltedBlade', 'Exalted Blade', NULL, 'exalted', 'exalted-weapon', 0,
                    NULL, 0, 1, NULL, NULL, NULL),
                   ('Vitality', 'Vitality', 'Живучесть', 'mod', 'warframe-mod', 0, NULL, 1,
                    1, NULL, NULL, NULL),
                   ('Madurai', 'Madurai Way', NULL, 'mod', 'focus-way', 0, NULL, 0, 1,
                    NULL, NULL, NULL),
                   ('ShwaakPrism', 'Shwaak Prism', NULL, 'modular', 'amp-part', 0, NULL, 0,
                    1, 'wielded', 0, 30),
                   ('ShwaakScaffold', 'Shwaak Scaffold', NULL, 'modular', 'amp-part', 0,
                    NULL, 0, 1, NULL, NULL, NULL),
                   ('OutOfScope', 'Out of scope', NULL, 'primary', 'primary', 0, NULL, 0, 0,
                    'wielded', 0, 30);
                 INSERT INTO item_icons VALUES ('VoltPrime', 'ru', 'volt-prime');
                 INSERT INTO sets VALUES ('volt_prime_set', 'VoltPrime');
                 INSERT INTO recipes VALUES
                   ('VoltPrimeBlueprint', 'VoltPrime', 25000, 259200, 1, 1),
                   ('VoltPrimeSystemsBlueprint', 'VoltPrimeSystems', 15000, 43200, 1, 1),
                   ('VoltPrimeChassisBlueprint', 'VoltPrimeChassis', 15000, 43200, 1, 1),
                   ('OrokinCellBlueprint', 'OrokinCell', 1000, 3600, 1, 1);
                 WITH RECURSIVE place(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM place WHERE n < 10)
                 INSERT INTO item_drops SELECT 'OrokinCell' FROM place;
                 INSERT INTO recipe_requires VALUES
                   ('VoltPrimeBlueprint', 'VoltPrimeSystems', 1),
                   ('VoltPrimeBlueprint', 'VoltPrimeChassis', 1),
                   ('VoltPrimeBlueprint', 'OrokinCell', 1),
                   ('VoltPrimeSystemsBlueprint', 'OrokinCell', 2),
                   ('VoltPrimeChassisBlueprint', 'OrokinCell', 2),
                   ('OrokinCellBlueprint', 'Neurodes', 1);
                 INSERT INTO relic_rewards VALUES
                   ('AxiV8Intact', 'VoltPrimeSystemsBlueprint'),
                   ('AxiV8Radiant', 'VoltPrimeSystemsBlueprint'),
                   ('LithS1Intact', 'FormaBlueprint');
                 INSERT INTO regions VALUES
                   ('SolNode27', 24), ('SolNode801', 0), ('EarthToVenusJunction', 1000);
                 INSERT INTO vendors VALUES ('market', 0), ('teshin', 0), ('baro', 1);
                 INSERT INTO vendor_offers VALUES
                   ('baro', 'VoltPrimeBlueprint', 300, 'PrimeBucks', 100000, 1, 1),
                   ('teshin', 'VoltPrimeChassisBlueprint', 50, 'PathosClamp', NULL, 1, 1),
                   ('market', 'VoltPrimeChassisBlueprint', NULL, NULL, 25000, 1, 1),
                   ('teshin', 'VoltPrimeSystemsBlueprint', 50, 'PathosClamp', NULL, 1, 1);",
            )
            .expect("the catalog tables");
        connection
    }

    #[test]
    fn only_what_gives_mastery_and_is_in_scope_is_listed() {
        let listed = masterable(&catalog(), "ru").expect("the list");

        let keys: Vec<&str> = listed.iter().map(|item| item.item.as_str()).collect();
        assert_eq!(keys, ["KuvaKraken", "ShwaakPrism", "VoltPrime"]);
        assert_eq!(listed[0].mastery, Mastery::Wielded);
        assert_eq!(listed[0].cap, 40);
        assert_eq!(listed[0].required, 13);
        assert_eq!(listed[2].name, "Вольт Прайм");
        assert_eq!(listed[2].image.as_deref(), Some("volt-prime"));
        assert!(listed[2].vaulted);
    }

    #[test]
    fn a_mastery_word_the_build_does_not_know_is_an_error_not_a_gap() {
        let connection = catalog();
        connection
            .execute(
                "UPDATE items SET mastery = 'worn' WHERE unique_name = 'VoltPrime'",
                [],
            )
            .expect("the update");

        assert!(masterable(&connection, "en").is_err());
    }

    #[test]
    fn the_collection_lists_what_is_whole_and_leaves_its_parts_out() {
        let listed = entries(&catalog(), "ru").expect("the collection");

        let keys: Vec<&str> = listed.iter().map(|entry| entry.item.as_str()).collect();
        assert_eq!(keys, ["KuvaKraken", "ShwaakPrism", "Vitality", "VoltPrime"]);
        assert_eq!(listed[3].mastery, Some(Mastery::Carried));
        assert_eq!(listed[2].name, "Живучесть");
        assert!(listed[2].tradable);
        assert_eq!(listed[2].mastery, None);
    }

    #[test]
    fn relics_held_hand_over_what_their_tables_hold() {
        let held = ["AxiV8Radiant".to_string(), "Rubedo".to_string()];

        let rewarded = rewards_of(&catalog(), &held).expect("the rewards");

        assert_eq!(
            rewarded,
            HashSet::from(["VoltPrimeSystemsBlueprint".to_string()])
        );
        assert!(rewards_of(&catalog(), &[]).expect("no rewards").is_empty());
    }

    #[test]
    fn the_chart_holds_only_nodes_that_give_something() {
        let nodes = chart(&catalog()).expect("the chart");

        let tags: Vec<&str> = nodes.iter().map(|node| node.tag.as_str()).collect();
        assert_eq!(tags, ["EarthToVenusJunction", "SolNode27"]);
        assert_eq!(nodes[0].points, 1_000);
    }

    #[test]
    fn parts_are_followed_down_and_resources_are_not() {
        let volt = assembly(&catalog(), "VoltPrime")
            .expect("the tree")
            .expect("a recipe");

        let craft = volt.craft.expect("a craft");
        assert_eq!(craft.blueprint, "VoltPrimeBlueprint");
        assert_eq!(craft.time, 259_200);
        let parts: Vec<(&str, u32, bool)> = craft
            .parts
            .iter()
            .map(|part| (part.item.as_str(), part.count, part.craft.is_some()))
            .collect();
        assert_eq!(
            parts,
            [
                ("OrokinCell", 1, false),
                ("VoltPrimeChassis", 1, true),
                ("VoltPrimeSystems", 1, true),
            ]
        );
    }

    #[test]
    fn a_blueprint_kept_on_sale_is_bought_for_credits_first_and_a_rotation_is_not() {
        let volt = assembly(&catalog(), "VoltPrime")
            .expect("the tree")
            .expect("a recipe");

        let craft = volt.craft.expect("a craft");
        assert_eq!(craft.bought, None);
        let bought = |part: &str| {
            craft
                .parts
                .iter()
                .find(|assembly| assembly.item == part)
                .and_then(|assembly| assembly.craft.as_ref())
                .and_then(|craft| craft.bought.clone())
        };
        assert_eq!(
            bought("VoltPrimeChassis"),
            Some(Purchase {
                pays: None,
                credits: 25_000,
                count: 1,
            })
        );
        assert_eq!(
            bought("VoltPrimeSystems"),
            Some(Purchase {
                pays: Some(("PathosClamp".to_string(), 50)),
                credits: 0,
                count: 1,
            })
        );
    }

    #[test]
    fn a_masterable_recipe_names_each_input_and_the_blueprint_that_builds_it() {
        let recipes = masterable_recipes(&catalog()).expect("the recipes");

        assert_eq!(recipes.len(), 1, "only Volt Prime is built");
        let volt = &recipes["VoltPrime"];
        assert_eq!(volt.blueprint, "VoltPrimeBlueprint");
        let inputs: Vec<(&str, &str, Option<&str>)> = volt
            .parts
            .iter()
            .map(|part| {
                (
                    part.item.as_str(),
                    part.class.as_str(),
                    part.blueprint.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            inputs,
            [
                ("OrokinCell", "resource", Some("OrokinCellBlueprint")),
                (
                    "VoltPrimeChassis",
                    "component",
                    Some("VoltPrimeChassisBlueprint")
                ),
                (
                    "VoltPrimeSystems",
                    "component",
                    Some("VoltPrimeSystemsBlueprint")
                ),
            ]
        );
    }

    #[test]
    fn nothing_builds_what_no_recipe_makes() {
        assert_eq!(assembly(&catalog(), "KuvaKraken").expect("the tree"), None);
    }

    #[test]
    fn a_part_builds_into_its_item_with_the_mastery_that_item_gives() {
        let built = builds(&catalog(), &["VoltPrimeSystems".to_string()]).expect("the builds");

        let volt = &built["VoltPrimeSystems"];
        assert_eq!(volt.item, "VoltPrime");
        assert_eq!(volt.mastery, Some(Mastery::Carried));
        assert_eq!(volt.cap, Some(30));
    }
}
