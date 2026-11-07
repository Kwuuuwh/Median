use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use deadpool_sqlite::{Config, Hook, HookError, InteractError, Pool, PoolError, Runtime};
use median_core::{CATALOG_FOLD, CATALOG_SCHEMA, Version};
use rusqlite::{Connection, OpenFlags};
use tracing::warn;

use crate::catalog::paths::Layout;

/// Maximum number of read connections open to one catalog file.
const READERS: usize = 4;

/// Reason a catalog read failed.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("no catalog is open")]
    Bare,
    #[error("no connection to the catalog: {0}")]
    Pool(#[from] PoolError),
    #[error("the read did not finish: {0}")]
    Interact(#[from] InteractError),
    #[error("{0}")]
    Query(String),
}

/// A catalog file open for reading.
pub struct Open {
    version: Version,
    path: PathBuf,
    pool: Pool,
}

impl Open {
    /// Opens the catalog at `path` read-only; fails when its schema, name-folding rule or version
    /// is not usable by this build.
    pub fn at(path: PathBuf) -> Result<Self, String> {
        let version = declared(&path)?;

        let pool = Config::new(&path)
            .builder(Runtime::Tokio1)
            .map_err(|e| e.to_string())?
            .max_size(READERS)
            .post_create(Hook::sync_fn(|wrapper, _| {
                let connection = wrapper
                    .lock()
                    .map_err(|_| HookError::message("the connection is poisoned"))?;
                connection
                    .pragma_update(None, "query_only", true)
                    .map_err(HookError::Backend)
            }))
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            version,
            path,
            pool,
        })
    }

    /// Version stored in the catalog's `meta` table.
    pub fn version(&self) -> Version {
        self.version
    }
}

/// Version stored in the catalog's `meta` table; fails when the schema (`user_version`) is not
/// `CATALOG_SCHEMA` or the name-folding rule is not `CATALOG_FOLD`.
fn declared(path: &Path) -> Result<Version, String> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;

    let schema: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if schema != i64::from(CATALOG_SCHEMA) {
        return Err(format!(
            "catalog schema {schema}, this build reads {CATALOG_SCHEMA}"
        ));
    }

    let folded: String = connection
        .query_row("SELECT value FROM meta WHERE key = 'fold'", [], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    if folded != CATALOG_FOLD {
        return Err(format!(
            "catalog folds names by {folded:?}, this build folds by {CATALOG_FOLD:?}"
        ));
    }

    let declared: String = connection
        .query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;

    Version::parse(&declared).ok_or_else(|| format!("unreadable version {declared:?}"))
}

/// Handle to the catalog in use; installing a newer catalog replaces it as a whole.
pub struct Catalog(Mutex<Option<Arc<Open>>>);

impl Catalog {
    /// Creates a handle holding no catalog.
    pub fn empty() -> Self {
        Self(Mutex::new(None))
    }

    /// Version of the catalog in use; `None` when no catalog is open.
    pub fn version(&self) -> Option<Version> {
        self.open().map(|open| open.version)
    }

    /// Runs `read` against one connection of the open catalog, off the caller's thread.
    pub async fn read<T, F>(&self, read: F) -> Result<T, ReadError>
    where
        F: FnOnce(&Connection) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        let open = self.open().ok_or(ReadError::Bare)?;
        let connection = open.pool.get().await?;

        connection
            .interact(move |connection| read(connection))
            .await?
            .map_err(ReadError::Query)
    }

    /// Makes `open` the catalog in use and deletes the file of the catalog it replaces.
    pub fn install(&self, open: Open) {
        let replaced = self
            .0
            .lock()
            .expect("catalog lock poisoned")
            .replace(Arc::new(open));
        let Some(previous) = replaced else {
            return;
        };

        let path = previous.path.clone();
        drop(previous);
        // A read still running keeps the file open; adopt() removes it at the next start.
        if let Err(e) = fs::remove_file(&path) {
            warn!(path = %path.display(), error = %e, "replaced catalog not removed");
        }
    }

    /// Shared handle to the catalog in use; a read holds it so the file stays open while
    /// `install` replaces it.
    fn open(&self) -> Option<Arc<Open>> {
        self.0.lock().expect("catalog lock poisoned").clone()
    }
}

/// Opens the newest usable catalog in `layout` and deletes every other catalog file; `None` when
/// no catalog is usable.
pub fn adopt(layout: &Layout) -> Option<Open> {
    let mut present = layout.catalogs().into_iter();
    let mut adopted = None;

    for (version, path) in present.by_ref() {
        match Open::at(path.clone()) {
            Ok(open) => {
                adopted = Some(open);
                break;
            }
            Err(e) => {
                warn!(%version, error = %e, "unusable catalog discarded");
                let _ = fs::remove_file(&path);
            }
        }
    }

    for (_, path) in present {
        let _ = fs::remove_file(path);
    }
    adopted
}
