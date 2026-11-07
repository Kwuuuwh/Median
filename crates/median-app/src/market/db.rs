use std::path::Path;

use deadpool_sqlite::Pool;

use crate::schema::{self, OpenError, Schema};

/// Schema of the market cache, which stores warframe.market data to avoid repeat requests.
/// Every row can be fetched again, so a file too old to migrate is deleted.
pub(crate) const SCHEMA: Schema = Schema {
    base: BASE,
    version: 14,
    steps: &[OBSERVATION],
    cache: true,
};

/// Version 14: renames the table of socket orders from `heard` to `observation`.
const OBSERVATION: &str = "
ALTER TABLE heard RENAME TO observation;
DROP INDEX heard_by_pair;
CREATE INDEX observation_by_pair ON observation(slug, variant, at);
";

/// The tables of a new file, at version 14.
const BASE: &str = "
CREATE TABLE closed (
  slug         TEXT    NOT NULL,
  variant      TEXT    NOT NULL,
  span         TEXT    NOT NULL,
  starts       INTEGER NOT NULL,
  rank         INTEGER,
  subtype      TEXT,
  amber_stars  INTEGER,
  cyan_stars   INTEGER,
  volume       INTEGER NOT NULL,
  min_price    REAL    NOT NULL,
  max_price    REAL    NOT NULL,
  open_price   REAL,
  closed_price REAL,
  avg_price    REAL    NOT NULL,
  wa_price     REAL,
  median       REAL    NOT NULL,
  moving_avg   REAL,
  donch_top    REAL,
  donch_bot    REAL,
  market_id    TEXT,
  PRIMARY KEY (slug, variant, span, starts)
) WITHOUT ROWID;

CREATE TABLE observation (
  id        TEXT    NOT NULL,
  slug      TEXT    NOT NULL,
  variant   TEXT    NOT NULL,
  side      TEXT    NOT NULL,
  platinum  INTEGER NOT NULL,
  quantity  INTEGER NOT NULL,
  per_trade INTEGER,
  reachable INTEGER NOT NULL,
  at        INTEGER NOT NULL,
  PRIMARY KEY (id, platinum)
) WITHOUT ROWID;

CREATE INDEX observation_by_pair ON observation(slug, variant, at);

CREATE TABLE item (
  slug             TEXT    PRIMARY KEY,
  market_id        TEXT    NOT NULL,
  tradable         INTEGER NOT NULL,
  trading_tax      INTEGER,
  rarity           TEXT,
  req_mastery_rank INTEGER,
  ducats           INTEGER,
  vaulted          INTEGER,
  max_rank         INTEGER,
  max_amber_stars  INTEGER,
  max_cyan_stars   INTEGER,
  seen             INTEGER NOT NULL,
  bulk             INTEGER
);

CREATE TABLE listed (
  id       TEXT    PRIMARY KEY,
  slug     TEXT    NOT NULL,
  game_ref TEXT,
  seen     INTEGER NOT NULL
) WITHOUT ROWID;

CREATE INDEX listed_by_slug ON listed(slug);

CREATE TABLE live (
  slug               TEXT    NOT NULL,
  variant            TEXT    NOT NULL,
  taken              INTEGER NOT NULL,
  rank               INTEGER,
  subtype            TEXT,
  amber_stars        INTEGER,
  cyan_stars         INTEGER,
  lowest             INTEGER,
  lowest_ingame      INTEGER,
  highest_bid_ingame INTEGER,
  sellers_ingame     INTEGER, held_ingame INTEGER,
  PRIMARY KEY (slug, variant, taken)
) WITHOUT ROWID;

CREATE TABLE refreshed (
  slug TEXT    NOT NULL,
  what TEXT    NOT NULL,
  at   INTEGER NOT NULL,
  PRIMARY KEY (slug, what)
) WITHOUT ROWID;

CREATE TABLE rival (
  slug    TEXT    NOT NULL,
  variant TEXT    NOT NULL,
  name    TEXT    NOT NULL,
  updated TEXT    NOT NULL,
  moves   INTEGER NOT NULL,
  seen    INTEGER NOT NULL,
  PRIMARY KEY (slug, variant, name)
) WITHOUT ROWID;

CREATE TABLE riven_auction (
  id         TEXT    PRIMARY KEY,
  weapon     TEXT    NOT NULL,
  name       TEXT    NOT NULL,
  buyout     INTEGER,
  starting   INTEGER NOT NULL,
  direct     INTEGER NOT NULL,
  rank       INTEGER NOT NULL,
  rerolls    INTEGER NOT NULL,
  mastery    INTEGER NOT NULL,
  polarity   TEXT,
  stats      TEXT    NOT NULL,
  owner      TEXT,
  created    TEXT    NOT NULL,
  first_seen INTEGER NOT NULL,
  last_seen  INTEGER NOT NULL,
  gone       INTEGER
) WITHOUT ROWID;

CREATE INDEX riven_auction_by_weapon ON riven_auction(weapon, first_seen);

CREATE TABLE riven_band (
  riven  TEXT    PRIMARY KEY,
  low    REAL    NOT NULL,
  middle REAL    NOT NULL,
  high   REAL    NOT NULL,
  lots   INTEGER NOT NULL,
  basis  TEXT    NOT NULL,
  capped INTEGER NOT NULL,
  at     INTEGER NOT NULL
) WITHOUT ROWID;

CREATE TABLE riven_price (
  id       TEXT    NOT NULL,
  at       INTEGER NOT NULL,
  buyout   INTEGER,
  starting INTEGER NOT NULL,
  PRIMARY KEY (id, at)
) WITHOUT ROWID;

CREATE TABLE standing (
  slug        TEXT    NOT NULL,
  variant     TEXT    NOT NULL,
  side        TEXT    NOT NULL,
  starts      INTEGER NOT NULL,
  rank        INTEGER,
  subtype     TEXT,
  amber_stars INTEGER,
  cyan_stars  INTEGER,
  orders      INTEGER NOT NULL,
  min_price   REAL    NOT NULL,
  max_price   REAL    NOT NULL,
  median      REAL    NOT NULL,
  PRIMARY KEY (slug, variant, side, starts)
) WITHOUT ROWID;

CREATE TABLE subtype (
  slug    TEXT NOT NULL REFERENCES item(slug),
  subtype TEXT NOT NULL,
  PRIMARY KEY (slug, subtype)
) WITHOUT ROWID;

CREATE TABLE unexpected (
  field   TEXT    NOT NULL,
  seen_in TEXT    NOT NULL,
  slug    TEXT    NOT NULL,
  at      INTEGER NOT NULL,
  sample  TEXT    NOT NULL,
  PRIMARY KEY (field, seen_in)
) WITHOUT ROWID;
";

/// Opens the market cache at `path`, creating it when missing.
pub fn open(path: &Path) -> Result<Pool, OpenError> {
    schema::open(path, &SCHEMA)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::schema::migrate;

    /// Every table and index in the file with its CREATE statement, without the quotes a rename
    /// adds around names.
    fn objects(connection: &Connection) -> Vec<(String, String, String)> {
        let mut statement = connection
            .prepare("SELECT type, name, coalesce(sql, '') FROM sqlite_master ORDER BY name")
            .expect("the objects read");
        statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, String>(2)?.replace('"', ""),
                ))
            })
            .expect("the objects read")
            .collect::<rusqlite::Result<_>>()
            .expect("the objects read")
    }

    #[test]
    fn a_file_at_version_13_ends_with_the_tables_of_a_new_one() {
        let mut fresh = Connection::open_in_memory().unwrap();
        migrate(&mut fresh, &SCHEMA).unwrap();
        let mut earlier = Connection::open_in_memory().unwrap();
        earlier.execute_batch(BASE).unwrap();
        earlier
            .execute_batch(
                "ALTER TABLE observation RENAME TO heard;
                 DROP INDEX observation_by_pair;
                 CREATE INDEX heard_by_pair ON heard(slug, variant, at);
                 PRAGMA user_version = 13;",
            )
            .unwrap();

        migrate(&mut earlier, &SCHEMA).unwrap();

        assert_eq!(objects(&earlier), objects(&fresh));
    }
}
