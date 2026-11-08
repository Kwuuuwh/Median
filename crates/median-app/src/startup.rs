use std::any::Any;
use std::error::Error;
use std::sync::{Mutex, PoisonError};

use median_ipc::StartFailure;
use tauri::{App, Manager};
use tracing::error;

use crate::account::StoreError;
use crate::schema::OpenError;
use crate::settings::SettingsStore;
use crate::window;

/// Why this run did not start, for the launcher; empty after a good start.
#[derive(Default)]
pub struct Failed(Mutex<Option<StartFailure>>);

impl Failed {
    pub fn get(&self) -> Option<StartFailure> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Outcome of `wiring::init` run inside `catch_unwind`.
pub type Started = std::thread::Result<Result<(), Box<dyn Error>>>;

/// After a failed start, keeps why and opens only the launcher, which can still update the app;
/// does nothing after a good start.
pub fn settle(app: &mut App, started: Started) {
    let Some((reason, newer_data)) = failure_of(started) else {
        return;
    };
    error!(reason, newer_data, "median did not start");
    let root = app.path().app_local_data_dir().ok();
    if let Some(root) = &root
        && app.try_state::<crate::commands::DataFolder>().is_none()
    {
        app.manage(crate::commands::DataFolder(root.clone()));
    }
    let locale = root.map_or_else(median_core::Locale::default, |root| {
        SettingsStore::new(root.join("settings.json")).load().locale
    });
    *app.state::<Failed>()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(StartFailure {
        reason,
        newer_data,
        locale: locale.as_code().to_owned(),
    });
    window::fail(app.handle());
}

/// Why a start failed, and whether it failed on account data from a later build; `None` for a
/// good start.
fn failure_of(started: Started) -> Option<(String, bool)> {
    match started {
        Ok(Ok(())) => None,
        Ok(Err(why)) => {
            let newer = matches!(
                why.downcast_ref::<StoreError>(),
                Some(StoreError::Open(OpenError::TooNew { .. }))
            );
            Some((why.to_string(), newer))
        }
        Err(panic) => Some((panic_text(panic.as_ref()), false)),
    }
}

/// The message a panic was raised with.
fn panic_text(panic: &(dyn Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the start panicked".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_start_is_no_failure() {
        assert_eq!(failure_of(Ok(Ok(()))), None);
    }

    #[test]
    fn account_data_from_a_later_build_is_told_apart() {
        let newer: Box<dyn Error> = Box::new(StoreError::Open(OpenError::TooNew {
            found: 30,
            version: 27,
        }));
        let other: Box<dyn Error> = Box::new(StoreError::Nobody);

        assert!(matches!(failure_of(Ok(Err(newer))), Some((_, true))));
        assert!(matches!(failure_of(Ok(Err(other))), Some((_, false))));
    }

    #[test]
    fn a_panic_keeps_its_message() {
        let panic = std::panic::catch_unwind(|| -> Result<(), Box<dyn Error>> {
            panic!("the tables land")
        });

        assert_eq!(
            failure_of(panic),
            Some(("the tables land".to_owned(), false))
        );
    }
}
