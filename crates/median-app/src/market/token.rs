use std::sync::{Mutex, OnceLock};

use keyring::v1::Entry;
use median_core::Standing;

/// Credential store service of the token and the account name: the app's identifier.
static SERVICE: OnceLock<String> = OnceLock::new();

/// Service used before `keep_under` is called.
const UNSET: &str = "com.median.dev";

const ACCOUNT: &str = "warframe.market";
const NAME: &str = "warframe.market name";

/// The credential store serialises writers itself; this lock keeps two writers in this process
/// from racing on it.
static WRITING: Mutex<()> = Mutex::new(());

/// The warframe.market token, stored in the system credential store.
pub struct Token;

fn service() -> &'static str {
    SERVICE.get().map_or(UNSET, String::as_str)
}

impl Token {
    /// Keeps the token under `service` from now on; the first call wins.
    pub fn keep_under(service: &str) {
        let _ = SERVICE.set(service.to_owned());
    }

    /// The stored token; `None` when there is none or the store cannot be read.
    pub fn read() -> Option<String> {
        Entry::new(service(), ACCOUNT).ok()?.get_password().ok()
    }

    /// Stores the token, replacing any previous one.
    pub fn write(value: &str) -> Result<(), String> {
        let _writing = WRITING
            .lock()
            .map_err(|_| "the token lock is poisoned".to_owned())?;
        Entry::new(service(), ACCOUNT)
            .map_err(|e| e.to_string())?
            .set_password(value)
            .map_err(|e| e.to_string())
    }

    /// `In` with the stored account name when a token is stored, else `Out`.
    pub fn standing() -> Standing {
        match Self::read() {
            Some(_) => Standing::In { name: Self::name() },
            None => Standing::Out,
        }
    }

    /// The account's warframe.market name at the time the token was issued.
    pub fn name() -> Option<String> {
        Entry::new(service(), NAME).ok()?.get_password().ok()
    }

    /// Stores the account name next to the token; `None` deletes the stored name.
    pub fn write_name(name: Option<&str>) -> Result<(), String> {
        let entry = Entry::new(service(), NAME).map_err(|e| e.to_string())?;
        match name {
            Some(name) => entry.set_password(name).map_err(|e| e.to_string()),
            None => {
                let _ = entry.delete_credential();
                Ok(())
            }
        }
    }

    /// Deletes the token and the account name from the store; a missing entry is not an error.
    pub fn clear() {
        let Ok(_writing) = WRITING.lock() else { return };
        for account in [ACCOUNT, NAME] {
            if let Ok(entry) = Entry::new(service(), account) {
                let _ = entry.delete_credential();
            }
        }
    }
}
