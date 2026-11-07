use std::fmt;

use thiserror::Error;

use crate::effect::Effect;

/// Catalog schema this build can open.
pub const CATALOG_SCHEMA: u32 = 19;

/// Identifier of the folding rule the catalog's search index uses (see `fold`).
pub const CATALOG_FOLD: &str = "ru-yo-1";

/// Name folded as the search index stores it: lower case, with `ё` replaced by `е`.
pub fn fold(name: &str) -> String {
    name.to_lowercase().replace('ё', "е")
}

/// The words a name is made of: folded, cut on everything that is neither letter nor digit,
/// and sorted. Two names made of the same words fold to the same list whatever order they
/// stand in — `ЧЕРТЁЖ: Баруук Прайм` and `Баруук Прайм (Чертеж)`.
pub fn words(name: &str) -> Vec<String> {
    let mut words: Vec<String> = fold(name)
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    words.sort();
    words
}

/// A catalog version, `MAJOR.MINOR.PATCH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parses `MAJOR.MINOR.PATCH`; `None` on any other shape.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        parts.next().is_none().then_some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// One downloadable part of a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub file: String,
    pub blake3: String,
    pub size: u64,
}

/// Published game data release, as the manifest describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Schema version of the published database.
    pub schema: u32,
    pub version: Version,
    /// Compressed catalog database.
    pub db: Asset,
    /// Image pack.
    pub pack: Asset,
}

/// Action a manifest check calls for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    Install(Release),
    UpToDate,
    /// The published catalog has a schema newer than this build can open.
    ClientTooOld {
        schema: u32,
    },
    /// The published catalog has a schema older than this build can open.
    CatalogTooOld {
        schema: u32,
    },
}

/// Action for `release` given the installed catalog and image pack versions. A catalog whose
/// image pack is missing or belongs to another release counts as not installed.
pub fn plan(installed: Option<Version>, pack: Option<Version>, release: Release) -> Plan {
    if release.schema > CATALOG_SCHEMA {
        return Plan::ClientTooOld {
            schema: release.schema,
        };
    }
    if release.schema < CATALOG_SCHEMA {
        return Plan::CatalogTooOld {
            schema: release.schema,
        };
    }
    match installed.filter(|version| pack == Some(*version)) {
        Some(have) if have >= release.version => Plan::UpToDate,
        _ => Plan::Install(release),
    }
}

/// Which part of a release is being fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Database,
    Pictures,
}

/// Reason a game data check or install failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogError {
    /// The published manifest could not be read.
    #[error("the manifest could not be read: {0}")]
    Manifest(String),
    /// The database archive could not be fetched.
    #[error("the database could not be fetched: {0}")]
    Database(String),
    /// The archive could not be unpacked or opened.
    #[error("the catalog could not be opened: {0}")]
    Install(String),
    /// The image pack could not be updated to the release.
    #[error("the pictures could not be fetched: {0}")]
    Pack(String),
}

/// Current game data operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Idle,
    Checking,
    Fetching {
        release: Release,
        part: Part,
        done: u64,
        total: u64,
    },
    /// The database archive is being unpacked and opened.
    Opening {
        release: Release,
    },
    /// The manifest or an asset could not be read.
    Unreachable {
        reason: CatalogError,
    },
    /// The published catalog needs a newer build than this one.
    Unsupported {
        schema: u32,
    },
    /// The published catalog's schema is older than this build can open, so there is nothing to
    /// install.
    Stale {
        schema: u32,
    },
}

impl Activity {
    /// Release being installed; `None` when no install is in progress.
    pub fn release(&self) -> Option<&Release> {
        match self {
            Activity::Fetching { release, .. } | Activity::Opening { release } => Some(release),
            Activity::Idle
            | Activity::Checking
            | Activity::Unreachable { .. }
            | Activity::Unsupported { .. }
            | Activity::Stale { .. } => None,
        }
    }
}

/// Installed game data versions and the current operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    /// Version of the open catalog; `None` when none is open.
    pub installed: Option<Version>,
    /// Version whose image pack is complete on disk.
    pub pack: Option<Version>,
    pub activity: Activity,
}

impl Catalog {
    /// Whether a check or an install is in progress.
    pub fn is_busy(&self) -> bool {
        !matches!(
            self.activity,
            Activity::Idle
                | Activity::Unreachable { .. }
                | Activity::Unsupported { .. }
                | Activity::Stale { .. }
        )
    }

    /// Whether the catalog or its image pack is missing, so the app has no usable game data yet.
    pub fn is_bare(&self) -> bool {
        self.installed.is_none() || self.pack.is_none()
    }
}

/// Game data event.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogEvent {
    /// A check was requested.
    SyncRequested,
    /// The published manifest was read.
    Checked(Release),
    /// The database archive is being unpacked and opened.
    Opening,
    /// A catalog of this version is open.
    Ready(Version),
    /// The image pack of this version is complete on disk.
    PackReady(Version),
    /// The check or the install did not finish.
    Failed(CatalogError),
}

/// Catalog state after `event`, and the effects to run.
pub(crate) fn update(catalog: &Catalog, event: CatalogEvent) -> (Catalog, Vec<Effect>) {
    match event {
        CatalogEvent::SyncRequested => {
            if catalog.is_busy() {
                return (catalog.clone(), Vec::new());
            }
            let mut next = catalog.clone();
            next.activity = Activity::Checking;
            (next, vec![Effect::CheckCatalog])
        }
        CatalogEvent::Checked(release) => {
            let mut next = catalog.clone();
            match plan(catalog.installed, catalog.pack, release) {
                Plan::Install(release) => {
                    next.activity = Activity::Fetching {
                        total: release.db.size,
                        release: release.clone(),
                        part: Part::Database,
                        done: 0,
                    };
                    (next, vec![Effect::FetchDatabase(release)])
                }
                Plan::UpToDate => {
                    next.activity = Activity::Idle;
                    (next, Vec::new())
                }
                Plan::ClientTooOld { schema } => {
                    next.activity = Activity::Unsupported { schema };
                    (next, Vec::new())
                }
                Plan::CatalogTooOld { schema } => {
                    next.activity = Activity::Stale { schema };
                    (next, Vec::new())
                }
            }
        }
        CatalogEvent::Opening => {
            let Activity::Fetching { release, .. } = &catalog.activity else {
                return (catalog.clone(), Vec::new());
            };
            let mut next = catalog.clone();
            next.activity = Activity::Opening {
                release: release.clone(),
            };
            (next, Vec::new())
        }
        CatalogEvent::Ready(version) => {
            let mut next = catalog.clone();
            next.installed = Some(version);
            match catalog.activity.release() {
                Some(release) => {
                    next.activity = Activity::Fetching {
                        total: release.pack.size,
                        release: release.clone(),
                        part: Part::Pictures,
                        done: 0,
                    };
                    (next, vec![Effect::FetchPictures(release.clone())])
                }
                None => {
                    next.activity = Activity::Idle;
                    (next, Vec::new())
                }
            }
        }
        CatalogEvent::PackReady(version) => {
            let mut next = catalog.clone();
            next.pack = Some(version);
            next.activity = Activity::Idle;
            (next, Vec::new())
        }
        CatalogEvent::Failed(reason) => {
            let mut next = catalog.clone();
            next.activity = Activity::Unreachable { reason };
            (next, Vec::new())
        }
    }
}

/// Catalog with the download progress of the current part set to `done` of `total` bytes;
/// unchanged when nothing is being fetched.
pub(crate) fn progressed(catalog: &Catalog, done: u64, total: u64) -> Catalog {
    let Activity::Fetching { release, part, .. } = &catalog.activity else {
        return catalog.clone();
    };
    let mut next = catalog.clone();
    next.activity = Activity::Fetching {
        release: release.clone(),
        part: *part,
        done,
        total,
    };
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_words_in_another_order_fold_to_the_same_list() {
        assert_eq!(
            words("ЧЕРТЁЖ: Баруук Прайм: Нейрооптика"),
            words("Баруук Прайм: Нейрооптика (Чертеж)")
        );
        assert_eq!(words("Лит G12 Реликвия"), words("Реликвия Лит G12"));
    }

    #[test]
    fn names_of_different_words_do_not_fold_together() {
        assert_ne!(
            words("Баруук Прайм: Нейрооптика (Чертеж)"),
            words("Баруук Прайм: Нейрооптика")
        );
    }

    fn version(major: u32, minor: u32, patch: u32) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    fn asset(size: u64) -> Asset {
        Asset {
            file: "catalog.sqlite.gz".into(),
            blake3: "0".repeat(64),
            size,
        }
    }

    fn release(schema: u32, version: Version) -> Release {
        Release {
            schema,
            version,
            db: asset(4096),
            pack: asset(8192),
        }
    }

    /// The release this build would install.
    fn published(version: Version) -> Release {
        release(CATALOG_SCHEMA, version)
    }

    /// A catalog part-way through an install: the database is being fetched.
    fn fetching(release: &Release) -> Catalog {
        Catalog {
            activity: Activity::Fetching {
                release: release.clone(),
                part: Part::Database,
                done: 0,
                total: release.db.size,
            },
            ..Catalog::default()
        }
    }

    #[test]
    fn parses_three_parts() {
        assert_eq!(Version::parse("5.12.3"), Some(version(5, 12, 3)));
    }

    #[test]
    fn rejects_anything_else() {
        assert_eq!(Version::parse("5.12"), None);
        assert_eq!(Version::parse("5.12.3.1"), None);
        assert_eq!(Version::parse("five"), None);
    }

    #[test]
    fn versions_order_by_part() {
        assert!(version(5, 2, 0) > version(5, 1, 9));
        assert!(version(6, 0, 0) > version(5, 9, 9));
    }

    #[test]
    fn a_first_run_installs() {
        let published = release(CATALOG_SCHEMA, version(5, 0, 0));
        assert_eq!(
            plan(None, None, published.clone()),
            Plan::Install(published)
        );
    }

    #[test]
    fn a_newer_version_installs() {
        let have = version(5, 0, 0);
        let published = release(CATALOG_SCHEMA, version(5, 1, 0));
        assert_eq!(
            plan(Some(have), Some(have), published.clone()),
            Plan::Install(published)
        );
    }

    #[test]
    fn a_complete_install_of_the_same_version_is_up_to_date() {
        let have = version(5, 1, 0);
        let published = release(CATALOG_SCHEMA, have);
        assert_eq!(plan(Some(have), Some(have), published), Plan::UpToDate);
    }

    #[test]
    fn a_catalog_without_its_pictures_installs_again() {
        let published = release(CATALOG_SCHEMA, version(5, 1, 0));
        assert_eq!(
            plan(Some(version(5, 1, 0)), None, published.clone()),
            Plan::Install(published)
        );
    }

    #[test]
    fn pictures_left_by_an_older_release_install_again() {
        let published = release(CATALOG_SCHEMA, version(5, 1, 0));
        assert_eq!(
            plan(
                Some(version(5, 1, 0)),
                Some(version(5, 0, 0)),
                published.clone()
            ),
            Plan::Install(published)
        );
    }

    #[test]
    fn another_schema_needs_a_newer_build() {
        let have = version(5, 1, 0);
        let published = release(CATALOG_SCHEMA + 1, version(6, 0, 0));
        assert_eq!(
            plan(Some(have), Some(have), published),
            Plan::ClientTooOld {
                schema: CATALOG_SCHEMA + 1
            }
        );
    }

    #[test]
    fn a_schema_gate_outranks_a_newer_version() {
        let published = release(CATALOG_SCHEMA - 1, version(9, 9, 9));
        assert!(matches!(
            plan(None, None, published),
            Plan::CatalogTooOld { .. }
        ));
    }

    #[test]
    fn an_older_schema_is_not_a_build_left_behind() {
        let published = release(CATALOG_SCHEMA - 1, version(5, 1, 0));
        assert_eq!(
            plan(None, None, published),
            Plan::CatalogTooOld {
                schema: CATALOG_SCHEMA - 1
            }
        );
    }

    #[test]
    fn a_bare_catalog_is_one_missing_either_half() {
        let complete = Catalog {
            installed: Some(version(5, 1, 0)),
            pack: Some(version(5, 1, 0)),
            activity: Activity::Idle,
        };
        assert!(!complete.is_bare());
        assert!(
            Catalog {
                pack: None,
                ..complete.clone()
            }
            .is_bare()
        );
        assert!(
            Catalog {
                installed: None,
                ..complete
            }
            .is_bare()
        );
    }

    #[test]
    fn a_sync_request_starts_a_check() {
        let (next, effects) = update(&Catalog::default(), CatalogEvent::SyncRequested);

        assert_eq!(next.activity, Activity::Checking);
        assert_eq!(effects, vec![Effect::CheckCatalog]);
    }

    #[test]
    fn a_sync_request_while_busy_is_a_noop() {
        let catalog = Catalog {
            activity: Activity::Checking,
            ..Catalog::default()
        };

        let (next, effects) = update(&catalog, CatalogEvent::SyncRequested);

        assert_eq!(next, catalog);
        assert!(effects.is_empty());
    }

    /// A published catalog too old to open leaves nothing installing, so a later check still runs.
    #[test]
    fn a_sync_request_after_a_stale_catalog_checks_again() {
        let catalog = Catalog {
            activity: Activity::Stale {
                schema: CATALOG_SCHEMA - 1,
            },
            ..Catalog::default()
        };

        let (next, effects) = update(&catalog, CatalogEvent::SyncRequested);

        assert_eq!(next.activity, Activity::Checking);
        assert_eq!(effects, vec![Effect::CheckCatalog]);
    }

    #[test]
    fn a_newer_release_fetches_the_database_first() {
        let release = published(version(5, 1, 0));

        let (next, effects) = update(&Catalog::default(), CatalogEvent::Checked(release.clone()));

        assert_eq!(
            next.activity,
            Activity::Fetching {
                total: release.db.size,
                release: release.clone(),
                part: Part::Database,
                done: 0,
            }
        );
        assert_eq!(effects, vec![Effect::FetchDatabase(release)]);
    }

    #[test]
    fn a_release_this_build_cannot_read_stops_at_unsupported() {
        let release = release(CATALOG_SCHEMA + 1, version(6, 0, 0));

        let (next, effects) = update(&Catalog::default(), CatalogEvent::Checked(release));

        assert_eq!(
            next.activity,
            Activity::Unsupported {
                schema: CATALOG_SCHEMA + 1
            }
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn progress_keeps_the_release_and_the_part() {
        let release = published(version(5, 1, 0));

        let next = progressed(&fetching(&release), 2048, release.db.size);

        assert_eq!(
            next.activity,
            Activity::Fetching {
                total: release.db.size,
                release,
                part: Part::Database,
                done: 2048,
            }
        );
    }

    #[test]
    fn progress_outside_a_fetch_is_a_noop() {
        let idle = Catalog::default();

        assert_eq!(progressed(&idle, 2048, 4096), idle);
    }

    #[test]
    fn unpacking_follows_the_fetch() {
        let release = published(version(5, 1, 0));

        let (next, effects) = update(&fetching(&release), CatalogEvent::Opening);

        assert_eq!(next.activity, Activity::Opening { release });
        assert!(effects.is_empty());
    }

    #[test]
    fn unpacking_outside_a_fetch_is_a_noop() {
        let idle = Catalog::default();

        let (next, effects) = update(&idle, CatalogEvent::Opening);

        assert_eq!(next, idle);
        assert!(effects.is_empty());
    }

    #[test]
    fn an_open_catalog_moves_on_to_the_pictures() {
        let release = published(version(5, 1, 0));
        let catalog = Catalog {
            activity: Activity::Opening {
                release: release.clone(),
            },
            ..Catalog::default()
        };

        let (next, effects) = update(&catalog, CatalogEvent::Ready(version(5, 1, 0)));

        assert_eq!(next.installed, Some(version(5, 1, 0)));
        assert_eq!(
            next.activity,
            Activity::Fetching {
                total: release.pack.size,
                release: release.clone(),
                part: Part::Pictures,
                done: 0,
            }
        );
        assert_eq!(effects, vec![Effect::FetchPictures(release)]);
    }

    #[test]
    fn the_install_ends_when_the_pictures_land() {
        let release = published(version(5, 1, 0));
        let catalog = Catalog {
            installed: Some(version(5, 1, 0)),
            ..fetching(&release)
        };

        let (next, effects) = update(&catalog, CatalogEvent::PackReady(version(5, 1, 0)));

        assert_eq!(next.pack, Some(version(5, 1, 0)));
        assert_eq!(next.activity, Activity::Idle);
        assert!(effects.is_empty());
    }

    #[test]
    fn a_failure_keeps_what_is_installed() {
        let catalog = Catalog {
            installed: Some(version(5, 0, 0)),
            pack: Some(version(5, 0, 0)),
            activity: Activity::Checking,
        };
        let reason = CatalogError::Manifest("connection refused".into());

        let (next, _) = update(&catalog, CatalogEvent::Failed(reason.clone()));

        assert_eq!(next.installed, Some(version(5, 0, 0)));
        assert_eq!(next.pack, Some(version(5, 0, 0)));
        assert_eq!(next.activity, Activity::Unreachable { reason });
    }

    #[test]
    fn folds_case_and_yo() {
        assert_eq!(fold("Дирижёр"), "дирижер");
        assert_eq!(fold("Volt Prime"), "volt prime");
    }
}
