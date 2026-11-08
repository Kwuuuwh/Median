use std::sync::{Mutex, PoisonError};

use median_core::{Status, UpdateChannel};
use median_ipc::{Release, UpdateProgressed};
use tauri::{AppHandle, Manager, Url};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_specta::Event as _;
use tracing::{info, warn};

use crate::market::stream::{Ask, Stream};
use crate::revision;
use crate::state_sink::to_release_channel;
use crate::window;

/// Update manifest URL with `{channel}`; empty when the build takes no updates.
const SOURCE: &str = env!("MEDIAN_UPDATE_SOURCE");

/// The update the last check found, until it is installed; held in Tauri state.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

/// Channel the build was made for; `None` for a local build.
pub fn own_channel() -> Option<UpdateChannel> {
    UpdateChannel::parse(revision::CHANNEL)
}

/// Whether the build takes updates: a released channel, an update source and a signing key.
pub fn enabled(app: &AppHandle) -> bool {
    own_channel().is_some() && !SOURCE.is_empty() && signed(app)
}

/// Whether `tauri.conf.json` holds the public key releases are signed with.
fn signed(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|updater| updater.get("pubkey"))
        .and_then(|key| key.as_str())
        .is_some_and(|key| !key.is_empty())
}

/// Update manifest URL of `channel`.
fn endpoint(source: &str, channel: UpdateChannel) -> Result<Url, String> {
    Url::parse(&source.replace("{channel}", channel.as_code())).map_err(|e| e.to_string())
}

/// Revision and channel of a release from its Tauri version: `0.0.<revision>` for stable,
/// `0.0.<revision>-preview` for preview.
fn release_of(version: &str) -> Option<(u32, UpdateChannel)> {
    let version = version.trim_start_matches('v');
    let (number, channel) = match version.split_once('-') {
        Some((number, label)) => (number, UpdateChannel::parse(label)?),
        None => (version, UpdateChannel::Stable),
    };
    if channel == UpdateChannel::Stable && number != version {
        return None;
    }
    let mut parts = number.split('.');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("0"), Some("0"), Some(revision), None) => Some((revision.parse().ok()?, channel)),
        _ => None,
    }
}

/// Whether the announcement marks the release as one installed builds must take before going on.
fn required(announcement: &serde_json::Value) -> bool {
    announcement
        .get("required")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// Revision of a newer release found on `channel`, `None` when its version does not read as one;
/// an error only for a version that reads as another channel's release.
fn offered(version: &str, channel: UpdateChannel) -> Result<Option<u32>, String> {
    match release_of(version) {
        Some((_, made_for)) if made_for != channel => Err(format!(
            "release {version} is a {} release, not a {} one",
            made_for.as_code(),
            channel.as_code()
        )),
        read => Ok(read.map(|(revision, _)| revision)),
    }
}

/// Checks `chosen`, or the build's own channel when `None`, for a release newer than this build
/// and keeps it for `install`; `None` when there is none.
pub async fn check(
    app: &AppHandle,
    chosen: Option<UpdateChannel>,
) -> Result<Option<Release>, String> {
    let own = own_channel()
        .filter(|_| enabled(app))
        .ok_or("this build takes no updates")?;
    let channel = chosen.unwrap_or(own);

    let mut builder = app
        .updater_builder()
        .endpoints(vec![endpoint(SOURCE, channel)?])
        .map_err(|e| e.to_string())?;
    // Sets the warframe.market status to invisible before the installer exits the app; a start
    // that failed has no market stream.
    if let Some(stream) = app.try_state::<Stream>() {
        let stream = stream.inner().clone();
        builder = builder.on_before_exit(move || {
            stream.ask(Ask::Status(Status::Invisible));
            std::thread::sleep(window::PARTING);
        });
    }
    let found = builder
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| {
            warn!(channel = channel.as_code(), error = %e, "update check failed");
            e.to_string()
        })?;
    let release = match &found {
        Some(update) => {
            let revision = offered(&update.version, channel).inspect_err(|why| {
                warn!(channel = channel.as_code(), error = %why, "update refused");
            })?;
            info!(
                channel = channel.as_code(),
                version = %update.version,
                revision,
                "update found"
            );
            Some(Release {
                revision,
                version: update.version.clone(),
                channel: to_release_channel(channel),
                changes: update
                    .body
                    .as_deref()
                    .map(revision::lines)
                    .unwrap_or_default(),
                published: update
                    .date
                    .and_then(|date| u32::try_from(date.unix_timestamp()).ok()),
                required: required(&update.raw_json),
            })
        }
        None => None,
    };
    *app.state::<Pending>()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = found;
    Ok(release)
}

/// Downloads and installs the update the last check found; progress arrives as
/// `UpdateProgressed`.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .state::<Pending>()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .ok_or("no update found to install")?;
    info!(version = %update.version, "update downloading");

    let handle = app.clone();
    let mut downloaded: u64 = 0;
    let mut shown: Option<u64> = None;
    update
        .download_and_install(
            move |part, total| {
                downloaded += part as u64;
                let percent = total.map(|total| downloaded * 100 / total.max(1));
                if percent.is_some() && percent == shown {
                    return;
                }
                shown = percent;
                let _ = UpdateProgressed {
                    downloaded: saturated(downloaded),
                    total: total.map(saturated),
                }
                .emit(&handle);
            },
            || info!("update downloaded"),
        )
        .await
        .map_err(|e| {
            warn!(error = %e, "update install failed");
            e.to_string()
        })
}

/// `bytes` as a `u32`, capped at `u32::MAX`.
fn saturated(bytes: u64) -> u32 {
    u32::try_from(bytes).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_channel_goes_into_the_announcement_url() {
        let source =
            "https://github.com/owner/median/releases/download/updater-{channel}/latest.json";
        assert_eq!(
            endpoint(source, UpdateChannel::Preview).unwrap().as_str(),
            "https://github.com/owner/median/releases/download/updater-preview/latest.json"
        );
    }

    #[test]
    fn the_signing_key_reads_as_the_plugin_reads_it() {
        use base64::Engine as _;

        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let key = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
        let text = base64::engine::general_purpose::STANDARD
            .decode(key)
            .unwrap();
        minisign_verify::PublicKey::decode(std::str::from_utf8(&text).unwrap()).unwrap();
    }

    #[test]
    fn a_release_is_read_from_a_zero_zero_version_with_its_channel() {
        assert_eq!(release_of("0.0.8217"), Some((8217, UpdateChannel::Stable)));
        assert_eq!(release_of("v0.0.12"), Some((12, UpdateChannel::Stable)));
        assert_eq!(
            release_of("0.0.12-preview"),
            Some((12, UpdateChannel::Preview))
        );
        assert_eq!(release_of("0.0.3-stable"), None);
        assert_eq!(release_of("0.0.3-beta"), None);
        assert_eq!(release_of("1.0.3"), None);
        assert_eq!(release_of("0.0"), None);
    }

    #[test]
    fn a_newer_release_is_offered_whatever_its_version_says_unless_it_names_another_channel() {
        assert_eq!(
            offered("0.0.17-preview", UpdateChannel::Preview),
            Ok(Some(17))
        );
        assert_eq!(offered("1.2.0", UpdateChannel::Preview), Ok(None));
        assert_eq!(offered("0.0.18-beta", UpdateChannel::Stable), Ok(None));
        assert!(offered("0.0.17", UpdateChannel::Preview).is_err());
        assert!(offered("0.0.17-preview", UpdateChannel::Stable).is_err());
    }

    #[test]
    fn a_release_is_required_only_when_its_announcement_says_so() {
        assert!(required(
            &serde_json::json!({ "version": "0.0.18", "required": true })
        ));
        assert!(!required(
            &serde_json::json!({ "version": "0.0.18", "required": false })
        ));
        assert!(!required(&serde_json::json!({ "version": "0.0.18" })));
        assert!(!required(
            &serde_json::json!({ "version": "0.0.18", "required": "yes" })
        ));
    }

    #[test]
    fn a_preview_comes_before_the_stable_of_its_revision_and_after_the_one_before() {
        let version = |text: &str| semver::Version::parse(text).unwrap();
        assert!(version("0.0.5-preview") < version("0.0.5"));
        assert!(version("0.0.5") < version("0.0.6-preview"));
        assert!(version("0.0.5-preview") < version("0.0.6-preview"));
    }

    #[test]
    fn a_download_past_four_gigabytes_is_capped() {
        assert_eq!(saturated(1_000), 1_000);
        assert_eq!(saturated(u64::from(u32::MAX) + 1), u32::MAX);
    }
}
