use rusqlite::{Connection, OptionalExtension, params};

/// Riven data of a weapon that can have a riven.
#[derive(Debug, Clone, PartialEq)]
pub struct Armed {
    /// Riven class that selects the stat pool, as spelled in the catalog, e.g. `pistol`.
    pub class: String,
    /// warframe.market slug of the weapon's riven family, e.g. `dual_toxocyst`.
    pub slug: Option<String>,
    /// English name of the weapon's riven family, e.g. `Dual Toxocyst`.
    pub family: Option<String>,
    pub disposition: f64,
}

/// Riven data of the weapon with unique name `weapon`; `None` when it cannot have a riven.
pub fn armed(connection: &Connection, weapon: &str) -> Result<Option<Armed>, String> {
    connection
        .query_row(
            "SELECT r.class, r.slug, r.family, w.disposition
               FROM riven_weapons r JOIN weapons w USING (unique_name)
              WHERE r.unique_name = ?1",
            params![weapon],
            |row| {
                Ok(Armed {
                    class: row.get(0)?,
                    slug: row.get(1)?,
                    family: row.get(2)?,
                    disposition: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
}

/// Riven class of the weapon with unique name `weapon`, as spelled in the catalog; `None` when
/// it cannot have a riven.
pub fn riven_class(connection: &Connection, weapon: &str) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT class FROM riven_weapons WHERE unique_name = ?1",
            params![weapon],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())
}

/// English family name and riven class of a warframe.market riven slug; `None` when no weapon
/// with that slug has a family name.
pub fn slug_family(
    connection: &Connection,
    slug: &str,
) -> Result<Option<(String, String)>, String> {
    connection
        .query_row(
            "SELECT family, class FROM riven_weapons
              WHERE slug = ?1 AND family IS NOT NULL LIMIT 1",
            params![slug],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())
}

/// Unique name of the weapon a warframe.market riven slug refers to: the family variant whose
/// disposition is closest to the market's, ties broken by unique name.
pub fn slug_weapon(connection: &Connection, slug: &str) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT r.unique_name
               FROM riven_weapons r JOIN weapons w USING (unique_name)
              WHERE r.slug = ?1
              ORDER BY abs(w.disposition - coalesce(r.market_disposition, w.disposition)),
                       r.unique_name
              LIMIT 1",
            params![slug],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())
}

/// One stat a riven class can roll.
#[derive(Debug, Clone, PartialEq)]
pub struct Pooled {
    pub tag: String,
    /// Stat value per point of disposition, before the riven's own scaling.
    pub base: f64,
    /// How the game shows the number: `percent`, `multiplier` or `plain`.
    pub form: String,
    /// Text the game shows after the number, in the requested language.
    pub label: String,
    /// Whether a riven can roll it as a buff.
    pub can_buff: bool,
    /// Whether a riven can roll it as a curse.
    pub can_curse: bool,
    /// warframe.market slug of the stat.
    pub slug: Option<String>,
    /// Syllable the stat puts at the start of a riven's name.
    pub prefix: String,
    /// Syllable the stat puts at the end of a riven's name.
    pub suffix: String,
}

/// Every stat a riven of `class` can roll.
pub fn pool(connection: &Connection, lang: &str, class: &str) -> Result<Vec<Pooled>, String> {
    let mut statement = connection
        .prepare(
            "SELECT s.tag, s.base, s.form, s.shown_en, s.shown_ru, s.positive_only,
                    s.negative_only, s.market_slug, s.prefix_en, s.suffix_en
               FROM riven_pools p JOIN riven_stats s ON s.pool = p.unique_name
              WHERE p.class = ?1 AND s.base IS NOT NULL
              ORDER BY s.tag",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![class], |row| {
            let english: String = row.get(3)?;
            let russian: Option<String> = row.get(4)?;
            let shown = match lang {
                "ru" => russian.unwrap_or(english),
                _ => english,
            };
            Ok(Pooled {
                tag: row.get(0)?,
                base: row.get(1)?,
                form: row.get(2)?,
                label: label(&shown),
                can_buff: row.get::<_, Option<bool>>(6)? != Some(true),
                can_curse: row.get::<_, Option<bool>>(5)? != Some(true),
                slug: row.get(7)?,
                prefix: row.get(8)?,
                suffix: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

/// Display template with the `|val|` placeholder and `<DT_…>` colour tags removed.
fn label(shown: &str) -> String {
    let mut text = shown.replace("|val|", "");
    while let Some(start) = text.find("<DT_") {
        match text[start..].find('>') {
            Some(end) => text.replace_range(start..=start + end, ""),
            None => break,
        }
    }
    text.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A catalog holding Dual Toxocyst and the pistol pool.
    fn catalog() -> Connection {
        let connection = Connection::open_in_memory().expect("a catalog in memory");
        connection
            .execute_batch(
                "CREATE TABLE weapons (
                     unique_name TEXT PRIMARY KEY, disposition REAL NOT NULL,
                     magazine INTEGER, reload_secs REAL);
                 CREATE TABLE riven_weapons (
                     unique_name TEXT PRIMARY KEY, class TEXT NOT NULL, slug TEXT,
                     family TEXT);
                 CREATE TABLE riven_pools (unique_name TEXT PRIMARY KEY, class TEXT NOT NULL);
                 CREATE TABLE riven_stats (
                     pool TEXT NOT NULL, tag TEXT NOT NULL, base REAL, form TEXT NOT NULL,
                     shown_en TEXT NOT NULL, shown_ru TEXT, positive_only INTEGER,
                     negative_only INTEGER, market_slug TEXT, prefix_en TEXT NOT NULL,
                     suffix_en TEXT NOT NULL);

                 INSERT INTO weapons VALUES ('/Toxocyst', 1.35, 12, 2.35);
                 INSERT INTO riven_weapons VALUES ('/Toxocyst', 'pistol', 'dual_toxocyst', 'Dual Toxocyst');
                 INSERT INTO riven_pools VALUES ('/PistolPool', 'pistol');
                 INSERT INTO riven_stats VALUES
                     ('/PistolPool', 'WeaponDamageAmountMod', 0.0244, 'percent',
                      '|val|% Damage', '|val|% урона', 0, 0, 'base_damage_/_melee_damage', 'visi', 'ata'),
                     ('/PistolPool', 'WeaponCritChanceMod', 0.016666, 'percent',
                      '|val|% Critical Chance', NULL, 0, 0, 'critical_chance', 'crita', 'cron'),
                     ('/PistolPool', 'WeaponToxinDamageMod', 0.01, 'percent',
                      '|val|% <DT_POISON_COLOR>Toxin', '|val|% урона <DT_POISON_COLOR>токсином',
                      1, 0, 'toxin_damage', 'toxi', 'tox');",
            )
            .expect("the catalog tables");
        connection
    }

    #[test]
    fn a_weapon_comes_with_its_riven_family() {
        let armed = armed(&catalog(), "/Toxocyst")
            .expect("a read")
            .expect("the weapon");

        assert_eq!(armed.class, "pistol");
        assert_eq!(armed.slug.as_deref(), Some("dual_toxocyst"));
        assert_eq!(armed.family.as_deref(), Some("Dual Toxocyst"));
        assert_eq!(armed.disposition, 1.35);
    }

    #[test]
    fn a_weapon_draws_from_its_riven_class() {
        assert_eq!(
            riven_class(&catalog(), "/Toxocyst")
                .expect("a read")
                .as_deref(),
            Some("pistol")
        );
    }

    #[test]
    fn a_weapon_no_riven_rolls_for_is_not_armed() {
        assert_eq!(armed(&catalog(), "/Nothing").expect("a read"), None);
    }

    #[test]
    fn a_pool_reads_in_the_language_asked_and_falls_back_to_english() {
        let pool = pool(&catalog(), "ru", "pistol").expect("a read");
        let labels: Vec<&str> = pool.iter().map(|stat| stat.label.as_str()).collect();

        assert_eq!(
            labels,
            vec!["% Critical Chance", "% урона", "% урона токсином"]
        );
        assert!(pool[2].can_buff && !pool[2].can_curse);
        assert_eq!(
            (pool[1].prefix.as_str(), pool[1].suffix.as_str()),
            ("visi", "ata")
        );
    }
}
