use std::collections::{HashMap, HashSet};

use median_ipc::{
    Area, Card, Class, Coin, Currency, Drop, GameData, Good, Icon, Kind, Need, Node, Part, Recipe,
    Research, Reward, Sale, SetRow, Taxonomy,
};
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use super::db::Catalog;
use crate::way::{way_of, wire};

/// In-scope items that have a picture in `lang`, ordered by English name; a `limit` of zero
/// returns all of them.
pub fn icons(connection: &Connection, lang: &str, limit: u32) -> Result<Vec<Icon>, String> {
    let mut statement = connection
        .prepare(
            "SELECT CASE ?1 WHEN 'ru' THEN items.name_ru ELSE items.name_en END, icons.image_id
             FROM item_icons AS icons
             JOIN items ON items.unique_name = icons.unique_name
             WHERE icons.lang = ?1 AND items.in_scope = 1
             ORDER BY items.name_en
             LIMIT CASE ?2 WHEN 0 THEN -1 ELSE ?2 END",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map((lang, limit), |row| {
            Ok(Icon {
                name: row.get(0)?,
                image: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Icon>>>()
        .map_err(|e| e.to_string())
}

/// Placeholders for an `IN` list of `count` values, numbered from `first`.
fn placeholders(first: usize, count: usize) -> String {
    (first..first + count)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Item that a blueprint or component builds, with the mastery it gives (`None` when it gives
/// none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub item: String,
    pub mastery: Option<median_core::Mastery>,
    pub cap: Option<u32>,
}

/// Item that each of `items` builds, where the given item is either the blueprint or a component
/// a recipe requires; items that build nothing are omitted.
pub fn builds(connection: &Connection, items: &[String]) -> Result<HashMap<String, Built>, String> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }

    let places = placeholders(1, items.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT items.unique_name, made.unique_name, made.mastery, made.max_level_cap
               FROM items
               LEFT JOIN recipes AS own ON own.blueprint = items.unique_name
               LEFT JOIN recipe_requires AS asks ON asks.item = items.unique_name
               LEFT JOIN recipes AS whole ON whole.blueprint = asks.blueprint
               JOIN items AS made ON made.unique_name = COALESCE(own.result, whole.result)
              WHERE items.unique_name IN ({places})
              GROUP BY items.unique_name"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(items.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?,
                Built {
                    item: row.get(1)?,
                    mastery: row
                        .get::<_, Option<String>>(2)?
                        .as_deref()
                        .and_then(median_core::Mastery::parse),
                    cap: row.get(3)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(|e| e.to_string())
}

/// Names of the given star chart nodes; node keys missing from the catalog are omitted.
pub fn nodes(connection: &Connection, keys: &[String]) -> Result<Vec<Node>, String> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(1, keys.len());

    let mut statement = connection
        .prepare(&format!(
            "SELECT regions.node,
                    COALESCE(regions.name_ru, regions.name, regions.node),
                    COALESCE(locations.name_ru, regions.location, ''),
                    COALESCE(regions.mission_ru, regions.mission_en, ''),
                    REPLACE(COALESCE(regions.mission_en, ''), ' ', '')
             FROM regions
             LEFT JOIN locations ON locations.name = regions.location
             WHERE regions.node IN ({places})"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(keys), |row| {
            Ok(Node {
                node: row.get(0)?,
                name: row.get(1)?,
                planet: row.get(2)?,
                mission: row.get(3)?,
                mission_key: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Node>>>()
        .map_err(|e| e.to_string())
}

/// One item a vendor sells for syndicate standing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stocked {
    pub vendor: String,
    /// Vendor name in the requested language.
    pub vendor_name: String,
    /// Area the vendor is in, in the requested language; `None` for a vendor found on every relay.
    pub place: Option<String>,
    /// Game unique name of the item.
    pub item: String,
    /// warframe.market slug of the item; `None` when the market does not trade it.
    pub slug: Option<String>,
    pub name: String,
    pub image: Option<String>,
    /// Catalog kind of the item; empty when it has none.
    pub kind: String,
    /// Whether players can trade the item.
    pub tradable: bool,
    /// Standing cost of one copy.
    pub cost: u32,
    /// Syndicate rank the vendor requires; `None` when it requires none.
    pub rank: Option<i32>,
}

/// Items the given vendors sell for standing at a cost above zero, in the order of `vendors`.
pub fn stocked(
    connection: &Connection,
    lang: &str,
    vendors: &[&str],
) -> Result<Vec<Stocked>, String> {
    let mut statement = connection
        .prepare_cached(
            "SELECT offers.vendor, items.unique_name, items.slug,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    icons.image_id, COALESCE(items.kind, ''), offers.cost, offers.rank,
                    CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(vendors.name_ru, ''), vendors.name)
                            ELSE vendors.name END,
                    CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(areas.name_ru, ''), areas.name)
                            ELSE areas.name END,
                    COALESCE(items.tradable, 0)
             FROM vendor_offers AS offers
             JOIN items ON items.unique_name = offers.item
             JOIN vendors ON vendors.key = offers.vendor
             LEFT JOIN areas ON areas.key = vendors.area
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = items.unique_name AND icons.lang = ?1
             WHERE offers.vendor = ?2 AND offers.currency = 'Standing' AND offers.cost > 0",
        )
        .map_err(|e| e.to_string())?;

    let mut found = Vec::new();
    for vendor in vendors {
        let rows = statement
            .query_map((lang, vendor), |row| {
                Ok(Stocked {
                    vendor: row.get(0)?,
                    item: row.get(1)?,
                    slug: row.get(2)?,
                    name: row.get(3)?,
                    image: row.get(4)?,
                    kind: row.get(5)?,
                    cost: row.get::<_, i64>(6)?.clamp(0, i64::from(u32::MAX)) as u32,
                    rank: row.get::<_, Option<i64>>(7)?.map(|rank| rank as i32),
                    vendor_name: row.get(8)?,
                    place: row.get(9)?,
                    tradable: row.get::<_, i64>(10)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            found.push(row.map_err(|e| e.to_string())?);
        }
    }
    Ok(found)
}

/// Catalog data of the item or set a warframe.market slug refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub name: String,
    /// Game unique name of the item; for a set, of the item its parts build.
    pub item: String,
    pub image: Option<String>,
    /// Catalog kind name in the requested language; empty when the item has no kind.
    pub category: String,
}

/// Trading group of each item of `items` found in the catalog: `riven`, `relic`, `arcane`,
/// `mod`, `prime`, or empty.
pub fn grouped(connection: &Connection, items: &[String]) -> Result<Vec<(String, String)>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(1, items.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT unique_name,
                    CASE
                      WHEN kind = 'riven'  THEN 'riven'
                      WHEN class = 'relic' THEN 'relic'
                      WHEN class = 'arcane' THEN 'arcane'
                      WHEN class = 'mod'   THEN 'mod'
                      WHEN prime = 1       THEN 'prime'
                      ELSE ''
                    END
             FROM items WHERE unique_name IN ({places})"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(items.iter().cloned()), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

/// Name, unique name, picture and category of the item a warframe.market slug refers to, or of
/// the set when no item has the slug; `None` when neither has it.
pub fn named(connection: &Connection, lang: &str, slug: &str) -> Result<Option<Named>, String> {
    connection
        .query_row(
            "SELECT CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    items.unique_name,
                    icons.image_id,
                    COALESCE(CASE ?1 WHEN 'ru' THEN kinds.name_ru ELSE kinds.name_en END, '')
             FROM items
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = items.unique_name AND icons.lang = ?1
             LEFT JOIN kinds ON kinds.slug = items.kind
             WHERE items.slug = ?2
             LIMIT 1",
            rusqlite::params![lang, slug],
            |row| {
                Ok(Named {
                    name: row.get(0)?,
                    item: row.get(1)?,
                    image: row.get(2)?,
                    category: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .map_or_else(
            || named_set(connection, lang, slug),
            |found| Ok(Some(found)),
        )
}

/// `named` for a set slug, read from the separate `sets` table. Unique name, picture and kind
/// are those of the item the parts build, as the inventory page expects them.
fn named_set(connection: &Connection, lang: &str, slug: &str) -> Result<Option<Named>, String> {
    connection
        .query_row(
            "SELECT CASE ?1 WHEN 'ru' THEN COALESCE(sets.name_ru, sets.name_en)
                            ELSE sets.name_en END,
                    COALESCE(sets.built, ''),
                    icons.image_id,
                    COALESCE(CASE ?1 WHEN 'ru' THEN kinds.name_ru ELSE kinds.name_en END, '')
             FROM sets
             LEFT JOIN items ON items.unique_name = sets.built
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = sets.built AND icons.lang = ?1
             LEFT JOIN kinds ON kinds.slug = items.kind
             WHERE sets.slug = ?2
             LIMIT 1",
            rusqlite::params![lang, slug],
            |row| {
                Ok(Named {
                    name: row.get(0)?,
                    item: row.get(1)?,
                    image: row.get(2)?,
                    category: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
}

/// Catalog match for each name as the game spells it, keyed by folded name: `by_name` over the
/// distinct folds of `names`, run off the caller's thread.
pub async fn told_by_fold(
    catalog: &Catalog,
    lang: &str,
    names: impl IntoIterator<Item = String>,
) -> Result<HashMap<String, Told>, String> {
    let mut folded: Vec<String> = names
        .into_iter()
        .map(|name| median_core::fold(&name))
        .collect();
    folded.sort();
    folded.dedup();
    let asked = lang.to_owned();
    Ok(catalog
        .read(move |connection| by_name(connection, &asked, &folded))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|told| (told.fold.clone(), told))
        .collect())
}

/// Catalog name and unique name of each of the folded names, in `lang`; names without a match
/// are omitted. The game spells names its own way — capitalised, with a bracket after — so a
/// name is matched by its folded form and returned in the catalog's spelling.
///
/// A folded name without an exact match is looked up by its words instead, because the game and
/// the catalog put the same words in a different order — `ЧЕРТЁЖ: Баруук Прайм: Нейрооптика`
/// against `Баруук Прайм: Нейрооптика (Чертеж)`, `Лит G12 Реликвия` against `Реликвия Лит G12`.
pub fn by_name(
    connection: &Connection,
    lang: &str,
    folded: &[String],
) -> Result<Vec<Told>, String> {
    if folded.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(2, folded.len());
    let said = "CASE ?1 WHEN 'ru' THEN COALESCE(fold_ru, fold_en) ELSE fold_en END";
    let mut statement = connection
        .prepare(&format!(
            "SELECT {said},
                    CASE ?1 WHEN 'ru' THEN COALESCE(name_ru, name_en) ELSE name_en END,
                    unique_name
             FROM search WHERE {said} IN ({places})"
        ))
        .map_err(|e| e.to_string())?;

    let arguments = std::iter::once(lang.to_string()).chain(folded.iter().cloned());
    let rows = statement
        .query_map(params_from_iter(arguments), |row| {
            Ok(Told {
                fold: row.get(0)?,
                name: row.get(1)?,
                item: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut found: Vec<Told> = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    let landed: HashSet<String> = found.iter().map(|told| told.fold.clone()).collect();
    let missed: Vec<String> = folded
        .iter()
        .filter(|asked| !landed.contains(*asked))
        .cloned()
        .collect();

    for asked in missed {
        if let Some((name, item)) = by_words(connection, lang, &asked)? {
            found.push(Told {
                fold: asked,
                name,
                item,
            });
        }
    }
    Ok(found)
}

/// Catalog match for one name as the game spells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Told {
    /// Folded name that was looked up.
    pub fold: String,
    /// Catalog name in the requested language.
    pub name: String,
    /// Game unique name of the item.
    pub item: String,
}

/// Catalog name and unique name of the single name made of exactly the words of `folded`;
/// `None` when no name or more than one distinct name matches.
fn by_words(
    connection: &Connection,
    lang: &str,
    folded: &str,
) -> Result<Option<(String, String)>, String> {
    let asked = median_core::words(folded);
    if asked.is_empty() {
        return Ok(None);
    }
    let terms = asked
        .iter()
        .map(|word| format!("\"{word}\""))
        .collect::<Vec<_>>()
        .join(" ");

    let said = "CASE ?1 WHEN 'ru' THEN COALESCE(fold_ru, fold_en) ELSE fold_en END";
    let mut statement = connection
        .prepare(&format!(
            "SELECT {said},
                    CASE ?1 WHEN 'ru' THEN COALESCE(name_ru, name_en) ELSE name_en END,
                    unique_name
             FROM search WHERE search MATCH ?2"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params![lang, terms], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut named: Option<(String, String)> = None;
    for row in rows {
        let (fold, name, item) = row.map_err(|e| e.to_string())?;
        if median_core::words(&fold) != asked {
            continue;
        }
        match &named {
            Some((already, _)) if already != &name => return Ok(None),
            Some(_) => {}
            None => named = Some((name, item)),
        }
    }
    Ok(named)
}

/// Name of `item` in the catalog's other language (English for `ru`, Russian otherwise); `None`
/// when the catalog has no row for it, or no name in that language.
pub fn other_name(
    connection: &Connection,
    lang: &str,
    item: &str,
) -> Result<Option<String>, String> {
    let mut statement = connection
        .prepare(
            "SELECT CASE ?1 WHEN 'ru' THEN items.name_en ELSE items.name_ru END
               FROM items WHERE items.unique_name = ?2",
        )
        .map_err(|e| e.to_string())?;

    statement
        .query_row(rusqlite::params![lang, item], |row| {
            row.get::<_, Option<String>>(0)
        })
        .optional()
        .map(Option::flatten)
        .map_err(|e| e.to_string())
}

/// Display name of `item`: its catalog name, or the unique name itself when `good` is `None`.
pub fn name_of(good: Option<&Good>, item: &str) -> String {
    good.map_or_else(|| item.to_string(), |good| good.name.clone())
}

/// Catalog data of each of `items` in `lang`, in the order given; a unique name missing from the
/// catalog yields a placeholder named by the unique name itself.
pub fn goods(connection: &Connection, lang: &str, items: &[String]) -> Result<Vec<Good>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(2, items.len());

    let mut statement = connection
        .prepare(&format!(
            "SELECT items.unique_name,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    items.kind,
                    icons.image_id,
                    items.slug,
                    items.tradable,
                    items.vaulted,
                    items.prime,
                    items.ducats,
                    made.kind,
                    (SELECT CASE ?1 WHEN 'ru' THEN COALESCE(packs.name_ru, packs.name_en)
                                    ELSE packs.name_en END
                     FROM item_grants AS grants
                     JOIN items AS packs ON packs.unique_name = grants.owner
                     WHERE grants.item = items.unique_name AND grants.how = 'pooled'
                     ORDER BY packs.unique_name LIMIT 1)
             FROM items
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = items.unique_name AND icons.lang = ?1
             LEFT JOIN recipes ON recipes.blueprint = items.unique_name
             LEFT JOIN items AS made ON made.unique_name = recipes.result
             WHERE items.unique_name IN ({places})"
        ))
        .map_err(|e| e.to_string())?;

    let arguments = std::iter::once(lang.to_string()).chain(items.iter().cloned());
    let rows = statement
        .query_map(params_from_iter(arguments), |row| {
            Ok(Good {
                item: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                image: row.get(3)?,
                known: true,
                slug: row.get(4)?,
                tradable: row.get(5)?,
                vaulted: row.get(6)?,
                prime: row.get(7)?,
                ducats: row.get::<_, Option<i64>>(8)?.map(|value| value as u32),
                makes: row.get(9)?,
                mastered: None,
                collection: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let found: HashMap<String, Good> = rows
        .collect::<rusqlite::Result<Vec<Good>>>()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|good| (good.item.clone(), good))
        .collect();

    Ok(items
        .iter()
        .map(|item| found.get(item).cloned().unwrap_or_else(|| unknown(item)))
        .collect())
}

/// Unique name and warframe.market slug of each tradable item of `items`; items missing from the
/// catalog, untradable or without a slug are omitted.
pub fn slugs(connection: &Connection, items: &[String]) -> Result<Vec<(String, String)>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(1, items.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT unique_name, slug FROM items
              WHERE tradable = 1 AND slug IS NOT NULL AND unique_name IN ({places})"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(items.iter()), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

/// warframe.market slug and refinement of each relic of `items`; other items are omitted. The
/// market has one slug per relic and distinguishes refinements by subtype, while the game has a
/// separate item per refinement.
pub fn relics(connection: &Connection, items: &[String]) -> Result<Vec<Refined>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(1, items.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT held.unique_name, items.slug, held.refinement
               FROM relics AS held
               JOIN relics AS intact
                 ON intact.base = held.base AND intact.refinement = 'intact'
               JOIN items ON items.unique_name = intact.unique_name
              WHERE held.unique_name IN ({places}) AND items.slug IS NOT NULL"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(items.iter()), |row| {
            Ok(Refined {
                item: row.get(0)?,
                slug: row.get(1)?,
                refinement: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

/// Relic item of one refinement, with the warframe.market slug of the relic.
pub struct Refined {
    pub item: String,
    pub slug: String,
    pub refinement: String,
}

/// warframe.market slugs of the sets that contain any of `items`, sorted and deduplicated.
pub fn set_slugs(connection: &Connection, items: &[String]) -> Result<Vec<String>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(1, items.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT DISTINCT slug FROM set_members WHERE item IN ({places}) ORDER BY slug"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(items.iter()), |row| row.get(0))
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<String>>>()
        .map_err(|e| e.to_string())
}

/// One part of a set that contains a queried item.
pub struct KitPart {
    /// Queried item whose set this part belongs to.
    pub of: String,
    /// warframe.market slug of the set.
    pub slug: String,
    pub item: String,
    pub image: Option<String>,
    /// Number of copies of this part in one set.
    pub per_set: u32,
}

/// Every part of each set that contains one of `items`, ordered by queried item, then part. An
/// item in no set yields no rows, and many items are in none: Forma, Kuva and other standalone
/// rewards.
pub fn kit(connection: &Connection, lang: &str, items: &[String]) -> Result<Vec<KitPart>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(2, items.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT asked.item, asked.slug, member.item, icons.image_id,
                    COALESCE((SELECT needs.count
                                FROM sets
                                JOIN recipes ON recipes.result = sets.built
                                JOIN recipe_requires AS needs
                                  ON needs.blueprint = recipes.blueprint
                               WHERE sets.slug = asked.slug AND needs.item = member.item), 1)
               FROM set_members AS asked
               JOIN set_members AS member ON member.slug = asked.slug
               LEFT JOIN item_icons AS icons
                 ON icons.unique_name = member.item AND icons.lang = ?1
              WHERE asked.item IN ({places})
              ORDER BY asked.item, member.item"
        ))
        .map_err(|e| e.to_string())?;

    let asking = std::iter::once(lang.to_owned()).chain(items.iter().cloned());
    let rows = statement
        .query_map(params_from_iter(asking), |row| {
            Ok(KitPart {
                of: row.get(0)?,
                slug: row.get(1)?,
                item: row.get(2)?,
                image: row.get(3)?,
                per_set: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<KitPart>>>()
        .map_err(|e| e.to_string())
}

/// Relic families of the relics in `items`, sorted and deduplicated; other items are omitted,
/// and every refinement of one relic gives the same family.
pub fn bases(connection: &Connection, items: &[String]) -> Result<Vec<String>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(1, items.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT DISTINCT base FROM relics WHERE unique_name IN ({places}) ORDER BY base"
        ))
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(params_from_iter(items.iter()), |row| row.get(0))
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<String>>>()
        .map_err(|e| e.to_string())
}

/// One refinement of a relic, with the name, picture, era and vaulted flag of its family.
#[derive(Clone)]
pub struct Grade {
    pub item: String,
    pub base: String,
    pub refinement: String,
    pub name: String,
    pub image: Option<String>,
    /// First word of the relic's English name, lowercased: `lith`, `axi`, `requiem`.
    pub era: String,
    pub vaulted: bool,
}

/// Every refinement of the given relic families. Name, picture and era come from the intact
/// relic, the only refinement warframe.market lists.
pub fn family(connection: &Connection, lang: &str, bases: &[String]) -> Result<Vec<Grade>, String> {
    if bases.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(2, bases.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT grade.unique_name, grade.base, grade.refinement,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    icons.image_id,
                    LOWER(CASE WHEN INSTR(items.name_en, ' ') > 0
                               THEN SUBSTR(items.name_en, 1, INSTR(items.name_en, ' ') - 1)
                               ELSE items.name_en END),
                    COALESCE(items.vaulted, 0)
               FROM relics AS grade
               JOIN relics AS intact
                 ON intact.base = grade.base AND intact.refinement = 'intact'
               JOIN items ON items.unique_name = intact.unique_name
               LEFT JOIN item_icons AS icons
                 ON icons.unique_name = intact.unique_name AND icons.lang = ?1
              WHERE grade.base IN ({places})"
        ))
        .map_err(|e| e.to_string())?;

    let asking = std::iter::once(lang.to_owned()).chain(bases.iter().cloned());
    let rows = statement
        .query_map(params_from_iter(asking), |row| {
            Ok(Grade {
                item: row.get(0)?,
                base: row.get(1)?,
                refinement: row.get(2)?,
                name: row.get(3)?,
                image: row.get(4)?,
                era: row.get(5)?,
                vaulted: row.get::<_, i64>(6)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Grade>>>()
        .map_err(|e| e.to_string())
}

/// One row of a relic's drop table.
#[derive(Clone)]
pub struct Slot {
    pub relic: String,
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    pub rarity: String,
    /// Share of the relic's rewards, in tenths of a percent.
    pub chance: u32,
    /// Number of copies the reward gives; more than one for Forma.
    pub count: u32,
    pub ducats: Option<u32>,
}

/// Drop table rows of the given relics, one row per relic and item with chances summed: the
/// drop tables key a row by rarity, so an item listed under two rarities of one relic has two.
pub fn slots(connection: &Connection, lang: &str, relics: &[String]) -> Result<Vec<Slot>, String> {
    if relics.is_empty() {
        return Ok(Vec::new());
    }

    let places = placeholders(2, relics.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT rewards.relic, items.unique_name,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    MIN(icons.image_id), {RARITY},
                    SUM(rewards.chance), MAX(rewards.count), items.ducats
               FROM relic_rewards AS rewards
               JOIN items ON items.unique_name = rewards.reward
               LEFT JOIN item_icons AS icons
                 ON icons.unique_name = items.unique_name AND icons.lang = ?1
              WHERE rewards.relic IN ({places})
              GROUP BY rewards.relic, items.unique_name"
        ))
        .map_err(|e| e.to_string())?;

    let asking = std::iter::once(lang.to_owned()).chain(relics.iter().cloned());
    let rows = statement
        .query_map(params_from_iter(asking), |row| {
            Ok(Slot {
                relic: row.get(0)?,
                item: row.get(1)?,
                name: row.get(2)?,
                image: row.get(3)?,
                rarity: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                chance: median_ipc::permille(row.get::<_, Option<f64>>(5)?.unwrap_or(0.0)),
                count: row.get::<_, Option<i64>>(6)?.unwrap_or(1).max(1) as u32,
                ducats: row.get::<_, Option<i64>>(7)?.map(|held| held as u32),
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Slot>>>()
        .map_err(|e| e.to_string())
}

/// Unique name and name in `lang` of every item any relic can drop. This is the full dictionary
/// for reading the reward screen: the screen shows only relic rewards, so recognised text is
/// matched against this list and not against the whole catalog.
pub fn reward_names(connection: &Connection, lang: &str) -> Result<Vec<(String, String)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT items.unique_name,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END
               FROM relic_rewards AS rewards
               JOIN items ON items.unique_name = rewards.reward",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map((lang,), |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<(String, String)>>>()
        .map_err(|e| e.to_string())
}

/// FTS5 expression for `query`: every word as a required prefix term; `None` when `query` has
/// no words.
fn expression(query: &str) -> Option<String> {
    let terms: Vec<String> = median_core::fold(query)
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{term}\" *"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

/// Unique names of items whose name matches every word of `query` as a prefix, best match
/// first: a name that starts with the query before one that only contains it, then a shorter
/// name before a longer one. A `limit` of zero returns every match.
pub fn search(
    connection: &Connection,
    lang: &str,
    query: &str,
    limit: u32,
) -> Result<Vec<String>, String> {
    let Some(expression) = expression(query) else {
        return Ok(Vec::new());
    };

    let mut statement = connection
        .prepare(
            "SELECT unique_name
             FROM search
             WHERE search MATCH ?2
             ORDER BY (instr(CASE ?1 WHEN 'ru' THEN COALESCE(fold_ru, fold_en)
                                     ELSE fold_en END, ?3) = 1) DESC,
                      length(CASE ?1 WHEN 'ru' THEN COALESCE(fold_ru, fold_en)
                                     ELSE fold_en END) ASC,
                      unique_name ASC
             LIMIT CASE ?4 WHEN 0 THEN -1 ELSE ?4 END",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map(
            (lang, &expression, median_core::fold(query), limit),
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<String>>>()
        .map_err(|e| e.to_string())
}

/// Placeholder for a unique name missing from the catalog, named by the unique name itself.
fn unknown(item: &str) -> Good {
    Good {
        item: item.to_string(),
        name: item.to_string(),
        kind: String::new(),
        image: None,
        known: false,
        slug: None,
        tradable: None,
        vaulted: None,
        prime: false,
        ducats: None,
        makes: None,
        mastered: None,
        collection: None,
    }
}

/// Unique names of the currencies that are catalog items; other currencies have no picture.
const COINS: [(Currency, &str); 3] = [
    (Currency::Ducats, "/Lotus/Types/Items/MiscItems/PrimeBucks"),
    (Currency::Aya, "/Lotus/Types/Items/MiscItems/SchismKey"),
    (
        Currency::RegalAya,
        "/Lotus/Types/Items/MiscItems/PremiumSchismKey",
    ),
];

/// Names and pictures of the `COINS` currencies; a currency missing from the catalog is named
/// by its unique name and has no picture.
pub fn coins(connection: &Connection, lang: &str) -> Result<Vec<Coin>, String> {
    let keys: Vec<String> = COINS.iter().map(|(_, key)| key.to_string()).collect();
    let known = goods(connection, lang, &keys)?;

    Ok(COINS
        .iter()
        .filter_map(|(currency, key)| {
            let good = known.iter().find(|good| good.item == *key)?;
            Some(Coin {
                currency: currency.clone(),
                name: good.name.clone(),
                image: good.image.clone(),
            })
        })
        .collect())
}

/// Every catalog area, named in `lang`, with the star chart nodes in it.
pub fn areas(connection: &Connection, lang: &str) -> Result<Vec<Area>, String> {
    let mut statement = connection
        .prepare_cached(
            "SELECT areas.key,
                    CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(areas.name_ru, ''), areas.name)
                            ELSE areas.name END,
                    regions.node
               FROM areas
               LEFT JOIN regions ON regions.area = areas.key
              ORDER BY areas.key, regions.node",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([lang], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    let mut out: Vec<Area> = Vec::new();
    for (key, name, node) in rows {
        if out.last().is_none_or(|area| area.key != key) {
            out.push(Area {
                key,
                name,
                nodes: Vec::new(),
            });
        }
        if let (Some(area), Some(node)) = (out.last_mut(), node) {
            area.nodes.push(node);
        }
    }
    Ok(out)
}

/// Full card of `item`: catalog data, sources, relic rewards, recipes and uses; `None` when the
/// catalog has no such item.
pub fn card(connection: &Connection, lang: &str, item: &str) -> Result<Option<Card>, String> {
    let found = goods(connection, lang, &[item.to_string()])?
        .pop()
        .filter(|good| good.known);
    let Some(good) = found else {
        return Ok(None);
    };

    Ok(Some(with_way(Card {
        good,
        sold: sold(connection, lang, item)?,
        inside: rewards(connection, lang, item, Side::Inside)?,
        found: rewards(connection, lang, item, Side::Found)?,
        drops: drops(connection, lang, item)?,
        research: research(connection, lang, item)?,
        recipes: recipes(connection, lang, item)?,
        needed_by: needed_by(connection, lang, item)?,
        way: None,
    })))
}

/// `card` with `way` set to the lowest-effort way of obtaining its item.
fn with_way(card: Card) -> Card {
    Card {
        way: way_of(&card).as_ref().map(wire),
        ..card
    }
}

/// Origin card of each catalog item of `items`, keyed by unique name: the card without relic
/// contents and uses. An item with no way of its own gets the way to its blueprint.
pub fn origins(
    connection: &Connection,
    lang: &str,
    items: &[String],
) -> Result<HashMap<String, Card>, String> {
    let mut out = HashMap::with_capacity(items.len());
    for good in goods(connection, lang, items)? {
        if !good.known {
            continue;
        }
        let item = good.item.clone();
        let mut card = origin(connection, lang, good)?;
        if card.way.is_none() {
            card.way = built_way(connection, lang, &card)?;
        }
        out.insert(item, card);
    }
    Ok(out)
}

/// Origin card of one item (no relic contents, no uses), with its lowest-effort way.
fn origin(connection: &Connection, lang: &str, good: Good) -> Result<Card, String> {
    let item = good.item.clone();
    Ok(with_way(Card {
        sold: sold(connection, lang, &item)?,
        inside: Vec::new(),
        found: rewards(connection, lang, &item, Side::Found)?,
        drops: drops(connection, lang, &item)?,
        research: research(connection, lang, &item)?,
        recipes: recipes(connection, lang, &item)?,
        needed_by: Vec::new(),
        good,
        way: None,
    }))
}

/// Lowest-effort way to obtain a blueprint of an item that is neither sold nor dropped itself,
/// e.g. a weapon obtained through its blueprint or an alloy refined from one; `None` when no
/// blueprint has a way.
fn built_way(
    connection: &Connection,
    lang: &str,
    card: &Card,
) -> Result<Option<median_ipc::Way>, String> {
    let blueprints: Vec<String> = card
        .recipes
        .iter()
        .map(|recipe| recipe.blueprint.clone())
        .filter(|blueprint| *blueprint != card.good.item)
        .collect();
    let mut ways = Vec::new();
    for good in goods(connection, lang, &blueprints)? {
        if good.known {
            ways.extend(way_of(&origin(connection, lang, good)?));
        }
    }
    Ok(median_core::easiest(ways).as_ref().map(wire))
}

fn sold(connection: &Connection, lang: &str, item: &str) -> Result<Vec<Sale>, String> {
    let mut statement = connection
        .prepare_cached(
            "SELECT CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(vendors.name_ru, ''), vendors.name)
                            ELSE vendors.name END,
                    offers.cost, offers.currency, offers.credits, offers.rank, vendors.area,
                    vendors.rotates,
                    CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(areas.name_ru, ''), areas.name)
                            ELSE areas.name END,
                    CASE WHEN offers.item = ?2 THEN NULL
                         WHEN ?1 = 'ru' THEN COALESCE(packs.name_ru, packs.name_en)
                         ELSE packs.name_en END,
                    CASE ?1 WHEN 'ru' THEN COALESCE(coins.name_ru, coins.name_en)
                            ELSE coins.name_en END
             FROM vendor_offers AS offers
             JOIN vendors ON vendors.key = offers.vendor
             LEFT JOIN areas ON areas.key = vendors.area
             LEFT JOIN items AS packs ON packs.unique_name = offers.item
             LEFT JOIN items AS coins ON coins.unique_name = offers.pays
             WHERE offers.item = ?2
                OR offers.item IN (SELECT owner FROM item_grants
                                   WHERE item = ?2 AND how = 'pooled')
             ORDER BY offers.cost DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map((lang, item), |row| {
            Ok(Sale {
                vendor: row.get(0)?,
                cost: row.get::<_, Option<i64>>(1)?.map(|value| value as u32),
                currency: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                credits: row.get::<_, Option<i64>>(3)?.map(|value| value as u32),
                rank: row.get::<_, Option<i64>>(4)?.map(|value| value as u32),
                area: row.get(5)?,
                rotates: row.get(6)?,
                place: row.get(7)?,
                pack: row.get(8)?,
                coin: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Sale>>>()
        .map_err(|e| e.to_string())
}

/// Recipes in which `item` is either the blueprint or the result, ordered by blueprint.
fn recipes(connection: &Connection, lang: &str, item: &str) -> Result<Vec<Recipe>, String> {
    let mut statement = connection
        .prepare_cached(
            "SELECT blueprint, build_price, build_time, rush_price
             FROM recipes
             WHERE blueprint = ?1 OR result = ?1
             ORDER BY blueprint",
        )
        .map_err(|e| e.to_string())?;

    let found = statement
        .query_map([item], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?.map(|value| value as u32),
                row.get::<_, Option<i64>>(2)?.map(|value| value as u32),
                row.get::<_, Option<i64>>(3)?.map(|value| value as u32),
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;

    found
        .into_iter()
        .map(|(blueprint, credits, time, rush)| {
            Ok(Recipe {
                parts: parts(connection, lang, &blueprint)?,
                blueprint,
                credits,
                time,
                rush,
            })
        })
        .collect()
}

/// Items built by recipes that require `item`, with the count each recipe requires.
fn needed_by(connection: &Connection, lang: &str, item: &str) -> Result<Vec<Need>, String> {
    let mut statement = connection
        .prepare(
            "SELECT recipes.result,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    icons.image_id,
                    requires.count
             FROM recipe_requires AS requires
             JOIN recipes ON recipes.blueprint = requires.blueprint
             JOIN items ON items.unique_name = recipes.result
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = items.unique_name AND icons.lang = ?1
             WHERE requires.item = ?2
             ORDER BY items.unique_name",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map((lang, item), |row| {
            Ok(Need {
                item: row.get(0)?,
                name: row.get(1)?,
                image: row.get(2)?,
                count: row.get::<_, i64>(3)? as u32,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Need>>>()
        .map_err(|e| e.to_string())
}

/// Inputs of one blueprint, largest count first.
fn parts(connection: &Connection, lang: &str, blueprint: &str) -> Result<Vec<Part>, String> {
    let mut statement = connection
        .prepare(
            "SELECT requires.item,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    icons.image_id,
                    requires.count,
                    items.class
             FROM recipe_requires AS requires
             JOIN items ON items.unique_name = requires.item
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = items.unique_name AND icons.lang = ?1
             WHERE requires.blueprint = ?2
             ORDER BY requires.count DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map((lang, blueprint), |row| {
            Ok(Part {
                item: row.get(0)?,
                name: row.get(1)?,
                image: row.get(2)?,
                count: row.get::<_, i64>(3)? as u32,
                class: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Part>>>()
        .map_err(|e| e.to_string())
}

/// Clan lab research entries of the blueprint `item`; empty when it is not researched.
fn research(connection: &Connection, lang: &str, item: &str) -> Result<Vec<Research>, String> {
    let mut statement = connection
        .prepare_cached(
            "SELECT CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(labs.name_ru, ''), labs.name)
                            ELSE labs.name END,
                    research.credits, research.time, research.affinity, research.prereq
             FROM research
             LEFT JOIN labs ON labs.key = research.lab
             WHERE research.blueprint = ?2",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map((lang, item), |row| {
            Ok(Research {
                lab: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                credits: row.get::<_, i64>(1)? as u32,
                time: row.get::<_, i64>(2)? as u32,
                affinity: row.get::<_, i64>(3)? as u32,
                prereq: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Research>>>()
        .map_err(|e| e.to_string())
}

/// Direction of a relic-reward lookup.
enum Side {
    /// Rewards of the relic `item`.
    Inside,
    /// Relics that drop `item`.
    Found,
}

/// SQL for the most common rarity in the group, spelled as in the IPC payload: `common`,
/// `uncommon` or `rare`.
const RARITY: &str = "CASE MIN(CASE rewards.rarity WHEN 'COMMON' THEN 1
                                                   WHEN 'UNCOMMON' THEN 2
                                                   ELSE 3 END)
                        WHEN 1 THEN 'common' WHEN 2 THEN 'uncommon' ELSE 'rare' END";

fn rewards(
    connection: &Connection,
    lang: &str,
    item: &str,
    side: Side,
) -> Result<Vec<Reward>, String> {
    // One row per item with chances summed: the drop tables key a row by rarity, so an item
    // listed under two rarities of the same relic has two rows.
    let sql = match side {
        Side::Inside => format!(
            "SELECT items.unique_name,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END,
                    MIN(icons.image_id), {RARITY}, NULL,
                    SUM(rewards.chance), MAX(rewards.count)
             FROM relic_rewards AS rewards
             JOIN items ON items.unique_name = rewards.reward
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = items.unique_name AND icons.lang = ?1
             WHERE rewards.relic = ?2
             GROUP BY items.unique_name"
        ),
        // One row per refinement, not per relic: this lookup exists to show the item's chance
        // at each refinement.
        Side::Found => format!(
            "SELECT items.unique_name,
                    CASE ?1 WHEN 'ru' THEN COALESCE(items.name_ru, items.name_en)
                            ELSE items.name_en END AS title,
                    MIN(icons.image_id), {RARITY}, MIN(relics.refinement),
                    SUM(rewards.chance) AS odds, MAX(rewards.count)
             FROM relic_rewards AS rewards
             JOIN items ON items.unique_name = rewards.relic
             LEFT JOIN relics ON relics.unique_name = rewards.relic
             LEFT JOIN item_icons AS icons
               ON icons.unique_name = items.unique_name AND icons.lang = ?1
             WHERE rewards.reward = ?2
             GROUP BY items.unique_name
             ORDER BY title, odds"
        ),
    };

    let mut statement = connection.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map((lang, item), |row| {
            Ok(Reward {
                item: row.get(0)?,
                name: row.get(1)?,
                image: row.get(2)?,
                rarity: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                refinement: row.get(4)?,
                chance: median_ipc::permille(row.get::<_, Option<f64>>(5)?.unwrap_or(0.0)),
                count: row.get::<_, Option<i64>>(6)?.unwrap_or(1).max(1) as u32,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<Reward>>>()
        .map_err(|e| e.to_string())
}

/// Every drop source of `item`, highest chance first: places of every kind and the enemies that
/// drop it. For a relic, the sources of all its refinements are included, as each drops apart.
/// The star chart is joined in through `places.region`, which matches far more nodes than
/// the place name does.
fn drops(connection: &Connection, lang: &str, item: &str) -> Result<Vec<Drop>, String> {
    let mut statement = connection
        .prepare_cached(
            "WITH kin(item) AS (
                 SELECT ?2
                 UNION
                 SELECT others.unique_name
                 FROM relics AS own
                 JOIN relics AS others ON others.base = own.base
                 WHERE own.unique_name = ?2
             )
             SELECT places.kind, COALESCE(places.event, 0),
                    CASE ?1 WHEN 'ru'
                      THEN COALESCE(NULLIF(bounty.settlement_ru, ''), bounty.settlement,
                                    NULLIF(regions.name_ru, ''), NULLIF(regions.name, ''),
                                    NULLIF(places.name_ru, ''), places.name)
                      ELSE COALESCE(bounty.settlement, NULLIF(regions.name, ''), places.name) END,
                    CASE ?1 WHEN 'ru' THEN COALESCE(locations.name_ru, locations.name)
                            ELSE locations.name END,
                    CASE ?1 WHEN 'ru'
                      THEN COALESCE(NULLIF(bounty.activity_ru, ''), bounty.activity,
                                    regions.mission_ru, regions.mission_en)
                      ELSE COALESCE(bounty.activity, regions.mission_en) END,
                    COALESCE(bounty.min_level, regions.min_level),
                    COALESCE(bounty.max_level, regions.max_level),
                    drops.rotation, drops.stage,
                    LOWER(REPLACE(drops.rarity, ' ', '-')),
                    drops.chance AS chance, drops.count,
                    CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(bounty.giver_ru, ''), bounty.giver)
                            ELSE bounty.giver END,
                    places.area,
                    NULL,
                    relics.refinement
             FROM item_drops AS drops
             JOIN places ON places.name = drops.place
             LEFT JOIN regions ON regions.node = places.region
             LEFT JOIN locations ON locations.name = places.location
             LEFT JOIN bounty_places AS bounty ON bounty.place = drops.place
             LEFT JOIN relics ON relics.unique_name = drops.item
             WHERE drops.item IN (SELECT item FROM kin)

             UNION ALL

             SELECT 'enemy', 0,
                    CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(enemies.name_ru, ''), kills.enemy)
                            ELSE kills.enemy END,
                    NULL, NULL,
                    kills.min_level, kills.max_level,
                    NULL, NULL,
                    LOWER(REPLACE(kills.rarity, ' ', '-')),
                    kills.chance * COALESCE(kills.table_chance, 1.0), kills.count, NULL, NULL,
                    kills.enemy, relics.refinement
             FROM enemy_drops AS kills
             LEFT JOIN enemies ON enemies.name = kills.enemy
             LEFT JOIN relics ON relics.unique_name = kills.item
             WHERE kills.item IN (SELECT item FROM kin)

             ORDER BY chance DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map((lang, item), |row| {
            let from = row.get::<_, Option<i64>>(5)?;
            let to = row.get::<_, Option<i64>>(6)?;
            // A hub has no enemies; the catalog stores that as a level range of 0 to 0.
            let levelled = matches!((from, to), (Some(from), Some(to)) if from > 0 || to > 0);

            let drop = Drop {
                kind: row.get(0)?,
                event: row.get::<_, i64>(1)? != 0,
                place: row.get(2)?,
                planet: row.get(3)?,
                mission: row.get(4)?,
                giver: row.get(12)?,
                level_from: levelled.then(|| from.unwrap_or(0) as u32),
                level_to: levelled.then(|| to.unwrap_or(0) as u32),
                rotation: row.get(7)?,
                stage: row.get(8)?,
                rarity: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                chance: median_ipc::permille(row.get::<_, Option<f64>>(10)?.unwrap_or(0.0)),
                count: row.get::<_, Option<i64>>(11)?.map(|value| value as u32),
                refinement: row.get(15)?,
                area: row.get(13)?,
                haunts: Vec::new(),
            };
            Ok((drop, row.get::<_, Option<String>>(14)?))
        })
        .map_err(|e| e.to_string())?;

    let found = rows
        .collect::<rusqlite::Result<Vec<(Drop, Option<String>)>>>()
        .map_err(|e| e.to_string())?;
    let enemies: Vec<String> = found
        .iter()
        .filter_map(|(_, enemy)| enemy.clone())
        .collect();
    let mut haunted = haunts(connection, lang, &enemies)?;
    Ok(found
        .into_iter()
        .map(|(mut drop, enemy)| {
            if let Some(planets) = enemy.and_then(|enemy| haunted.remove(&enemy)) {
                drop.haunts = planets;
            }
            drop
        })
        .collect())
}

/// Planets where each of `enemies` appears, ordered by its highest share of a node's enemies.
fn haunts(
    connection: &Connection,
    lang: &str,
    enemies: &[String],
) -> Result<HashMap<String, Vec<String>>, String> {
    if enemies.is_empty() {
        return Ok(HashMap::new());
    }
    let places = placeholders(2, enemies.len());
    let mut statement = connection
        .prepare(&format!(
            "SELECT haunts.enemy,
                    CASE ?1 WHEN 'ru' THEN COALESCE(NULLIF(locations.name_ru, ''), regions.location)
                            ELSE regions.location END AS planet,
                    MAX(haunts.share) AS share
             FROM region_enemies AS haunts
             JOIN regions ON regions.node = haunts.node
             LEFT JOIN locations ON locations.name = regions.location
             WHERE haunts.enemy IN ({places})
             GROUP BY haunts.enemy, planet
             ORDER BY share DESC, planet"
        ))
        .map_err(|e| e.to_string())?;
    let arguments = std::iter::once(lang.to_owned()).chain(enemies.iter().cloned());
    let rows = statement
        .query_map(params_from_iter(arguments), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut haunted: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (enemy, planet) = row.map_err(|e| e.to_string())?;
        haunted.entry(enemy).or_default().push(planet);
    }
    Ok(haunted)
}

/// Version, fetch time and game build of the open game data.
pub fn game_data(connection: &Connection) -> Result<GameData, String> {
    let value = |key: &str| -> Result<Option<String>, String> {
        connection
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())
    };
    Ok(GameData {
        version: value("version")?.unwrap_or_default(),
        fetched: value("fetched_ms")?
            .and_then(|ms| ms.parse::<i64>().ok())
            .and_then(|ms| u32::try_from(ms / 1000).ok()),
        game_build: value("game.build")?,
    })
}

/// Item classes and the kinds within each class, in catalog order, named in `lang`.
pub fn taxonomy(connection: &Connection, lang: &str) -> Result<Taxonomy, String> {
    let mut grouped = connection
        .prepare(
            "SELECT slug, CASE ?1 WHEN 'ru' THEN name_ru ELSE name_en END, ord
             FROM classes
             ORDER BY ord",
        )
        .map_err(|e| e.to_string())?;

    let classes = grouped
        .query_map([lang], |row| {
            Ok(Class {
                slug: row.get(0)?,
                name: row.get(1)?,
                ord: row.get::<_, i64>(2)? as u32,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<Class>>>()
        .map_err(|e| e.to_string())?;

    let mut sorted = connection
        .prepare(
            "SELECT slug, class, CASE ?1 WHEN 'ru' THEN name_ru ELSE name_en END, ord
             FROM kinds
             ORDER BY class, ord",
        )
        .map_err(|e| e.to_string())?;

    let kinds = sorted
        .query_map([lang], |row| {
            Ok(Kind {
                slug: row.get(0)?,
                class: row.get(1)?,
                name: row.get(2)?,
                ord: row.get::<_, i64>(3)? as u32,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<Kind>>>()
        .map_err(|e| e.to_string())?;

    Ok(Taxonomy { classes, kinds })
}

/// Every set in the catalog, named in `lang`, with its parts and the count of each per set.
pub fn sets(connection: &Connection, lang: &str) -> Result<Vec<SetRow>, String> {
    let mut statement = connection
        .prepare(
            "SELECT sets.slug,
                    CASE ?1 WHEN 'ru' THEN COALESCE(sets.name_ru, sets.name_en)
                            ELSE sets.name_en END,
                    sets.ducats,
                    sets.vaulted,
                    sets.built,
                    members.item,
                    COALESCE((SELECT needs.count
                                FROM recipes
                                JOIN recipe_requires AS needs
                                  ON needs.blueprint = recipes.blueprint
                               WHERE recipes.result = sets.built AND needs.item = members.item), 1)
             FROM sets
             LEFT JOIN set_members AS members ON members.slug = sets.slug
             ORDER BY sets.slug, members.item",
        )
        .map_err(|e| e.to_string())?;

    let rows = statement
        .query_map([lang], |row| {
            let set = SetRow {
                slug: row.get(0)?,
                name: row.get(1)?,
                ducats: row.get::<_, Option<i64>>(2)?.map(|value| value as u32),
                vaulted: row.get(3)?,
                built: row.get(4)?,
                members: Vec::new(),
                per_set: Vec::new(),
            };
            Ok((set, row.get::<_, Option<String>>(5)?, row.get::<_, u32>(6)?))
        })
        .map_err(|e| e.to_string())?;

    let mut found: Vec<SetRow> = Vec::new();
    for row in rows {
        let (set, member, per_set) = row.map_err(|e| e.to_string())?;
        if found.last().is_none_or(|last| last.slug != set.slug) {
            found.push(set);
        }
        if let Some(item) = member {
            let kit = found.last_mut().expect("the set the member belongs to");
            kit.members.push(item);
            kit.per_set.push(per_set);
        }
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A catalog `meta` table holding these pairs.
    fn meta(pairs: &[(&str, &str)]) -> Connection {
        let connection = Connection::open_in_memory().expect("a catalog in memory");
        connection
            .execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .expect("the meta table");
        for (key, value) in pairs {
            connection
                .execute("INSERT INTO meta VALUES (?1, ?2)", [key, value])
                .expect("a meta row");
        }
        connection
    }

    #[test]
    fn the_game_data_tells_its_version_date_and_game_build() {
        let connection = meta(&[
            ("version", "19.4.1"),
            ("fetched_ms", "1793312824780"),
            ("game.build", "2026.10.28"),
        ]);

        assert_eq!(
            game_data(&connection).expect("read"),
            GameData {
                version: "19.4.1".into(),
                fetched: Some(1_793_312_824),
                game_build: Some("2026.10.28".into()),
            }
        );
    }

    #[test]
    fn data_that_does_not_say_when_it_was_made_has_no_date() {
        let connection = meta(&[("version", "19.4.1")]);

        let data = game_data(&connection).expect("read");
        assert_eq!((data.fetched, data.game_build), (None, None));
    }

    /// A catalog holding only the tables these queries read.
    fn catalog() -> Connection {
        let connection = Connection::open_in_memory().expect("a catalog in memory");
        connection
            .execute_batch(
                "CREATE TABLE items (
                     unique_name TEXT PRIMARY KEY,
                     name_en TEXT NOT NULL,
                     name_ru TEXT,
                     kind TEXT NOT NULL,
                     slug TEXT,
                     tradable INTEGER,
                     vaulted INTEGER,
                     prime INTEGER NOT NULL,
                     ducats INTEGER
                 );
                 CREATE TABLE item_icons (
                     unique_name TEXT NOT NULL,
                     lang TEXT NOT NULL,
                     image_id TEXT NOT NULL
                 );
                 CREATE TABLE recipes (
                     blueprint TEXT PRIMARY KEY,
                     result TEXT NOT NULL,
                     build_price INTEGER,
                     build_time INTEGER,
                     rush_price INTEGER
                 );
                 CREATE TABLE recipe_requires (
                     blueprint TEXT NOT NULL,
                     item TEXT NOT NULL,
                     count INTEGER NOT NULL
                 );
                 CREATE TABLE sets (
                     slug TEXT PRIMARY KEY,
                     name_en TEXT NOT NULL,
                     name_ru TEXT,
                     ducats INTEGER,
                     vaulted INTEGER,
                     built TEXT
                 );
                 CREATE TABLE set_members (slug TEXT NOT NULL, item TEXT NOT NULL);
                 CREATE TABLE places (
                     name TEXT PRIMARY KEY,
                     name_ru TEXT,
                     kind TEXT NOT NULL,
                     region TEXT,
                     location TEXT,
                     event INTEGER,
                     area TEXT
                 );
                 CREATE TABLE regions (
                     node TEXT PRIMARY KEY,
                     name TEXT NOT NULL,
                     name_ru TEXT,
                     mission_en TEXT,
                     mission_ru TEXT,
                     min_level INTEGER,
                     max_level INTEGER,
                     area TEXT
                 );
                 CREATE TABLE areas (key TEXT PRIMARY KEY, name TEXT NOT NULL, name_ru TEXT);
                 CREATE TABLE locations (name TEXT PRIMARY KEY, name_ru TEXT);
                 CREATE TABLE bounty_places (
                   place TEXT PRIMARY KEY, settlement TEXT NOT NULL, settlement_ru TEXT,
                   giver TEXT, giver_ru TEXT, min_level INTEGER NOT NULL,
                   max_level INTEGER NOT NULL, activity TEXT NOT NULL, activity_ru TEXT
                 );
                 CREATE TABLE item_drops (
                     place TEXT NOT NULL,
                     item TEXT NOT NULL,
                     rotation TEXT,
                     stage TEXT,
                     rarity TEXT NOT NULL,
                     chance REAL NOT NULL,
                     count INTEGER
                 );
                 CREATE TABLE vendors (
                     key TEXT PRIMARY KEY,
                     name TEXT NOT NULL,
                     name_ru TEXT,
                     kind TEXT,
                     area TEXT,
                     rotates INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE vendor_offers (
                     vendor TEXT NOT NULL,
                     item TEXT NOT NULL,
                     cost INTEGER,
                     currency TEXT,
                     credits INTEGER,
                     rank INTEGER
                 );
                 CREATE TABLE item_grants (
                     owner TEXT NOT NULL,
                     item TEXT NOT NULL,
                     how TEXT NOT NULL
                 );
                 CREATE TABLE relics (
                     unique_name TEXT PRIMARY KEY,
                     base TEXT NOT NULL,
                     refinement TEXT NOT NULL
                 );
                 CREATE TABLE relic_rewards (
                     relic TEXT NOT NULL,
                     reward TEXT NOT NULL,
                     rarity TEXT NOT NULL,
                     count INTEGER NOT NULL,
                     chance REAL
                 );
                 CREATE TABLE labs (
                     key TEXT PRIMARY KEY,
                     name TEXT NOT NULL,
                     name_ru TEXT,
                     faction TEXT
                 );
                 CREATE TABLE research (
                     lab TEXT NOT NULL,
                     blueprint TEXT NOT NULL,
                     credits INTEGER NOT NULL,
                     time INTEGER NOT NULL,
                     affinity INTEGER NOT NULL,
                     prereq TEXT
                 );
                 CREATE TABLE enemies (name TEXT PRIMARY KEY, name_ru TEXT);
                 CREATE TABLE enemy_drops (
                     enemy TEXT NOT NULL,
                     item TEXT NOT NULL,
                     rarity TEXT NOT NULL,
                     chance REAL NOT NULL,
                     table_chance REAL,
                     min_level INTEGER,
                     max_level INTEGER,
                     count INTEGER
                 );
                 CREATE VIRTUAL TABLE search USING fts5(
                    unique_name UNINDEXED,
                    name_en UNINDEXED,
                    name_ru UNINDEXED,
                    fold_en,
                    fold_ru,
                    tokenize = 'unicode61 remove_diacritics 2'
                 );

                 INSERT INTO items VALUES
                   ('/Lotus/Volt/Systems', 'Volt Prime Systems', 'Вольт Прайм: Система',
                    'warframe-component', 'volt_prime_systems', 1, 1, 1, 15),
                   ('/Lotus/Volt/Suit', 'Volt Prime', 'Вольт Прайм',
                    'warframe', NULL, 0, NULL, 1, NULL),
                   ('/Lotus/Recipes/Lens', 'Eidolon Lens Blueprint', 'Эйдолонская линза (Чертеж)',
                    'blueprint', 'eidolon_lens_blueprint', 1, 0, 0, NULL),
                   ('/Lotus/Items/Lens', 'Eidolon Lens', 'Эйдолонская линза',
                    'focus-lens', NULL, 0, NULL, 0, NULL),
                   ('/Lotus/Projections/LithT8Bronze', 'Lith T8 Relic', 'Реликвия Лит T8',
                    'relic', NULL, 1, NULL, 0, NULL),
                   ('/Lotus/Projections/LithT8Platinum', 'Lith T8 Relic', 'Реликвия Лит T8',
                    'relic', NULL, 1, NULL, 0, NULL),
                   ('/Lotus/Recipes/Forma', 'Forma Blueprint', 'Форма (Чертеж)',
                    'blueprint', NULL, 0, NULL, 0, NULL);
                 ALTER TABLE items ADD COLUMN class TEXT NOT NULL DEFAULT 'component';
                 INSERT INTO item_icons VALUES ('/Lotus/Volt/Systems', 'ru', 'a1b2');
                 INSERT INTO recipes VALUES
                   ('/Lotus/Recipes/Lens', '/Lotus/Items/Lens', 15000, 43200, 25);
                 INSERT INTO recipe_requires VALUES
                   ('/Lotus/Recipes/Lens', '/Lotus/Volt/Systems', 2),
                   ('/Lotus/Recipes/Lens', '/Lotus/Volt/Suit', 1);

                 INSERT INTO sets VALUES
                   ('volt_prime_set', 'Volt Prime Set', 'Вольт Прайм: Комплект', 45, 1,
                    '/Lotus/Volt/Suit'),
                   ('lonely_set', 'Lonely Set', 'Одинокий комплект', NULL, NULL, NULL);
                 INSERT INTO set_members VALUES
                   ('volt_prime_set', '/Lotus/Volt/Systems'),
                   ('volt_prime_set', '/Lotus/Volt/Suit');

                 INSERT INTO locations VALUES ('Eris', 'Эрида'), ('Lua', NULL);
                 INSERT INTO bounty_places VALUES
                   ('Level 5 - 15 Cetus Bounty', 'Cetus', 'Цетус', 'Konzu', 'Конзу',
                    5, 15, 'Cetus Bounty', 'Заказы Цетуса');
                 INSERT INTO regions VALUES
                   ('SolNode1', 'Oestrus', '', 'Hive', 'Сбор', 34, 38, NULL),
                   ('SolNode2', 'Strata Relay', 'Реле Страта', 'Hub', 'Хаб', 0, 0, NULL),
                   ('CetusHub4', 'Cetus', 'Цетус', 'Hub', 'Хаб', 0, 0, 'cetus'),
                   ('SolNode228', 'Plains of Eidolon', 'Равнины Эйдолона', 'Free Roam',
                    'Открытый мир', 0, 0, 'cetus');
                 INSERT INTO areas VALUES ('cetus', 'Cetus', 'Цетус'), ('duviri', 'Duviri', NULL);
                 INSERT INTO places VALUES
                   ('Eris/Oestrus (Hive)', NULL, 'node', 'SolNode1', 'Eris', 0, NULL),
                   ('Lua/Strata (Hub)', NULL, 'node', 'SolNode2', 'Lua', 0, NULL),
                   ('Event: Eris/Cyath (Spy)', NULL, 'node', NULL, 'Eris', 1, NULL),
                   ('Level 5 - 15 Cetus Bounty', NULL, 'bounty', NULL, NULL, NULL, 'cetus');
                 INSERT INTO item_drops VALUES
                   ('Eris/Oestrus (Hive)', '/Lotus/Volt/Systems', 'C', NULL,
                    'Uncommon', 0.1429, NULL),
                   ('Lua/Strata (Hub)', '/Lotus/Volt/Systems', NULL, NULL,
                    'Ultra Rare', 0.0149, 3),
                   ('Level 5 - 15 Cetus Bounty', '/Lotus/Volt/Systems', 'B', 'Final Stage',
                    'Rare', 0.3056, NULL),
                   ('Event: Eris/Cyath (Spy)', '/Lotus/Volt/Systems', 'C', NULL,
                    'Rare', 0.086, NULL);
                 INSERT INTO relics VALUES
                   ('/Lotus/Projections/LithT8Bronze', 'LithT8', 'intact'),
                   ('/Lotus/Projections/LithT8Platinum', 'LithT8', 'radiant');
                 INSERT INTO relic_rewards VALUES
                   ('/Lotus/Projections/LithT8Bronze', '/Lotus/Volt/Systems', 'RARE', 1, 0.02),
                   ('/Lotus/Projections/LithT8Platinum', '/Lotus/Volt/Systems', 'RARE', 1, 0.1),
                   ('/Lotus/Projections/LithT8Platinum', '/Lotus/Recipes/Forma', 'COMMON', 2, 0.2),
                   ('/Lotus/Projections/LithT8Platinum', '/Lotus/Recipes/Forma',
                    'UNCOMMON', 2, 0.17);
                 INSERT INTO labs VALUES ('Orokin', 'Orokin Lab', 'Лаборатория Орокин', 'Orokin');
                 INSERT INTO research VALUES
                   ('Orokin', '/Lotus/Recipes/Lens', 10000, 259200, 3000, 'Amprex');
                 INSERT INTO vendors VALUES
                   ('new-loka', 'New Loka', '', 'Syndicate', NULL, 0),
                   ('baro', 'Baro Ki''Teer', 'Баро Ки''Тир', 'Store', NULL, 1);
                 INSERT INTO vendor_offers VALUES
                   ('new-loka', '/Lotus/Volt/Systems', 25000, 'Standing', NULL, 5),
                   ('baro', '/Lotus/Volt/Systems', 300, 'Ducats', 150000, NULL),
                   ('loid', '/Lotus/Packs/Eidolon', 200, 'Vosfor', NULL, NULL);
                 INSERT INTO vendors VALUES ('loid', 'Loid', 'Лойд', 'Store', 'cetus', 0);
                 INSERT INTO items (unique_name, name_en, name_ru, kind, slug, tradable, vaulted,
                                    prime, ducats) VALUES
                   ('/Lotus/Packs/Eidolon', 'Eidolon Arcane Collection',
                    'Коллекция мистификаторов: Эйдолон', 'arcane-collection', NULL, NULL, NULL,
                    0, NULL),
                   ('/Lotus/Arcanes/Grace', 'Arcane Grace', 'Мистическая благодать', 'arcane',
                    'arcane_grace', 1, NULL, 0, NULL);
                 INSERT INTO item_grants VALUES
                   ('/Lotus/Packs/Eidolon', '/Lotus/Arcanes/Grace', 'pooled');
                 ALTER TABLE vendor_offers ADD COLUMN pays TEXT;
                 UPDATE vendor_offers SET pays = '/Lotus/Coins/Vosfor' WHERE vendor = 'loid';
                 INSERT INTO items (unique_name, name_en, name_ru, kind, prime) VALUES
                   ('/Lotus/Coins/Vosfor', 'Vosfor', 'Восфор', 'currency', 0);
                 INSERT INTO enemies VALUES ('Axio Shockwave Moa', 'Ударный МОА Аксио'),
                                            ('Breacher Moa', NULL);
                 INSERT INTO enemy_drops VALUES
                   ('Axio Shockwave Moa', '/Lotus/Volt/Systems', 'Uncommon', 0.2, NULL, 30, 40, NULL),
                   ('Breacher Moa', '/Lotus/Volt/Systems', 'Rare', 0.5, 0.1, NULL, NULL, NULL);
                 ALTER TABLE regions ADD COLUMN location TEXT;
                 UPDATE regions SET location = 'Eris' WHERE node = 'SolNode1';
                 UPDATE regions SET location = 'Lua' WHERE node = 'SolNode2';
                 CREATE TABLE region_enemies (
                     node TEXT NOT NULL, enemy TEXT NOT NULL, share REAL NOT NULL,
                     tier INTEGER NOT NULL
                 );
                 INSERT INTO region_enemies VALUES
                   ('SolNode2', 'Breacher Moa', 0.1, 0),
                   ('SolNode1', 'Breacher Moa', 0.3, 0);

                 INSERT INTO search VALUES
                   ('/Lotus/Volt/Systems', 'Volt Prime Systems', 'Вольт Прайм: Система',
                    'volt prime systems', 'вольт прайм: система'),
                   ('/Lotus/Volt/Suit', 'Volt Prime', 'Вольт Прайм',
                    'volt prime', 'вольт прайм'),
                   ('/Lotus/Conductor', 'Conductor', 'Дирижёр',
                    'conductor', 'дирижер'),
                   ('/Lotus/Baruuk/NeuropticsBp', 'Baruuk Prime Neuroptics Blueprint',
                    'Баруук Прайм: Нейрооптика (Чертеж)',
                    'baruuk prime neuroptics blueprint', 'баруук прайм: нейрооптика (чертеж)'),
                   ('/Lotus/Relics/LithG12', 'Lith G12 Relic', 'Реликвия Лит G12',
                    'lith g12 relic', 'реликвия лит g12'),
                   ('/Lotus/Glyphs/SunMoon', 'Sun and Moon Glyph', 'Глиф: Луна и Солнце',
                    'sun and moon glyph', 'глиф: луна и солнце'),
                   ('/Lotus/Glyphs/MoonSun', 'Moon and Sun Glyph', 'Глиф: Солнце и Луна',
                    'moon and sun glyph', 'глиф: солнце и луна');",
            )
            .expect("the tables the queries read");
        connection
    }

    #[test]
    fn the_other_name_is_none_where_the_catalog_has_none() {
        let connection = catalog();
        connection
            .execute(
                "INSERT INTO items (unique_name, name_en, name_ru, kind, prime)
                 VALUES ('/Lotus/Untranslated', 'Untranslated', NULL, 'misc', 0)",
                [],
            )
            .expect("a row without a Russian name");

        assert_eq!(
            other_name(&connection, "ru", "/Lotus/Volt/Suit").expect("a name"),
            Some("Volt Prime".to_owned())
        );
        assert_eq!(
            other_name(&connection, "en", "/Lotus/Untranslated").expect("no error"),
            None
        );
        assert_eq!(
            other_name(&connection, "en", "/Lotus/Nowhere").expect("no error"),
            None
        );
    }

    #[test]
    fn goods_answers_for_every_key_asked_for() {
        let asked = [
            "/Lotus/Volt/Suit".to_string(),
            "/Lotus/Nowhere".to_string(),
            "/Lotus/Volt/Systems".to_string(),
        ];

        let found = goods(&catalog(), "ru", &asked).expect("the goods");

        let keys: Vec<&str> = found.iter().map(|good| good.item.as_str()).collect();
        assert_eq!(keys, asked.iter().map(String::as_str).collect::<Vec<_>>());
        assert!(!found[1].known);
        assert_eq!(found[1].name, "/Lotus/Nowhere");
    }

    #[test]
    fn goods_tells_a_prime_part_from_the_warframe_it_builds() {
        let asked = [
            "/Lotus/Volt/Systems".to_string(),
            "/Lotus/Volt/Suit".to_string(),
        ];

        let found = goods(&catalog(), "ru", &asked).expect("the goods");

        assert!(found[0].prime);
        assert_eq!(found[0].ducats, Some(15));
        assert_eq!(found[0].image.as_deref(), Some("a1b2"));
        assert!(found[1].prime);
        assert_eq!(found[1].ducats, None);
    }

    #[test]
    fn an_unknown_key_carries_no_flags() {
        let found = goods(&catalog(), "ru", &["/Lotus/Nowhere".to_string()]).expect("the goods");

        assert!(!found[0].prime);
        assert_eq!(found[0].tradable, None);
        assert_eq!(found[0].slug, None);
        assert_eq!(found[0].vaulted, None);
        assert_eq!(found[0].makes, None);
    }

    #[test]
    fn goods_say_what_a_blueprint_builds() {
        let asked = [
            "/Lotus/Recipes/Lens".to_string(),
            "/Lotus/Volt/Systems".to_string(),
        ];

        let found = goods(&catalog(), "ru", &asked).expect("the goods");

        assert_eq!(found[0].makes.as_deref(), Some("focus-lens"));
        assert_eq!(found[1].makes, None);
    }

    #[test]
    fn goods_carry_the_vault_flag_as_three_states() {
        let asked = [
            "/Lotus/Volt/Systems".to_string(),
            "/Lotus/Recipes/Lens".to_string(),
            "/Lotus/Volt/Suit".to_string(),
        ];

        let found = goods(&catalog(), "ru", &asked).expect("the goods");

        assert_eq!(found[0].vaulted, Some(true));
        assert_eq!(found[1].vaulted, Some(false));
        assert_eq!(found[2].vaulted, None);
    }

    #[test]
    fn sets_keep_every_member() {
        let catalog = catalog();
        catalog
            .execute_batch(
                "INSERT INTO recipes VALUES ('/Lotus/Recipes/Volt', '/Lotus/Volt/Suit', 0, 0, 0);
                 INSERT INTO recipe_requires VALUES
                   ('/Lotus/Recipes/Volt', '/Lotus/Volt/Systems', 2);",
            )
            .expect("a recipe");

        let found = sets(&catalog, "ru").expect("the sets");

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].slug, "lonely_set");
        assert!(found[0].members.is_empty());
        assert_eq!(found[1].name, "Вольт Прайм: Комплект");
        assert_eq!(
            found[1].members,
            vec![
                "/Lotus/Volt/Suit".to_string(),
                "/Lotus/Volt/Systems".to_string()
            ]
        );
        assert_eq!(found[1].per_set, vec![1, 2]);
        assert_eq!(found[1].built.as_deref(), Some("/Lotus/Volt/Suit"));
        assert_eq!(found[1].ducats, Some(45));
        assert_eq!(found[1].vaulted, Some(true));
    }

    #[test]
    fn sets_take_the_english_name_when_asked_in_english() {
        let found = sets(&catalog(), "en").expect("the sets");

        assert_eq!(found[1].name, "Volt Prime Set");
    }

    /// Drop in `found` from `place`; panics when there is none.
    fn from(found: &[Drop], place: &str) -> Drop {
        found
            .iter()
            .find(|drop| drop.place == place)
            .unwrap_or_else(|| panic!("a drop from {place}"))
            .clone()
    }

    #[test]
    fn a_relic_answers_once_per_refinement() {
        let found =
            rewards(&catalog(), "ru", "/Lotus/Volt/Systems", Side::Found).expect("the relics");

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "Реликвия Лит T8");
        assert_eq!(found[0].refinement.as_deref(), Some("intact"));
        assert_eq!(found[0].chance, 20);
        assert_eq!(found[1].refinement.as_deref(), Some("radiant"));
        assert_eq!(found[1].chance, 100);
        assert_eq!(found[1].rarity, "rare");
    }

    /// Reward in `held` for `item`; panics when there is none.
    fn reward(held: &[Reward], item: &str) -> Reward {
        held.iter()
            .find(|reward| reward.item == item)
            .unwrap_or_else(|| panic!("a reward of {item}"))
            .clone()
    }

    #[test]
    fn what_a_relic_holds_carries_its_chance() {
        let held = rewards(
            &catalog(),
            "ru",
            "/Lotus/Projections/LithT8Platinum",
            Side::Inside,
        )
        .expect("the rewards");

        let systems = reward(&held, "/Lotus/Volt/Systems");
        assert_eq!(systems.chance, 100);
        assert_eq!(systems.count, 1);
        assert_eq!(systems.refinement, None);
    }

    #[test]
    fn a_reward_in_two_rarities_adds_its_chances() {
        let held = rewards(
            &catalog(),
            "ru",
            "/Lotus/Projections/LithT8Platinum",
            Side::Inside,
        )
        .expect("the rewards");

        let forma = reward(&held, "/Lotus/Recipes/Forma");
        assert_eq!(held.len(), 2);
        assert_eq!(forma.chance, 370);
        assert_eq!(forma.count, 2);
    }

    #[test]
    fn drops_carry_the_condition_and_the_place_context() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        let bounty = from(&found, "Цетус");
        assert_eq!(bounty.kind, "bounty");
        assert_eq!(bounty.planet, None);
        assert_eq!(bounty.mission.as_deref(), Some("Заказы Цетуса"));
        assert_eq!(bounty.giver.as_deref(), Some("Конзу"));
        assert_eq!(bounty.level_from, Some(5));
        assert_eq!(bounty.level_to, Some(15));
        assert_eq!(bounty.rotation.as_deref(), Some("B"));
        assert_eq!(bounty.stage.as_deref(), Some("Final Stage"));
        assert_eq!(bounty.chance, 306);
        assert_eq!(bounty.area.as_deref(), Some("cetus"));

        let node = from(&found, "Oestrus");
        assert_eq!(node.kind, "node");
        assert!(!node.event);
        assert_eq!(node.planet.as_deref(), Some("Эрида"));
        assert_eq!(node.mission.as_deref(), Some("Сбор"));
        assert_eq!(node.level_from, Some(34));
        assert_eq!(node.level_to, Some(38));
        assert_eq!(node.rarity, "uncommon");
        assert_eq!(node.area, None);
    }

    #[test]
    fn a_relic_drops_from_where_each_of_its_refinements_does() {
        let catalog = catalog();
        catalog
            .execute_batch(
                "INSERT INTO item_drops VALUES
                   ('Eris/Oestrus (Hive)', '/Lotus/Projections/LithT8Bronze', 'A', NULL,
                    'Rare', 0.05, NULL),
                   ('Lua/Strata (Hub)', '/Lotus/Projections/LithT8Platinum', NULL, NULL,
                    'Rare', 0.02, NULL);",
            )
            .expect("relic drops");

        for relic in [
            "/Lotus/Projections/LithT8Bronze",
            "/Lotus/Projections/LithT8Platinum",
        ] {
            let found = drops(&catalog, "ru", relic).expect("the drops");
            assert_eq!(found.len(), 2);
            assert_eq!(
                from(&found, "Oestrus").refinement.as_deref(),
                Some("intact")
            );
            assert_eq!(
                from(&found, "Реле Страта").refinement.as_deref(),
                Some("radiant")
            );
        }
        let parts = drops(&catalog, "ru", "/Lotus/Volt/Systems").expect("the drops");
        assert!(parts.iter().all(|drop| drop.refinement.is_none()));
    }

    #[test]
    fn an_area_carries_the_nodes_that_stand_in_it() {
        let found = areas(&catalog(), "ru").expect("the areas");

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].key, "cetus");
        assert_eq!(found[0].name, "Цетус");
        assert_eq!(found[0].nodes, ["CetusHub4", "SolNode228"]);
        assert_eq!(found[1].name, "Duviri");
        assert!(found[1].nodes.is_empty());
    }

    #[test]
    fn a_place_that_only_exists_during_an_event_says_so() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        let event = from(&found, "Event: Eris/Cyath (Spy)");
        assert!(event.event);
        assert_eq!(event.planet.as_deref(), Some("Эрида"));
    }

    #[test]
    fn a_grade_keeps_the_spelling_the_drop_tables_use() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        let hub = from(&found, "Реле Страта");
        assert_eq!(hub.rarity, "ultra-rare");
        assert_eq!(hub.count, Some(3));
    }

    #[test]
    fn a_place_without_a_fight_reports_no_levels() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        let hub = from(&found, "Реле Страта");
        assert_eq!(hub.planet.as_deref(), Some("Lua"));
        assert_eq!(hub.level_from, None);
        assert_eq!(hub.level_to, None);
    }

    #[test]
    fn an_item_got_only_by_its_blueprint_is_got_the_blueprints_way() {
        let found = origins(&catalog(), "ru", &["/Lotus/Items/Lens".to_owned()]).expect("origins");
        let way = found["/Lotus/Items/Lens"].way.clone();
        assert!(matches!(way, Some(median_ipc::Way::Lab { .. })), "{way:?}");
    }

    #[test]
    fn an_english_card_keeps_the_english_place() {
        let found = drops(&catalog(), "en", "/Lotus/Volt/Systems").expect("the drops");

        let node = from(&found, "Oestrus");
        assert_eq!(node.planet.as_deref(), Some("Eris"));
        assert_eq!(node.mission.as_deref(), Some("Hive"));
    }

    #[test]
    fn an_item_names_what_its_recipes_build() {
        let found = needed_by(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the needs");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].item, "/Lotus/Items/Lens");
        assert_eq!(found[0].name, "Эйдолонская линза");
        assert_eq!(found[0].count, 2);
    }

    #[test]
    fn an_item_no_recipe_wants_is_needed_by_nothing() {
        let found = needed_by(&catalog(), "ru", "/Lotus/Conductor").expect("the needs");

        assert!(found.is_empty());
    }

    #[test]
    fn a_recipe_is_found_from_the_blueprint_and_from_what_it_makes() {
        let from_blueprint = recipes(&catalog(), "ru", "/Lotus/Recipes/Lens").expect("the recipes");
        let from_result = recipes(&catalog(), "ru", "/Lotus/Items/Lens").expect("the recipes");

        assert_eq!(from_blueprint, from_result);
        assert_eq!(from_blueprint.len(), 1);
        assert_eq!(from_blueprint[0].blueprint, "/Lotus/Recipes/Lens");
        assert_eq!(from_blueprint[0].credits, Some(15000));
        assert_eq!(from_blueprint[0].time, Some(43200));
        assert_eq!(from_blueprint[0].rush, Some(25));
    }

    #[test]
    fn a_recipe_lists_what_it_consumes_richest_first() {
        let found = recipes(&catalog(), "ru", "/Lotus/Recipes/Lens").expect("the recipes");

        let parts = &found[0].parts;
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "Вольт Прайм: Система");
        assert_eq!(parts[0].count, 2);
        assert_eq!(parts[0].image.as_deref(), Some("a1b2"));
        assert_eq!(parts[1].name, "Вольт Прайм");
        assert_eq!(parts[1].count, 1);
    }

    #[test]
    fn an_item_nothing_builds_has_no_recipe() {
        let found = recipes(&catalog(), "ru", "/Lotus/Conductor").expect("the recipes");

        assert!(found.is_empty());
    }

    #[test]
    fn a_researched_blueprint_names_its_lab_and_its_price() {
        let found = research(&catalog(), "ru", "/Lotus/Recipes/Lens").expect("the research");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].lab, "Лаборатория Орокин");
        assert_eq!(found[0].credits, 10000);
        assert_eq!(found[0].time, 259200);
        assert_eq!(found[0].affinity, 3000);
        assert_eq!(found[0].prereq.as_deref(), Some("Amprex"));
    }

    #[test]
    fn an_item_no_clan_researches_has_no_lab() {
        let found = research(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the research");

        assert!(found.is_empty());
    }

    #[test]
    fn a_syndicate_offer_carries_the_rank_it_opens_at() {
        let found = sold(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the sales");

        assert_eq!(found[0].vendor, "New Loka");
        assert_eq!(found[0].cost, Some(25000));
        assert_eq!(found[0].currency, "Standing");
        assert_eq!(found[0].rank, Some(5));

        assert_eq!(found[1].vendor, "Баро Ки'Тир");
        assert_eq!(found[1].credits, Some(150000));
        assert_eq!(found[1].rank, None);
    }

    #[test]
    fn an_arcane_is_sold_inside_the_collection_that_rolls_it() {
        let found = sold(&catalog(), "ru", "/Lotus/Arcanes/Grace").expect("the sales");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].vendor, "Лойд");
        assert_eq!(
            found[0].pack.as_deref(),
            Some("Коллекция мистификаторов: Эйдолон")
        );
        assert_eq!(found[0].place.as_deref(), Some("Цетус"));
        assert_eq!(found[0].cost, Some(200));
        assert_eq!(found[0].coin.as_deref(), Some("Восфор"));

        let own = sold(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the sales");
        assert!(own.iter().all(|sale| sale.pack.is_none()));
    }

    #[test]
    fn an_arcane_names_its_collection() {
        let found =
            goods(&catalog(), "en", &["/Lotus/Arcanes/Grace".to_string()]).expect("the goods");

        assert_eq!(
            found[0].collection.as_deref(),
            Some("Eidolon Arcane Collection")
        );
    }

    #[test]
    fn a_trader_asked_in_english_keeps_the_english_name() {
        let found = sold(&catalog(), "en", "/Lotus/Volt/Systems").expect("the sales");

        assert_eq!(found[1].vendor, "Baro Ki'Teer");
    }

    #[test]
    fn enemies_stand_among_the_sources_richest_first() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        let places: Vec<&str> = found.iter().map(|drop| drop.place.as_str()).collect();
        assert_eq!(
            places,
            vec![
                "Цетус",
                "Ударный МОА Аксио",
                "Oestrus",
                "Event: Eris/Cyath (Spy)",
                "Breacher Moa",
                "Реле Страта",
            ]
        );

        let moa = from(&found, "Ударный МОА Аксио");
        assert_eq!(moa.kind, "enemy");
        assert_eq!(moa.level_from, Some(30));
        assert_eq!(moa.rotation, None);
    }

    #[test]
    fn an_enemy_drop_names_the_planets_it_is_met_on_most_first() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        assert_eq!(from(&found, "Breacher Moa").haunts, vec!["Эрида", "Lua"]);
        assert!(from(&found, "Ударный МОА Аксио").haunts.is_empty());
        assert!(from(&found, "Oestrus").haunts.is_empty());
    }

    #[test]
    fn an_enemy_without_a_russian_name_keeps_the_one_it_has() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        assert_eq!(from(&found, "Breacher Moa").kind, "enemy");
    }

    #[test]
    fn an_enemy_drop_counts_the_roll_for_its_table() {
        let found = drops(&catalog(), "ru", "/Lotus/Volt/Systems").expect("the drops");

        assert_eq!(from(&found, "Breacher Moa").chance, 50);
    }

    #[test]
    fn names_a_blueprint_the_game_spells_the_other_way_round() {
        let found = by_name(
            &catalog(),
            "ru",
            &["чертеж: баруук прайм: нейрооптика".to_owned()],
        )
        .expect("the names");

        assert_eq!(
            found,
            vec![Told {
                fold: "чертеж: баруук прайм: нейрооптика".to_owned(),
                name: "Баруук Прайм: Нейрооптика (Чертеж)".to_owned(),
                item: "/Lotus/Baruuk/NeuropticsBp".to_owned(),
            }]
        );
    }

    #[test]
    fn names_a_relic_the_game_spells_the_other_way_round() {
        let found = by_name(&catalog(), "ru", &["лит g12 реликвия".to_owned()]).expect("the names");

        assert_eq!(
            found,
            vec![Told {
                fold: "лит g12 реликвия".to_owned(),
                name: "Реликвия Лит G12".to_owned(),
                item: "/Lotus/Relics/LithG12".to_owned(),
            }]
        );
    }

    #[test]
    fn says_nothing_when_two_names_are_made_of_the_same_words() {
        let found =
            by_name(&catalog(), "ru", &["луна и солнце глиф".to_owned()]).expect("the names");

        assert!(found.is_empty());
    }

    #[test]
    fn names_what_the_fold_lands_on_outright() {
        let found = by_name(&catalog(), "ru", &["вольт прайм".to_owned()]).expect("the names");

        assert_eq!(
            found,
            vec![Told {
                fold: "вольт прайм".to_owned(),
                name: "Вольт Прайм".to_owned(),
                item: "/Lotus/Volt/Suit".to_owned(),
            }]
        );
    }

    #[test]
    fn finds_a_name_written_without_yo() {
        let found = search(&catalog(), "ru", "дирижер", 0).expect("a search");
        assert_eq!(found, vec!["/Lotus/Conductor"]);
    }

    #[test]
    fn puts_the_name_that_begins_with_the_query_first() {
        let found = search(&catalog(), "ru", "вольт", 0).expect("a search");
        assert_eq!(found, vec!["/Lotus/Volt/Suit", "/Lotus/Volt/Systems"]);
    }

    #[test]
    fn every_word_is_required() {
        let found = search(&catalog(), "ru", "система вольт", 0).expect("a search");
        assert_eq!(found, vec!["/Lotus/Volt/Systems"]);
    }

    #[test]
    fn a_query_with_no_word_matches_nothing() {
        assert!(
            search(&catalog(), "ru", " ", 0)
                .expect("a search")
                .is_empty()
        );
    }

    /// The reward screen dictionary lists every item any relic drops once, however many relics
    /// and rarities list it.
    #[test]
    fn the_reward_dictionary_names_each_item_once() {
        let mut found = reward_names(&catalog(), "ru").expect("the dictionary");
        found.sort();

        assert_eq!(
            found,
            vec![
                (
                    "/Lotus/Recipes/Forma".to_owned(),
                    "Форма (Чертеж)".to_owned()
                ),
                (
                    "/Lotus/Volt/Systems".to_owned(),
                    "Вольт Прайм: Система".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn the_reward_dictionary_answers_in_english_when_asked() {
        let found = reward_names(&catalog(), "en").expect("the dictionary");

        assert!(found.iter().any(|(_, name)| name == "Volt Prime Systems"));
    }

    #[test]
    fn a_part_a_set_takes_twice_is_counted_twice() {
        let catalog = catalog();
        catalog
            .execute_batch(
                "INSERT INTO recipes VALUES ('/Lotus/Recipes/Volt', '/Lotus/Volt/Suit', 0, 0, 0);
                 INSERT INTO recipe_requires VALUES
                   ('/Lotus/Recipes/Volt', '/Lotus/Volt/Systems', 2);",
            )
            .expect("a recipe");

        let parts = kit(&catalog, "en", &["/Lotus/Volt/Systems".into()]).expect("the kit");
        let per_set = |item: &str| {
            parts
                .iter()
                .find(|part| part.item == item)
                .map(|part| part.per_set)
        };
        assert_eq!(per_set("/Lotus/Volt/Systems"), Some(2));
        assert_eq!(per_set("/Lotus/Volt/Suit"), Some(1));
    }
}
