use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use deadpool_sqlite::{Config, Hook, HookError, Pool, Runtime};
use rusqlite::Connection;
use tracing::warn;

/// Maximum number of open connections per pool.
const CONNECTIONS: usize = 4;

/// SQLite busy timeout in milliseconds: how long a statement waits for a lock held by another
/// connection.
const BUSY: i64 = 5_000;

/// Schema of one database at the version this build expects.
pub struct Schema {
    /// SQL that creates the tables of a new file.
    pub base: &'static str,
    /// Version of `base`, stored in the file's `user_version`.
    pub version: i64,
    /// Migrations from earlier versions, oldest first, each raising the version by one; the last
    /// one ends at `version`. A file older than the first step's start version cannot be upgraded.
    pub steps: &'static [&'static str],
    /// Whether the file only caches data that can be fetched again, so a file too old to upgrade
    /// is deleted instead of moved aside.
    pub cache: bool,
}

impl Schema {
    /// Oldest version a file can be upgraded from: `version - steps.len()`.
    fn oldest(&self) -> i64 {
        self.version - self.steps.len() as i64
    }
}

/// Why a database file could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("the file could not be migrated: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("the file is at version {found}; this build upgrades from version {oldest} on")]
    TooOld { found: i64, oldest: i64 },
    #[error("the file too old to upgrade could not be moved aside: {0}")]
    SetAside(io::Error),
    #[error("the connection pool could not be built: {0}")]
    Pool(String),
}

/// Opens the database at `path`: switches it to WAL, migrates it to `schema`, and returns a pool
/// whose connections have foreign keys on and a busy timeout. A file too old to upgrade is moved
/// aside as `<name>-v<version>.sqlite` (deleted for a cache) and a new one is created.
pub fn open(path: &Path, schema: &Schema) -> Result<Pool, OpenError> {
    match prepare(path, schema) {
        Err(OpenError::TooOld { found, oldest }) => {
            let aside = set_aside(path, found, schema.cache).map_err(OpenError::SetAside)?;
            warn!(
                path = %path.display(),
                version = found,
                oldest_version = oldest,
                moved_to = aside.map_or_else(|| "deleted".to_owned(), |to| to.display().to_string()),
                "database too old to upgrade, starting a new one"
            );
            prepare(path, schema)?;
        }
        prepared => prepared?,
    }

    Config::new(path)
        .builder(Runtime::Tokio1)
        .map_err(|e| OpenError::Pool(e.to_string()))?
        .max_size(CONNECTIONS)
        .post_create(Hook::sync_fn(|wrapper, _| {
            let connection = wrapper
                .lock()
                .map_err(|_| HookError::message("the connection is poisoned"))?;
            connection
                .pragma_update(None, "foreign_keys", true)
                .map_err(HookError::Backend)?;
            connection
                .pragma_update(None, "busy_timeout", BUSY)
                .map_err(HookError::Backend)
        }))
        .build()
        .map_err(|e| OpenError::Pool(e.to_string()))
}

/// Switches the file to WAL and migrates it to `schema`. The connection is closed on return,
/// so the file can be moved.
fn prepare(path: &Path, schema: &Schema) -> Result<(), OpenError> {
    let mut connection = Connection::open(path)?;
    connection.execute_batch("PRAGMA journal_mode = WAL;")?;
    migrate(&mut connection, schema)
}

/// Migrates the file to `schema`: a new file gets `base`, a file from an earlier build gets the
/// steps it has not run, a file from a later build is left unchanged. A file with tables but
/// no version is `TooOld`.
pub fn migrate(connection: &mut Connection, schema: &Schema) -> Result<(), OpenError> {
    let found: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found == 0 {
        let objects: i64 =
            connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
        if objects > 0 {
            return Err(OpenError::TooOld {
                found,
                oldest: schema.oldest(),
            });
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(schema.base)?;
        transaction.pragma_update(None, "user_version", schema.version)?;
        transaction.commit()?;
        return Ok(());
    }

    let oldest = schema.oldest();
    if found < oldest {
        return Err(OpenError::TooOld { found, oldest });
    }
    for (from, step) in (oldest..).zip(schema.steps) {
        if from < found {
            continue;
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(step)?;
        transaction.pragma_update(None, "user_version", from + 1)?;
        transaction.commit()?;
    }
    Ok(())
}

/// Moves the file and its `-wal`/`-shm` files to `<stem>-v<version>.<extension>`, or deletes
/// them for a cache; returns the new path, or `None` when deleted.
fn set_aside(path: &Path, version: i64, cache: bool) -> io::Result<Option<PathBuf>> {
    let companions = |base: &Path| {
        ["", "-wal", "-shm"].map(|suffix| {
            let mut name = base.as_os_str().to_owned();
            name.push(suffix);
            PathBuf::from(name)
        })
    };
    if cache {
        for file in companions(path) {
            match fs::remove_file(&file) {
                Err(why) if why.kind() != io::ErrorKind::NotFound => return Err(why),
                _ => {}
            }
        }
        return Ok(None);
    }

    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let aside = path.with_file_name(format!("{stem}-v{version}.{extension}"));
    for (from, to) in companions(path).into_iter().zip(companions(&aside)) {
        match fs::rename(&from, &to) {
            Err(why) if why.kind() != io::ErrorKind::NotFound => return Err(why),
            _ => {}
        }
    }
    Ok(Some(aside))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Version 3: the first step adds `added` (version 2), the second adds `note` (version 3).
    const SCHEMA: Schema = Schema {
        base: "CREATE TABLE held (id INTEGER PRIMARY KEY, name TEXT, note TEXT);
               CREATE TABLE added (id INTEGER PRIMARY KEY);",
        version: 3,
        steps: &[
            "CREATE TABLE added (id INTEGER PRIMARY KEY);",
            "ALTER TABLE held ADD COLUMN note TEXT;",
        ],
        cache: false,
    };

    fn version(connection: &Connection) -> i64 {
        connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("the version the file carries")
    }

    fn columns(connection: &Connection, table: &str) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .expect("the columns the table carries");
        statement
            .query_map([], |row| row.get(0))
            .expect("the columns read")
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap()
    }

    /// A file at version 1 with one row in `held`.
    fn first_version() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE held (id INTEGER PRIMARY KEY, name TEXT);
                 INSERT INTO held (id, name) VALUES (1, 'Tenno');
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        connection
    }

    #[test]
    fn a_new_file_gets_the_base_at_its_version() {
        let mut connection = Connection::open_in_memory().unwrap();

        migrate(&mut connection, &SCHEMA).unwrap();

        assert_eq!(columns(&connection, "held"), ["id", "name", "note"]);
        assert_eq!(columns(&connection, "added"), ["id"]);
        assert_eq!(version(&connection), 3);
    }

    #[test]
    fn a_file_at_the_oldest_version_takes_every_step_and_keeps_its_rows() {
        let mut connection = first_version();

        migrate(&mut connection, &SCHEMA).unwrap();

        assert_eq!(columns(&connection, "held"), ["id", "name", "note"]);
        assert_eq!(columns(&connection, "added"), ["id"]);
        let name: String = connection
            .query_row("SELECT name FROM held WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Tenno");
        assert_eq!(version(&connection), 3);
    }

    #[test]
    fn takes_no_step_twice() {
        let mut connection = first_version();
        migrate(&mut connection, &SCHEMA).unwrap();

        migrate(&mut connection, &SCHEMA).unwrap();

        assert_eq!(version(&connection), 3);
    }

    #[test]
    fn a_file_older_than_the_steps_reach_is_refused() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE held (id INTEGER PRIMARY KEY); PRAGMA user_version = 1;")
            .unwrap();
        let later = Schema {
            steps: &SCHEMA.steps[1..],
            ..SCHEMA
        };

        let refused = migrate(&mut connection, &later);

        assert!(matches!(
            refused,
            Err(OpenError::TooOld {
                found: 1,
                oldest: 2
            })
        ));
    }

    #[test]
    fn a_file_with_tables_but_no_version_is_refused() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE held (id INTEGER PRIMARY KEY);")
            .unwrap();

        let refused = migrate(&mut connection, &SCHEMA);

        assert!(matches!(refused, Err(OpenError::TooOld { found: 0, .. })));
    }

    #[test]
    fn leaves_a_file_from_a_later_build_alone() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE later (id INTEGER PRIMARY KEY); PRAGMA user_version = 9;")
            .unwrap();

        migrate(&mut connection, &SCHEMA).unwrap();

        assert_eq!(version(&connection), 9);
        assert_eq!(columns(&connection, "later"), ["id"]);
    }

    /// A fresh directory under the system temp directory.
    fn place(name: &str) -> PathBuf {
        let place =
            std::env::temp_dir().join(format!("median-schema-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&place);
        fs::create_dir_all(&place).expect("a place for the files");
        place
    }

    fn too_old_file(path: &Path) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch("CREATE TABLE gone (id INTEGER PRIMARY KEY); PRAGMA user_version = 1;")
            .unwrap();
    }

    #[test]
    fn a_file_too_old_is_kept_aside_and_a_new_one_started() {
        let place = place("aside");
        let path = place.join("account.sqlite");
        too_old_file(&path);
        let later = Schema {
            steps: &SCHEMA.steps[1..],
            ..SCHEMA
        };

        let pool = open(&path, &later).expect("a new file opens");
        drop(pool);

        let fresh = Connection::open(&path).unwrap();
        assert_eq!(version(&fresh), 3);
        let kept = Connection::open(place.join("account-v1.sqlite")).unwrap();
        assert_eq!(columns(&kept, "gone"), ["id"]);
    }

    #[test]
    fn a_cache_too_old_is_deleted_and_a_new_one_started() {
        let place = place("cache");
        let path = place.join("market.sqlite");
        too_old_file(&path);
        let cache = Schema {
            steps: &SCHEMA.steps[1..],
            cache: true,
            ..SCHEMA
        };

        let pool = open(&path, &cache).expect("a new file opens");
        drop(pool);

        assert_eq!(version(&Connection::open(&path).unwrap()), 3);
        assert!(!place.join("market-v1.sqlite").exists());
    }
}
