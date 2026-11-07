/// Precedes the JSON with the node the host is about to load: `{"name":"SolNode236_Hard"}`.
const ANNOUNCED: &str = "Host loading {\"name\":\"";

/// Logged when the game opens a level; when the level is a node, the node was announced before.
const OPENS: &str = "ServerFramework:OpenLevel";

/// Start of the line that logs the node a mission was loaded at, whoever hosts it.
const MISSION: &str = "SyncAutoPopulatedConsumables for mission";

/// Precedes the node tag in that line.
const LOCATED: &str = " with location ";

/// Suffix of the tag of a node played on the Steel Path.
const STEEL: &str = "_Hard";

/// Node the host announces it is loading, without the Steel Path suffix; `None` for other lines.
pub(super) fn announced(line: &str) -> Option<String> {
    let (_, rest) = line.split_once(ANNOUNCED)?;
    let (tag, _) = rest.split_once('"')?;
    node(tag)
}

/// Whether the line opens a level.
pub(super) fn opens(line: &str) -> bool {
    line.contains(OPENS)
}

/// Node a mission was loaded at, without the Steel Path suffix; `None` for other lines.
pub(super) fn located(line: &str) -> Option<String> {
    let (_, rest) = line.split_once(MISSION)?.1.split_once(LOCATED)?;
    node(rest.split_whitespace().next()?)
}

/// Node tag as the catalog keys it, with the Steel Path suffix removed; `None` when empty.
fn node(tag: &str) -> Option<String> {
    let tag = tag.trim().trim_end_matches(STEEL);
    (!tag.is_empty()).then(|| tag.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_names_the_node_it_loads_the_steel_path_left_off() {
        let line = "119.236 Script [Info]: ThemedSquadOverlay.lua: Host loading \
                    {\"name\":\"SolNode236_Hard\"} with MissionInfo: ";

        assert_eq!(announced(line).as_deref(), Some("SolNode236"));
        assert_eq!(located(line), None);
    }

    #[test]
    fn a_mission_says_where_it_was_loaded() {
        let line = "125.135 Sys [Info]: SyncAutoPopulatedConsumables for mission MT_LANDSCAPE \
                    with location SolNode236";

        assert_eq!(located(line).as_deref(), Some("SolNode236"));
    }

    #[test]
    fn a_level_opening_is_told_apart_from_a_level_named() {
        assert!(opens(
            "7158.116 Game [Info]: ServerFramework:OpenLevel - \
             /Lotus/Levels/Proc/TheNewWar/PartTwo/TNWDrifterCampMain/BoC8ACA.lp"
        ));
        assert!(!opens(
            "512.064 Game [Info]: Level=/Lotus/Levels/Duviri/WarframeArenas/\
             WFArenasEchoesOfDuviri/FractalPasturesCombat.level, region index=1, layer=255"
        ));
    }
}
