use std::sync::Arc;

use crate::account::Inventory;
use crate::catalog::Release;
use crate::membership::Status;
use crate::settings::Settings;
use crate::watch::Alarm;

/// Side effect returned by `update`; the app performs it.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Write the settings to disk.
    PersistSettings(Settings),
    /// Save this inventory reading to the account store.
    StoreSnapshot(Arc<Inventory>),
    /// Fetch the published game data manifest.
    CheckCatalog,
    /// Download, verify and open this release's database.
    FetchDatabase(Release),
    /// Download the images referenced by this release's catalog.
    FetchPictures(Release),
    /// Set this status on warframe.market.
    ShowStatus(Status),
    /// Show (`true`) or hide (`false`) the overlay over the game.
    ShowOverlay(bool),
    /// Notify the user that these watches fired.
    Ring(Vec<Alarm>),
    /// Play the chime at this volume, in percent.
    Chime(u8),
}
