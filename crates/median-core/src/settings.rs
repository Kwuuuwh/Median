use crate::locale::Locale;
use crate::policy::Policy;
use crate::watch::Watch;

/// Accent colour of the interface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Accent {
    #[default]
    Ice,
    Indigo,
    Amethyst,
    Orchid,
    Steel,
    Terracotta,
}

impl Accent {
    /// Every accent, in the order the interface offers them.
    pub const ALL: [Accent; 6] = [
        Accent::Ice,
        Accent::Indigo,
        Accent::Amethyst,
        Accent::Orchid,
        Accent::Steel,
        Accent::Terracotta,
    ];

    /// Code the accent is stored and requested under, e.g. `"ice"`.
    pub fn as_code(self) -> &'static str {
        match self {
            Accent::Ice => "ice",
            Accent::Indigo => "indigo",
            Accent::Amethyst => "amethyst",
            Accent::Orchid => "orchid",
            Accent::Steel => "steel",
            Accent::Terracotta => "terracotta",
        }
    }

    /// Accent with this code; `None` when no accent has it.
    pub fn parse(code: &str) -> Option<Accent> {
        Accent::ALL.into_iter().find(|a| a.as_code() == code)
    }
}

/// Sound alert settings for game log events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    /// Whether a whisper in the game plays a sound.
    pub whisper: bool,
    /// Alert volume, in percent.
    pub volume: u8,
}

impl Default for Alert {
    fn default() -> Self {
        Self {
            whisper: true,
            volume: 25,
        }
    }
}

/// User settings, saved to disk: interface, agent limits, sound alerts and watches.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    pub locale: Locale,
    pub accent: Accent,
    /// Limits the trading agents act within.
    pub policy: Policy,
    /// Sound alert settings for game log events.
    pub alert: Alert,
    /// World state events the user is notified about.
    pub watches: Vec<Watch>,
    /// Whether the market status follows the game: "in game" while it runs, "invisible" otherwise.
    pub follow_game: bool,
    /// Release channel the app takes updates from; `None` for the channel the build was made for.
    pub update_channel: Option<UpdateChannel>,
}

/// A release channel updates can come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannel {
    /// Released from `main`.
    Stable,
    /// Released from `preview`, ahead of the stable channel.
    Preview,
}

impl UpdateChannel {
    /// Code the channel is stored and published under: `stable` or `preview`.
    pub fn as_code(self) -> &'static str {
        match self {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Preview => "preview",
        }
    }

    /// Channel with this code; `None` for any other code, `dev` included.
    pub fn parse(code: &str) -> Option<UpdateChannel> {
        [UpdateChannel::Stable, UpdateChannel::Preview]
            .into_iter()
            .find(|channel| channel.as_code() == code)
    }
}

#[cfg(test)]
mod tests {
    use super::Accent;

    #[test]
    fn every_accent_answers_to_its_own_code() {
        for accent in Accent::ALL {
            assert_eq!(Accent::parse(accent.as_code()), Some(accent));
        }
    }

    #[test]
    fn an_unknown_code_is_no_accent() {
        assert_eq!(Accent::parse("chartreuse"), None);
    }
}
