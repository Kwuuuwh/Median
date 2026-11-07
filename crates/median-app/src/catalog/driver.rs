use std::sync::Arc;
use std::time::{Duration, Instant};

use median_core::{CatalogError, CatalogEvent, Domain, Event, Progress, Release};
use median_engine::{Input, Raw};
use tokio::sync::mpsc::Sender;
use tracing::{Instrument, Span, debug, info, info_span, warn};

use crate::catalog::db::Catalog;
use crate::catalog::pack;
use crate::catalog::paths::Layout;
use crate::catalog::source::{self, Source};
use crate::catalog::unpack::unpack;

/// Minimum interval between progress inputs; short enough for the progress bar to move smoothly
/// rather than in steps.
const PROGRESS_EVERY: Duration = Duration::from_millis(120);

/// Runs catalog effects (manifest check, database and picture downloads) and sends their results
/// to the engine as inputs.
pub struct Driver {
    source: Source,
    layout: Arc<Layout>,
    catalog: Arc<Catalog>,
    inputs: Sender<Input>,
}

impl Driver {
    pub fn new(
        source: Source,
        layout: Arc<Layout>,
        catalog: Arc<Catalog>,
        inputs: Sender<Input>,
    ) -> Self {
        Self {
            source,
            layout,
            catalog,
            inputs,
        }
    }

    /// Fetches the published manifest in a background task and sends it as
    /// `Raw::CatalogManifest`.
    pub fn check(self: &Arc<Self>) {
        let driver = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let span = info_span!("catalog.check");
            match driver
                .source
                .text(source::MANIFEST)
                .instrument(span.clone())
                .await
            {
                Ok(body) => {
                    debug!(parent: &span, bytes = body.len(), "catalog manifest read");
                    let _ = driver
                        .inputs
                        .send(Input::Raw(Raw::CatalogManifest(body)))
                        .await;
                }
                Err(reason) => driver.failed(&span, CatalogError::Manifest(reason)).await,
            }
        });
    }

    /// Downloads, verifies and installs the release's database in a background task, then sends
    /// `Raw::CatalogVersion`.
    pub fn database(self: &Arc<Self>, release: Release) {
        let driver = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let span = info_span!("catalog.database", version = %release.version);

            let archive = driver
                .source
                .download(
                    &release.db.file,
                    release.db.size,
                    reporter(driver.inputs.clone()),
                )
                .instrument(span.clone())
                .await;

            let archive = match archive {
                Ok(bytes) => bytes,
                Err(reason) => {
                    return driver.failed(&span, CatalogError::Database(reason)).await;
                }
            };

            let _ = driver
                .inputs
                .send(Input::Command(Event::Catalog(CatalogEvent::Opening)))
                .await;

            let layout = Arc::clone(&driver.layout);
            let opened =
                tauri::async_runtime::spawn_blocking(move || unpack(&layout, &release, &archive))
                    .await;

            match opened {
                Ok(Ok(open)) => {
                    let version = open.version().to_string();
                    driver.catalog.install(open);
                    info!(parent: &span, %version, "catalog installed");
                    let _ = driver
                        .inputs
                        .send(Input::Raw(Raw::CatalogVersion(version)))
                        .await;
                }
                Ok(Err(reason)) => driver.failed(&span, CatalogError::Install(reason)).await,
                Err(e) => {
                    driver
                        .failed(&span, CatalogError::Install(e.to_string()))
                        .await;
                }
            }
        });
    }

    /// Updates the picture pack to `release` in a background task, then sends
    /// `Raw::PackVersion`.
    pub fn pictures(self: &Arc<Self>, release: Release) {
        let driver = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let span = info_span!("catalog.pictures", version = %release.version);

            let synced = pack::sync(
                &driver.source,
                &driver.layout,
                &release,
                reporter(driver.inputs.clone()),
            )
            .instrument(span.clone())
            .await;

            match synced {
                Ok(()) => {
                    let version = release.version.to_string();
                    info!(parent: &span, %version, "pictures installed");
                    let _ = driver
                        .inputs
                        .send(Input::Raw(Raw::PackVersion(version)))
                        .await;
                }
                Err(reason) => driver.failed(&span, CatalogError::Pack(reason)).await,
            }
        });
    }

    async fn failed(&self, span: &Span, reason: CatalogError) {
        warn!(parent: span, error = %reason, "catalog update failed");
        let _ = self
            .inputs
            .send(Input::Command(Event::Catalog(CatalogEvent::Failed(reason))))
            .await;
    }
}

/// Progress callback that sends at most one progress input per `PROGRESS_EVERY` and drops the
/// updates in between.
fn reporter(inputs: Sender<Input>) -> impl FnMut(u64, u64) {
    let mut last: Option<Instant> = None;
    move |done, total| {
        if last.is_none_or(|at| at.elapsed() >= PROGRESS_EVERY) {
            last = Some(Instant::now());
            let _ = inputs.try_send(Input::Command(Event::Progressed(Progress {
                domain: Domain::Catalog,
                done,
                total,
            })));
        }
    }
}
