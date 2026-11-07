/// Logged when the riven cycling screen opens.
const OPENS: &str = "Created /Lotus/Interface/OmegaRerollSelection.swf";

/// Logged when the level under the cycling screen is unloaded, which closes the screen.
const CLOSES: [&str; 2] = ["NpcManager::ClearAgents()", "ytes of recycled effects"];

/// Precedes the text of a confirmation dialog.
const ASKS: &str = "Dialog::CreateOkCancel(description=";

/// Follows the text of that confirmation dialog.
const ASKED: &str = ", title=";

/// Dialog result that confirms the dialog (OK).
const CONFIRMED: &str = "Dialog::SendResult(4)";

/// Prefix of any dialog result.
const ANSWERED: &str = "Dialog::SendResult(";

/// Phrases of the cycling dialogs in one game language.
struct Words {
    /// Precedes the weapon and the riven's name.
    cycle: &'static str,
    /// Separates the riven from its kuva cost.
    costs: &'static str,
    /// Question that confirms the selected riven roll.
    keep: &'static str,
}

const SPOKEN: [Words; 2] = [
    Words {
        cycle: "Вы уверены, что хотите изменить характеристики мода ",
        costs: " за ",
        keep: "Создать выбранный мод Разлома?",
    },
    Words {
        cycle: "Are you sure you want to cycle ",
        costs: " for ",
        keep: "Cycle Riven into current selection?",
    },
];

/// One riven cycle the game asks the player to confirm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    /// Weapon name in the game's language.
    pub weapon: String,
    /// Riven name, e.g. `Hexa-visitis`.
    pub name: String,
    /// Kuva the cycle costs.
    pub kuva: u32,
}

/// Kind of confirmation dialog on the cycling screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Cycle(Cycle),
    Keep,
}

/// Whether the line opens the cycling screen.
pub(super) fn opens(line: &str) -> bool {
    line.contains(OPENS)
}

/// Whether the line closes the cycling screen.
pub(super) fn closes(line: &str) -> bool {
    CLOSES.iter().any(|said| line.contains(said))
}

/// Cycling confirmation the line opens; `None` for other lines and other dialogs.
pub(super) fn asked(line: &str) -> Option<Asked> {
    let text = line.split_once(ASKS)?.1;
    let text = text.split(ASKED).next()?;
    SPOKEN.iter().find_map(|words| {
        if text.contains(words.keep) {
            return Some(Asked::Keep);
        }
        cycle(words, text).map(Asked::Cycle)
    })
}

/// `Some(confirmed)` when the line logs a dialog result, `None` otherwise.
pub(super) fn answered(line: &str) -> Option<bool> {
    line.contains(ANSWERED).then(|| line.contains(CONFIRMED))
}

fn cycle(words: &Words, text: &str) -> Option<Cycle> {
    let rest = text.split_once(words.cycle)?.1;
    let (riven, price) = rest.rsplit_once(words.costs)?;
    let kuva = price
        .trim_end_matches('?')
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()?;
    let (weapon, name) = riven.trim().rsplit_once(' ')?;
    Some(Cycle {
        weapon: weapon.trim().to_owned(),
        name: name.to_owned(),
        kuva,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASKED_RU: &str = "812.400 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы уверены, что хотите изменить характеристики мода Парные Токсоцисты Hexa-visitis за 3,500?, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)";

    #[test]
    fn a_cycle_names_the_weapon_the_riven_and_the_kuva() {
        assert_eq!(
            asked(ASKED_RU),
            Some(Asked::Cycle(Cycle {
                weapon: "Парные Токсоцисты".into(),
                name: "Hexa-visitis".into(),
                kuva: 3_500,
            }))
        );
    }

    #[test]
    fn keeping_the_chosen_riven_is_asked_apart() {
        let line = "830.100 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Создать выбранный мод Разлома?, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)";
        assert_eq!(asked(line), Some(Asked::Keep));
    }

    #[test]
    fn other_confirmations_are_not_cycling() {
        let line = "205.809 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы уверены, что хотите создать 'ИНЖЕКТОР ДЕТОНИТА'?, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)";
        assert_eq!(asked(line), None);
    }

    #[test]
    fn an_answer_confirms_only_with_the_ok() {
        assert_eq!(
            answered("206.556 Script [Info]: Dialog.lua: Dialog::SendResult(4)"),
            Some(true)
        );
        assert_eq!(
            answered("206.556 Script [Info]: Dialog.lua: Dialog::SendResult(1)"),
            Some(false)
        );
        assert_eq!(
            answered("206.556 Script [Info]: Dialog.lua: SendResult_MENU_SELECT()"),
            None
        );
    }

    #[test]
    fn the_screen_opens_on_its_own_swf_and_closes_with_its_level() {
        let open = "800.001 Sys [Info]: Created /Lotus/Interface/OmegaRerollSelection.swf";
        assert!(opens(open));
        assert!(!closes(open));
        assert!(!closes(
            "800.500 Sys [Info]: Created /Lotus/Interface/Dialog.swf"
        ));
        assert!(closes(
            "900.200 AI [Info]: NpcManager::ClearAgents() ReadyToCreateAgents = false"
        ));
    }
}
