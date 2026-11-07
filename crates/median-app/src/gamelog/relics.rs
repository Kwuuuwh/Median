/// Logged when the reward screen opens, before the reward cards are shown.
const OPENS: &str = "VoidProjections: OpenVoidProjectionRewardScreenRMI";

/// Precedes the item this account received from its own relic.
const OWN: &str = " gets reward ";

/// Logged when the reward cards are shown and can be read.
const SHOWN: &str = "ProjectionRewardChoice.lua: Got rewards";

/// Precedes the number of seconds the reward screen stays open.
const COUNTDOWN: &str = "ProjectionsCountdown.lua: Initialize timer";

/// Logged when the reward screen closes, in one of three forms: the countdown ends, a reward is
/// chosen, or the host closes the screen.
const GONE: [&str; 3] = [
    "ProjectionRewardChoice.lua: Relic reward screen shut down",
    "ProjectionRewardChoice.lua: Selection countdown done",
    "VoidProjections: CloseVoidProjectionRewardScreen",
];

/// The game log names an item under `/Lotus/StoreItems/`, the catalog under `/Lotus/Types/`.
const STORE: &str = "/Lotus/StoreItems/";

/// Whether the line opens the reward screen.
pub(super) fn opens(line: &str) -> bool {
    line.contains(OPENS)
}

/// Whether the line logs that the reward cards are shown.
pub(super) fn shown(line: &str) -> bool {
    line.contains(SHOWN)
}

/// Whether the line closes the reward screen.
pub(super) fn gone(line: &str) -> bool {
    GONE.iter().any(|said| line.contains(said))
}

/// Unique name, as the catalog spells it, of the item this account received from its relic;
/// `None` for other lines.
pub(super) fn own(line: &str) -> Option<String> {
    if !line.contains("VoidProjections:") {
        return None;
    }
    let item = line.split(OWN).nth(1)?.trim();
    item.starts_with('/').then(|| catalogued(item))
}

/// Seconds the reward screen stays open; `None` for other lines.
pub(super) fn countdown(line: &str) -> Option<u32> {
    line.contains(COUNTDOWN)
        .then(|| line.rsplit(char::is_whitespace).next())
        .flatten()?
        .parse()
        .ok()
}

/// Converts a `/Lotus/StoreItems/` name to the catalog's `/Lotus/` name; other names are
/// returned unchanged.
fn catalogued(item: &str) -> String {
    match item.strip_prefix(STORE) {
        Some(rest) => format!("/Lotus/{rest}"),
        None => item.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines copied verbatim from a real game log.
    const OPENED: &str = "4776.566 Sys [Info]: VoidProjections: OpenVoidProjectionRewardScreenRMI";
    const GOT: &str = "4776.845 Sys [Info]: VoidProjections: 6a2b3287efa0fb861d00c610 gets reward \
         /Lotus/StoreItems/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel";
    const READY: &str = "4776.933 Script [Info]: ProjectionRewardChoice.lua: Got rewards";
    const TIMER: &str =
        "4776.935 Script [Info]: ProjectionsCountdown.lua: Initialize timer nil\t15";
    const SHUT: &str =
        "4791.950 Script [Info]: ProjectionRewardChoice.lua: Relic reward screen shut down";

    #[test]
    fn knows_the_screen_opening_and_closing() {
        assert!(opens(OPENED));
        assert!(shown(READY));
        assert!(gone(SHUT));
        assert!(!opens(READY));
        assert!(!gone(READY));
    }

    #[test]
    fn takes_every_way_the_screen_is_closed() {
        let chosen = "4791.950 Script [Info]: ProjectionRewardChoice.lua: Selection countdown done";
        let torn = "5566.170 Sys [Info]: VoidProjections: CloseVoidProjectionRewardScreen";

        assert!(gone(chosen));
        assert!(gone(torn));
    }

    #[test]
    fn takes_our_own_reward_under_the_name_the_catalog_uses() {
        assert_eq!(
            own(GOT).as_deref(),
            Some("/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel")
        );
    }

    #[test]
    fn leaves_the_other_players_alone() {
        let waiting = "4776.841 Sys [Info]: VoidProjections: Still waiting on response from \
                       6a620496371e27399a01a673";
        let answered = "4776.841 Sys [Info]: VoidProjections: Client got reward info from \
                        6a620496371e27399a01a673";

        assert_eq!(own(waiting), None);
        assert_eq!(own(answered), None);
    }

    #[test]
    fn reads_the_seconds_the_screen_stands() {
        assert_eq!(countdown(TIMER), Some(15));
        assert_eq!(
            countdown("4791.950 Script [Info]: ProjectionsCountdown.lua: Initialize timer nil\t0"),
            Some(0)
        );
        assert_eq!(countdown(READY), None);
    }
}
