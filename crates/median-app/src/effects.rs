use std::sync::Arc;

use async_trait::async_trait;
use median_core::Effect;
use median_engine::EffectRunner;
use median_ipc::Rang;
use tracing::warn;

use tauri::AppHandle;
use tauri_specta::Event as _;

use crate::account::Store;
use crate::catalog::Driver;
use crate::layer::Layer;
use crate::market::stream::{Ask, Stream};
use crate::settings::SettingsStore;
use crate::state_sink::to_alarms;

/// Executes core effects against the settings store, the catalog driver, the account store,
/// the warframe.market socket and the overlay.
pub struct AppEffects {
    settings: Arc<SettingsStore>,
    catalog: Arc<Driver>,
    history: Arc<Store>,
    stream: Stream,
    layer: Arc<Layer>,
    app: AppHandle,
}

impl AppEffects {
    pub fn new(
        settings: Arc<SettingsStore>,
        catalog: Arc<Driver>,
        history: Arc<Store>,
        stream: Stream,
        layer: Arc<Layer>,
        app: AppHandle,
    ) -> Self {
        Self {
            settings,
            catalog,
            history,
            stream,
            layer,
            app,
        }
    }
}

#[async_trait]
impl EffectRunner for AppEffects {
    async fn run(&self, effect: Effect) {
        match effect {
            Effect::PersistSettings(settings) => {
                if let Err(e) = self.settings.save(&settings) {
                    warn!(error = %e, "settings save failed");
                }
            }
            Effect::StoreSnapshot(inventory) => {
                if let Err(e) = self.history.keep(inventory).await {
                    warn!(error = %e, "account snapshot not saved");
                }
            }
            Effect::CheckCatalog => self.catalog.check(),
            Effect::FetchDatabase(release) => self.catalog.database(release),
            Effect::FetchPictures(release) => self.catalog.pictures(release),
            Effect::ShowStatus(status) => self.stream.ask(Ask::Status(status)),
            Effect::ShowOverlay(on) => self.layer.want(on),
            Effect::Chime(volume) => crate::sound::chime(volume),
            Effect::Ring(alarms) => {
                let _ = Rang {
                    alarms: to_alarms(&alarms),
                }
                .emit(&self.app);
            }
        }
    }
}
