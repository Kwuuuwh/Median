use std::fs;
use std::io;
use std::path::PathBuf;

use median_core::{Accent, Alert, Capped, Locale, Policy, Settings, UpdateChannel, Watch};
use serde::{Deserialize, Serialize};

/// On-disk form of the settings; a missing or unreadable field falls back to its default.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    locale: Option<String>,
    accent: Option<String>,
    policy: Option<StoredPolicy>,
    alert: Option<StoredAlert>,
    /// Watches as their codes; a code this build cannot parse is dropped.
    watches: Option<Vec<String>>,
    /// Whether the warframe.market status follows the game: "in game" while it runs.
    follow_game: Option<bool>,
    /// Release channel updates come from: `stable` or `preview`; absent for the build's own.
    update_channel: Option<String>,
}

/// On-disk form of the sound alert settings for game log events.
#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredAlert {
    whisper: Option<bool>,
    volume: Option<u8>,
}

impl StoredAlert {
    fn into_alert(self) -> Alert {
        let fallback = Alert::default();
        Alert {
            whisper: self.whisper.unwrap_or(fallback.whisper),
            volume: self.volume.unwrap_or(fallback.volume),
        }
    }

    fn of(alert: &Alert) -> Self {
        Self {
            whisper: Some(alert.whisper),
            volume: Some(alert.volume),
        }
    }
}

/// On-disk form of the agent policy; every field is optional so a file from an older build
/// still loads.
#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredPolicy {
    #[serde(alias = "least_per_exchange")]
    least_profit: Option<u32>,
    tied_from_purse: Option<bool>,
    purse_share: Option<u32>,
    least_exchanges: Option<u32>,
    least_per_day: Option<u32>,
    most_per_position: Option<u32>,
    most_tied: Option<u32>,
    most_per_trade: Option<u32>,
    most_per_order: Option<u32>,
    sale_days: Option<u32>,
    blacklist: Option<Vec<String>>,
    whitelist: Option<Vec<StoredCapped>>,
    rivens: Option<bool>,
    rank_up: Option<bool>,
    riven_capital: Option<u32>,
    riven_share: Option<u32>,
    riven_margin: Option<u32>,
    riven_least_profit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCapped {
    item: String,
    most: u32,
}

impl StoredPolicy {
    fn into_policy(self) -> Policy {
        let fallback = Policy::default();
        Policy {
            least_profit: self.least_profit.unwrap_or(fallback.least_profit),
            tied_from_purse: self.tied_from_purse.unwrap_or(fallback.tied_from_purse),
            purse_share: self.purse_share.unwrap_or(fallback.purse_share),
            least_exchanges: self.least_exchanges.unwrap_or(fallback.least_exchanges),
            least_per_day: self.least_per_day.unwrap_or(fallback.least_per_day),
            most_per_position: self.most_per_position.unwrap_or(fallback.most_per_position),
            most_tied: self.most_tied.unwrap_or(fallback.most_tied),
            most_per_trade: self.most_per_trade.unwrap_or(fallback.most_per_trade),
            most_per_order: self.most_per_order.unwrap_or(fallback.most_per_order),
            sale_days: self.sale_days.unwrap_or(fallback.sale_days),
            blacklist: self.blacklist.unwrap_or_default(),
            whitelist: self
                .whitelist
                .unwrap_or_default()
                .into_iter()
                .map(|line| Capped {
                    item: line.item,
                    most: line.most,
                })
                .collect(),
            rivens: self.rivens.unwrap_or(fallback.rivens),
            rank_up: self.rank_up.unwrap_or(fallback.rank_up),
            riven_capital: self.riven_capital.unwrap_or(fallback.riven_capital),
            riven_share: self.riven_share.unwrap_or(fallback.riven_share),
            riven_margin: self.riven_margin.unwrap_or(fallback.riven_margin),
            riven_least_profit: self
                .riven_least_profit
                .unwrap_or(fallback.riven_least_profit),
        }
    }

    fn of(policy: &Policy) -> Self {
        Self {
            least_profit: Some(policy.least_profit),
            tied_from_purse: Some(policy.tied_from_purse),
            purse_share: Some(policy.purse_share),
            least_exchanges: Some(policy.least_exchanges),
            least_per_day: Some(policy.least_per_day),
            most_per_position: Some(policy.most_per_position),
            most_tied: Some(policy.most_tied),
            most_per_trade: Some(policy.most_per_trade),
            most_per_order: Some(policy.most_per_order),
            sale_days: Some(policy.sale_days),
            blacklist: Some(policy.blacklist.clone()),
            whitelist: Some(
                policy
                    .whitelist
                    .iter()
                    .map(|line| StoredCapped {
                        item: line.item.clone(),
                        most: line.most,
                    })
                    .collect(),
            ),
            rivens: Some(policy.rivens),
            rank_up: Some(policy.rank_up),
            riven_capital: Some(policy.riven_capital),
            riven_share: Some(policy.riven_share),
            riven_margin: Some(policy.riven_margin),
            riven_least_profit: Some(policy.riven_least_profit),
        }
    }
}

/// Reads and writes user settings on disk.
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// Creates a store for the JSON file at `path`.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Loads the saved settings; a missing or unparsable file or field takes its default.
    pub fn load(&self) -> Settings {
        let stored = fs::read_to_string(&self.path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Stored>(&raw).ok())
            .unwrap_or_default();

        Settings {
            locale: stored
                .locale
                .as_deref()
                .and_then(Locale::parse)
                .unwrap_or_default(),
            accent: stored
                .accent
                .as_deref()
                .and_then(Accent::parse)
                .unwrap_or_default(),
            policy: stored.policy.unwrap_or_default().into_policy(),
            alert: stored.alert.unwrap_or_default().into_alert(),
            watches: stored
                .watches
                .unwrap_or_default()
                .iter()
                .filter_map(|code| Watch::parse(code))
                .collect(),
            follow_game: stored.follow_game.unwrap_or(true),
            update_channel: stored
                .update_channel
                .as_deref()
                .and_then(UpdateChannel::parse),
        }
    }

    /// Writes `settings` to the file as pretty-printed JSON, replacing its contents.
    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        let stored = Stored {
            locale: Some(settings.locale.as_code().to_string()),
            accent: Some(settings.accent.as_code().to_string()),
            policy: Some(StoredPolicy::of(&settings.policy)),
            alert: Some(StoredAlert::of(&settings.alert)),
            watches: Some(settings.watches.iter().map(Watch::as_code).collect()),
            follow_game: Some(settings.follow_game),
            update_channel: settings
                .update_channel
                .map(|channel| channel.as_code().to_string()),
        };
        let raw = serde_json::to_string_pretty(&stored).map_err(io::Error::other)?;
        fs::write(&self.path, raw)
    }
}
