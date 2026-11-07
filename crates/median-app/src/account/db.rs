use std::path::Path;

use deadpool_sqlite::Pool;

use crate::schema::{self, OpenError, Schema};

/// Schema of the account store: the account's holdings, trades and plans, kept between runs.
pub(crate) const SCHEMA: Schema = Schema {
    base: BASE,
    version: 27,
    steps: &[DROP_TRADE],
    cache: false,
};

/// Migration to version 27: drops the trade window readings; trades are stored in `exchange`
/// and `swap`.
const DROP_TRADE: &str = "DROP TABLE trade;";

/// Tables of a new file, at version 27.
const BASE: &str = "
CREATE TABLE acquired (
  profile INTEGER NOT NULL REFERENCES profile(id),
  item    TEXT    NOT NULL,
  at      INTEGER NOT NULL,
  PRIMARY KEY (profile, item)
) WITHOUT ROWID;

CREATE TABLE affiliation (
  profile  INTEGER NOT NULL REFERENCES profile(id),
  tag      TEXT    NOT NULL,
  standing INTEGER NOT NULL,
  title    INTEGER NOT NULL,
  PRIMARY KEY (profile, tag)
) WITHOUT ROWID;

CREATE TABLE crafting (
  profile INTEGER NOT NULL REFERENCES profile(id),
  id      TEXT    NOT NULL,
  item    TEXT    NOT NULL,
  ready   INTEGER NOT NULL,
  PRIMARY KEY (profile, id)
) WITHOUT ROWID;

CREATE TABLE equipped (
  profile INTEGER NOT NULL REFERENCES profile(id),
  item    TEXT    NOT NULL,
  variant TEXT    NOT NULL,
  PRIMARY KEY (profile, item, variant)
) WITHOUT ROWID;

CREATE TABLE exchange (
  id       INTEGER PRIMARY KEY,
  at       INTEGER NOT NULL,
  item     TEXT    NOT NULL,
  slug     TEXT    NOT NULL,
  variant  TEXT    NOT NULL,
  side     TEXT    NOT NULL,
  platinum INTEGER NOT NULL,
  quantity INTEGER NOT NULL,
  origin   TEXT    NOT NULL,
  own      TEXT
);

CREATE INDEX exchange_by_time ON exchange(at);

CREATE TABLE favourite (
  profile INTEGER NOT NULL REFERENCES profile(id),
  item    TEXT    NOT NULL,
  at      INTEGER NOT NULL,
  PRIMARY KEY (profile, item)
) WITHOUT ROWID;

CREATE TABLE formed (
  profile INTEGER NOT NULL REFERENCES profile(id),
  item    TEXT    NOT NULL,
  forma   INTEGER NOT NULL,
  PRIMARY KEY (profile, item)
) WITHOUT ROWID;

CREATE TABLE haul_claim (
  profile   INTEGER NOT NULL REFERENCES profile(id),
  blueprint TEXT    NOT NULL,
  count     INTEGER NOT NULL,
  PRIMARY KEY (profile, blueprint)
) WITHOUT ROWID;

CREATE TABLE haul_gain (
  profile INTEGER NOT NULL REFERENCES profile(id),
  item    TEXT    NOT NULL,
  variant TEXT    NOT NULL,
  count   INTEGER NOT NULL,
  PRIMARY KEY (profile, item, variant)
) WITHOUT ROWID;

CREATE TABLE haul_start (
  profile INTEGER PRIMARY KEY REFERENCES profile(id),
  started INTEGER NOT NULL
);

CREATE TABLE holding (
  snapshot INTEGER NOT NULL REFERENCES snapshot(id),
  item     INTEGER NOT NULL REFERENCES item(id),
  variant  TEXT    NOT NULL,
  count    INTEGER NOT NULL,
  PRIMARY KEY (snapshot, item, variant)
) WITHOUT ROWID;

CREATE TABLE intrinsic (
  profile INTEGER NOT NULL REFERENCES profile(id),
  school  TEXT    NOT NULL,
  rank    INTEGER NOT NULL,
  PRIMARY KEY (profile, school)
) WITHOUT ROWID;

CREATE TABLE item (
  id  INTEGER PRIMARY KEY,
  key TEXT    NOT NULL UNIQUE
);

CREATE TABLE mastered (
  profile INTEGER NOT NULL REFERENCES profile(id),
  item    TEXT    NOT NULL,
  xp      INTEGER NOT NULL,
  PRIMARY KEY (profile, item)
) WITHOUT ROWID;

CREATE TABLE mastery_rank (
  profile   INTEGER PRIMARY KEY REFERENCES profile(id),
  rank      INTEGER NOT NULL,
  next_test INTEGER
);

CREATE TABLE mission (
  profile   INTEGER NOT NULL REFERENCES profile(id),
  tag       TEXT    NOT NULL,
  completes INTEGER NOT NULL,
  tier      INTEGER,
  PRIMARY KEY (profile, tag)
) WITHOUT ROWID;

CREATE TABLE order_step (
  own      TEXT    NOT NULL,
  at       INTEGER NOT NULL,
  platinum INTEGER NOT NULL,
  quantity INTEGER NOT NULL,
  step     TEXT    NOT NULL
);

CREATE INDEX order_step_of_order ON order_step(own, at);

CREATE TABLE own_order (
  id         TEXT    PRIMARY KEY,
  item_id    TEXT    NOT NULL,
  slug       TEXT,
  variant    TEXT    NOT NULL,
  side       TEXT    NOT NULL,
  platinum   INTEGER NOT NULL,
  quantity   INTEGER NOT NULL,
  per_trade  INTEGER NOT NULL,
  visible    INTEGER NOT NULL,
  created_at TEXT    NOT NULL,
  updated_at TEXT    NOT NULL,
  seen       INTEGER NOT NULL,
  gone       INTEGER,
  by_agent   INTEGER NOT NULL DEFAULT 0,
  promised   INTEGER
) WITHOUT ROWID;

CREATE INDEX own_order_standing ON own_order(gone, side);

CREATE TABLE position (
  slug     TEXT    NOT NULL,
  variant  TEXT    NOT NULL,
  item     TEXT    NOT NULL,
  quantity INTEGER NOT NULL,
  spent    INTEGER NOT NULL,
  opened   INTEGER NOT NULL, ranked_before INTEGER,
  PRIMARY KEY (slug, variant)
) WITHOUT ROWID;

CREATE TABLE presence (
  began INTEGER PRIMARY KEY,
  ended INTEGER
) WITHOUT ROWID;

CREATE TABLE profile (
  id      INTEGER PRIMARY KEY,
  created INTEGER NOT NULL UNIQUE,
  name    TEXT    NOT NULL
);

CREATE TABLE riven_buy (
  auction TEXT    PRIMARY KEY,
  weapon  TEXT    NOT NULL,
  name    TEXT    NOT NULL,
  riven   TEXT    NOT NULL,
  rank    INTEGER NOT NULL,
  price   INTEGER NOT NULL,
  bought  INTEGER NOT NULL,
  sold    INTEGER,
  sale    INTEGER
) WITHOUT ROWID;

CREATE TABLE roadmap (
  profile INTEGER NOT NULL REFERENCES profile(id),
  item    TEXT    NOT NULL,
  at      INTEGER NOT NULL,
  PRIMARY KEY (profile, item)
) WITHOUT ROWID;

CREATE TABLE roadmap_part (
  profile INTEGER NOT NULL,
  item    TEXT    NOT NULL,
  part    TEXT    NOT NULL,
  buy     INTEGER NOT NULL,
  PRIMARY KEY (profile, item, part),
  FOREIGN KEY (profile, item) REFERENCES roadmap(profile, item) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE snapshot (
  id          INTEGER PRIMARY KEY,
  profile     INTEGER NOT NULL REFERENCES profile(id),
  taken       INTEGER NOT NULL,
  digest      BLOB    NOT NULL,
  whole       INTEGER NOT NULL,
  platinum    INTEGER,
  credits     INTEGER,
  trades_left INTEGER,
  endo        INTEGER
);

CREATE INDEX snapshot_of_profile ON snapshot(profile, id);

CREATE TABLE swap (
  id       INTEGER PRIMARY KEY,
  at       INTEGER NOT NULL,
  partner  TEXT    NOT NULL,
  outcome  TEXT    NOT NULL,
  platinum INTEGER NOT NULL,
  buying   INTEGER NOT NULL
);

CREATE INDEX swap_by_time ON swap(at);

CREATE TABLE swap_part (
  swap  INTEGER NOT NULL REFERENCES swap(id),
  side  TEXT    NOT NULL,
  name  TEXT    NOT NULL,
  count INTEGER NOT NULL,
  kind  TEXT    NOT NULL,
  rank  INTEGER
);

CREATE INDEX swap_part_of_swap ON swap_part(swap);
";

pub fn open(path: &Path) -> Result<Pool, OpenError> {
    schema::open(path, &SCHEMA)
}

/// In-memory database with every table of a new account store.
#[cfg(test)]
pub(crate) fn laid() -> rusqlite::Connection {
    let mut connection = rusqlite::Connection::open_in_memory().expect("memory opens");
    schema::migrate(&mut connection, &SCHEMA).expect("the tables land");
    connection
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::schema::migrate;

    /// Every table and index in the file, with the statement that creates it.
    fn objects(connection: &Connection) -> Vec<(String, String, Option<String>)> {
        let mut statement = connection
            .prepare("SELECT type, name, sql FROM sqlite_master ORDER BY name")
            .expect("the objects read");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("the objects read")
            .collect::<rusqlite::Result<_>>()
            .expect("the objects read")
    }

    #[test]
    fn a_file_at_version_26_ends_with_the_tables_of_a_new_one() {
        let mut fresh = Connection::open_in_memory().unwrap();
        migrate(&mut fresh, &SCHEMA).unwrap();
        let mut earlier = Connection::open_in_memory().unwrap();
        earlier.execute_batch(BASE).unwrap();
        earlier
            .execute_batch(
                "CREATE TABLE trade (
                   session  TEXT    NOT NULL,
                   revision INTEGER NOT NULL,
                   origin   TEXT    NOT NULL,
                   PRIMARY KEY (session, revision, origin)
                 ) WITHOUT ROWID;
                 PRAGMA user_version = 26;",
            )
            .unwrap();

        migrate(&mut earlier, &SCHEMA).unwrap();

        assert_eq!(objects(&earlier), objects(&fresh));
    }
}
