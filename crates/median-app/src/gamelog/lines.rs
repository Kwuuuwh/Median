use median_core::{Instant, Outcome, Swap, SwapKind, SwapPart};

use super::reroll::{self, Asked, Cycle};
use super::{nodes, relics};

/// Event recognised in one game log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    /// A private chat tab with `player` was opened, by either side.
    Opened { player: String },
    /// The user sent a private message to `player`.
    Wrote { player: String },
    /// The trade window asks to confirm a trade.
    Proposed(Proposal),
    /// The trade ended with this outcome.
    Settled(Outcome),
    /// The relic reward screen opened, before the reward cards are shown.
    RewardScreen,
    /// Item this account received from its own relic.
    OwnReward { item: String },
    /// The reward cards are shown and can be read.
    RewardsShown,
    /// The reward screen stays open for this many more seconds.
    RewardsCountdown { seconds: u32 },
    /// The relic reward screen closed.
    RewardsGone,
    /// The host is about to load this star-chart node.
    Announced { node: String },
    /// The game opened a level: the node announced before it, or a level that is not a node.
    LevelOpened,
    /// A mission was loaded at this node.
    Located { node: String },
    /// The riven cycling screen opened.
    CycleScreen,
    /// A cycle was confirmed.
    Cycled(Cycle),
    /// The selected riven roll was confirmed on the cycling screen.
    CycleKept,
    /// The riven cycling screen closed.
    CycleScreenGone,
    /// The riven cycling screen is reloading after a roll was confirmed.
    CycleScreenReloading,
}

/// One trade as the trade window shows it: the partner, the items given and the items received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub partner: String,
    pub given: Vec<SwapPart>,
    pub taken: Vec<SwapPart>,
}

impl Proposal {
    /// The trade with its time and outcome, once the game logs how it ended.
    pub fn settled(self, at: Instant, outcome: Outcome) -> Swap {
        Swap {
            at,
            partner: self.partner,
            given: self.given,
            taken: self.taken,
            outcome,
        }
    }
}

/// Phrases of the trade dialog in one game language.
struct Words {
    offers: &'static str,
    /// Ends the line that separates the two sides of the trade.
    follows: &'static str,
    /// Precedes the partner's name in that line.
    from: &'static str,
    done: &'static str,
    failed: &'static str,
    cancelled: &'static str,
    platinum: &'static str,
    credits: &'static str,
}

/// Every language the game log is read in; the phrases that match identify the game's language.
const SPOKEN: [Words; 2] = [
    Words {
        offers: "Вы хотите принять условия сделки? Вы предлагаете",
        follows: " следующее:",
        from: " от ",
        done: "Обмен успешно завершён!",
        failed: "Обмен не удался.",
        cancelled: "Обмен был отменён",
        platinum: "Платина",
        credits: "Кредиты",
    },
    Words {
        offers: "Are you sure you want to accept this trade? You are offering",
        follows: " the following:",
        from: " from ",
        done: "The trade was successful!",
        failed: "The trade failed.",
        cancelled: "The trade was cancelled",
        platinum: "Platinum",
        credits: "Credits",
    },
];

/// Ends the trade text of the dialog line; the button definitions follow it.
const BUTTONS: &str = ", title= leftItem=";

/// Prefix of a private chat tab's channel name; other kinds of tabs use other letters.
const PRIVATE: &str = "channel name: F";

/// Logged when the user sends a private message.
const SENT: &str = "IRC out: PRIVMSG ";

/// Whether the line starts a new log record (`<seconds>.<fraction> …`) rather than continuing
/// the text of the previous one.
fn starts_record(line: &str) -> bool {
    let mut chars = line.chars();
    let mut digits = 0;
    for char in chars.by_ref() {
        match char {
            '0'..='9' => digits += 1,
            '.' if digits > 0 => break,
            _ => return false,
        }
    }
    digits > 0 && chars.next().is_some_and(|char| char.is_ascii_digit())
}

/// `text` trimmed and without the private-use characters the game draws as badges in names.
fn plain(text: &str) -> String {
    text.chars()
        .filter(|char| !matches!(*char, '\u{e000}'..='\u{f8ff}'))
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Player a private chat tab was opened with; `None` for other lines.
fn whisper(line: &str) -> Option<String> {
    let start = line.find(PRIVATE)? + PRIVATE.len();
    let name = plain(line[start..].split(" to index").next()?);
    (!name.is_empty()).then_some(name)
}

/// Rank words the game writes in brackets after a ranked copy, after a shortened name.
const RANKS: [&str; 2] = ["РАНГ", "RANK"];

/// Item name without the bracket the game appends, and the rank from that bracket; `None` when
/// there is no rank.
fn ranked(name: &str) -> (String, Option<u8>) {
    let Some(open) = name.rfind(" (") else {
        return (name.to_owned(), None);
    };
    let Some(said) = name[open + 2..].strip_suffix(')') else {
        return (name.to_owned(), None);
    };

    let rank = RANKS.iter().find_map(|word| {
        let at = said.rfind(word)? + word.len();
        said[at..].trim().parse::<u8>().ok()
    });
    (name[..open].trim().to_owned(), rank)
}

/// One line of the trade dialog as an item of one side: name, count (`x N`), kind and rank.
fn part(words: &Words, line: &str) -> SwapPart {
    let (line, count) = match line.rsplit_once(" x ") {
        Some((name, count)) => match count.trim().parse::<u32>() {
            Ok(count) => (name.trim(), count),
            Err(_) => (line, 1),
        },
        None => (line, 1),
    };

    let kind = match line {
        name if name == words.platinum => SwapKind::Platinum,
        name if name == words.credits => SwapKind::Credits,
        _ => SwapKind::Item,
    };
    let (name, rank) = match kind {
        SwapKind::Item => ranked(line),
        _ => (line.to_owned(), None),
    };
    SwapPart {
        name,
        count,
        kind,
        rank,
    }
}

/// Recipient of a private message the user sent; `None` for other lines.
fn wrote(line: &str) -> Option<String> {
    let start = line.find(SENT)? + SENT.len();
    let name = plain(line[start..].split(' ').next()?);
    (!name.is_empty()).then_some(name)
}

/// Parses the collected trade dialog lines into a proposal; `None` when no partner is found.
fn proposal(words: &Words, block: &[String]) -> Option<Proposal> {
    let mut proposal = Proposal {
        partner: String::new(),
        given: Vec::new(),
        taken: Vec::new(),
    };
    let mut receiving = false;

    for raw in block {
        let line = plain(raw.split(BUTTONS).next().unwrap_or_default());
        if line.is_empty() || line.contains(words.offers) {
            continue;
        }

        // The line between the sides names the partner. The text before the name differs
        // between game builds, so only the end of the line is matched.
        if let Some(said) = line.strip_suffix(words.follows)
            && let Some(start) = said.rfind(words.from)
        {
            proposal.partner = plain(&said[start + words.from.len()..]);
            receiving = true;
            continue;
        }

        match receiving {
            true => proposal.taken.push(part(words, &line)),
            false => proposal.given.push(part(words, &line)),
        }
    }

    (!proposal.partner.is_empty()).then_some(proposal)
}

/// Reward screen event of the line; `None` for other lines.
fn screen(line: &str) -> Option<LogEvent> {
    if relics::opens(line) {
        return Some(LogEvent::RewardScreen);
    }
    if let Some(item) = relics::own(line) {
        return Some(LogEvent::OwnReward { item });
    }
    if relics::shown(line) {
        return Some(LogEvent::RewardsShown);
    }
    if relics::gone(line) {
        return Some(LogEvent::RewardsGone);
    }
    relics::countdown(line).map(|seconds| LogEvent::RewardsCountdown { seconds })
}

/// Level loading event of the line; `None` for other lines.
fn whereabouts(line: &str) -> Option<LogEvent> {
    if let Some(node) = nodes::announced(line) {
        return Some(LogEvent::Announced { node });
    }
    if nodes::opens(line) {
        return Some(LogEvent::LevelOpened);
    }
    nodes::located(line).map(|node| LogEvent::Located { node })
}

/// Seconds after the cycling screen opens during which a level unload belongs to the opening,
/// not to a close.
const OPENING: f64 = 3.0;

/// Seconds after a roll is confirmed during which a screen close means the game is reloading
/// the screen.
const RELOADING: f64 = 12.0;

/// Timestamp in seconds at the start of a game log line.
fn stamp(line: &str) -> Option<f64> {
    line.split(' ').next()?.parse().ok()
}

/// Game log parser fed line by line; returns an event for each line that carries one.
pub struct Reading {
    /// Trade dialog being collected, with the index of its language in `SPOKEN`.
    block: Option<(usize, Vec<String>)>,
    /// Game log time at which the cycling screen opened; `None` while it is closed.
    cycling: Option<f64>,
    /// Game log time of the last roll confirmed on the cycling screen.
    kept: Option<f64>,
    /// Cycling confirmation dialog waiting for its result.
    asked: Option<Asked>,
}

impl Default for Reading {
    fn default() -> Self {
        Self::new()
    }
}

impl Reading {
    /// Riven cycling screen event of the line; `None` for other lines.
    fn cycling(&mut self, line: &str) -> Option<LogEvent> {
        let at = stamp(line);
        if reroll::opens(line) {
            self.cycling = Some(at.unwrap_or_default());
            self.asked = None;
            return Some(LogEvent::CycleScreen);
        }
        let opened = self.cycling?;
        if reroll::closes(line) {
            let at = at.unwrap_or(opened);
            if at - opened < OPENING {
                return None;
            }
            self.asked = None;
            if self.kept.take().is_some_and(|kept| at - kept < RELOADING) {
                self.cycling = Some(at);
                return Some(LogEvent::CycleScreenReloading);
            }
            self.cycling = None;
            return Some(LogEvent::CycleScreenGone);
        }
        if let Some(asked) = reroll::asked(line) {
            self.asked = Some(asked);
            return None;
        }
        let confirmed = reroll::answered(line)?;
        match (self.asked.take()?, confirmed) {
            (Asked::Cycle(cycle), true) => Some(LogEvent::Cycled(cycle)),
            (Asked::Keep, true) => {
                self.kept = at;
                Some(LogEvent::CycleKept)
            }
            (_, false) => None,
        }
    }

    pub fn new() -> Self {
        Self {
            block: None,
            cycling: None,
            kept: None,
            asked: None,
        }
    }

    /// Parses one game log line; `None` when it carries no event or continues a trade dialog
    /// being collected.
    pub fn line(&mut self, line: &str) -> Option<LogEvent> {
        if let Some((spoken, block)) = self.block.as_mut() {
            let spoken = *spoken;
            let closing = line.contains(BUTTONS);
            if closing || !starts_record(line) {
                block.push(line.to_owned());
            }
            if !closing && !starts_record(line) {
                return None;
            }
            let (_, block) = self.block.take()?;
            return proposal(&SPOKEN[spoken], &block).map(LogEvent::Proposed);
        }

        if let Some(events) = self.cycling(line) {
            return Some(events);
        }
        if let Some(events) = screen(line) {
            return Some(events);
        }
        if let Some(events) = whereabouts(line) {
            return Some(events);
        }
        if let Some(player) = whisper(line) {
            return Some(LogEvent::Opened { player });
        }
        if let Some(player) = wrote(line) {
            return Some(LogEvent::Wrote { player });
        }

        for (spoken, words) in SPOKEN.iter().enumerate() {
            if line.contains(words.offers) {
                self.block = Some((spoken, vec![line.to_owned()]));
                return None;
            }
            if line.contains(words.done) {
                return Some(LogEvent::Settled(Outcome::Done));
            }
            if line.contains(words.failed) {
                return Some(LogEvent::Settled(Outcome::Failed));
            }
            if line.contains(words.cancelled) {
                return Some(LogEvent::Settled(Outcome::Cancelled));
            }
        }
        match line.contains("OnTradeAccepted failed") {
            true => Some(LogEvent::Settled(Outcome::Failed)),
            false => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dialog as the game wrote it on 28.08.2026: blank lines between the parts, and
    /// "получаете", not "получите".
    const OFFER: &str = "\
2525.123 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы хотите принять условия сделки? Вы предлагаете:

Платина x 108



и получаете от kafasi следующее:

Мистическая зарядка, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)";

    fn read(text: &str) -> Vec<LogEvent> {
        let mut reading = Reading::new();
        text.lines().filter_map(|line| reading.line(line)).collect()
    }

    #[test]
    fn a_record_is_told_from_the_text_that_runs_over() {
        assert!(starts_record("21983.216 Script [Info]: Dialog.lua: x"));
        assert!(!starts_record("Платина x 108"));
        assert!(!starts_record("и получаете от kafasi следующее:"));
        assert!(!starts_record(""));
    }

    #[test]
    fn a_buy_says_what_goes_and_what_comes() {
        let events = read(OFFER);

        let [LogEvent::Proposed(proposal)] = events.as_slice() else {
            panic!("one proposal, got {events:?}");
        };
        assert_eq!(proposal.partner, "kafasi");
        assert_eq!(
            proposal.given,
            vec![SwapPart {
                name: "Платина".into(),
                count: 108,
                kind: SwapKind::Platinum,
                rank: None
            }]
        );
        assert_eq!(
            proposal.taken,
            vec![SwapPart {
                name: "Мистическая зарядка".into(),
                count: 1,
                kind: SwapKind::Item,
                rank: None
            }]
        );
        let swap = proposal.clone().settled(0, Outcome::Done);
        assert!(swap.buying());
        assert!(!swap.selling());
        assert_eq!(swap.platinum_moved(), 108);
    }

    #[test]
    fn a_sale_is_the_same_exchange_the_other_way_round() {
        let events = read(
            "21983.216 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы хотите принять условия сделки? Вы предлагаете
Мистическая зарядка
и получаете от TrustyPatchesPrime следующее:
Платина x 140
, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)",
        );

        let [LogEvent::Proposed(proposal)] = events.as_slice() else {
            panic!("one proposal, got {events:?}");
        };
        let swap = proposal.clone().settled(0, Outcome::Done);
        assert!(swap.selling());
        assert!(!swap.buying());
        assert_eq!(swap.platinum_moved(), 140);
        assert_eq!(swap.partner, "TrustyPatchesPrime");
    }

    #[test]
    fn a_stack_carries_its_count_and_several_items_all_land() {
        let events = read(
            "21983.216 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы хотите принять условия сделки? Вы предлагаете
Платина x 60
и получаете от Somebody следующее:
Лит G12 Реликвия x 3
Мезо F3 Реликвия
, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)",
        );

        let [LogEvent::Proposed(proposal)] = events.as_slice() else {
            panic!("one proposal, got {events:?}");
        };
        assert_eq!(proposal.taken.len(), 2);
        assert_eq!(proposal.taken[0].count, 3);
        assert_eq!(proposal.taken[0].name, "Лит G12 Реликвия");
        assert_eq!(proposal.taken[1].count, 1);
    }

    #[test]
    fn the_english_game_says_the_same_thing() {
        let events = read(
            "21983.216 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Are you sure you want to accept this trade? You are offering
Platinum x 108
and will receive from kafasi the following:
Arcane Energize
, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)",
        );

        let [LogEvent::Proposed(proposal)] = events.as_slice() else {
            panic!("one proposal, got {events:?}");
        };
        assert_eq!(proposal.partner, "kafasi");
        assert_eq!(proposal.given[0].kind, SwapKind::Platinum);
        assert_eq!(proposal.taken[0].name, "Arcane Energize");
    }

    #[test]
    fn a_dialog_cut_short_by_the_next_record_still_reads() {
        let events = read(
            "21983.216 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы хотите принять условия сделки? Вы предлагаете
Платина x 12
и получаете от Somebody следующее:
Форма Чертёж
21983.900 Sys [Info]: something else entirely",
        );

        let [LogEvent::Proposed(proposal)] = events.as_slice() else {
            panic!("one proposal, got {events:?}");
        };
        assert_eq!(proposal.taken[0].name, "Форма Чертёж");
    }

    #[test]
    fn the_bracket_after_a_mod_carries_its_rank_not_its_name_again() {
        let events = read(
            "260.096 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы хотите принять условия сделки? Вы предлагаете:
Платина x 46
и получаете от 3vialArT следующее:
Гальванизированный Прицел (ГАЛЬВАНИЗИРОВАННЫЙ РАНГ 10), title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)",
        );

        let [LogEvent::Proposed(proposal)] = events.as_slice() else {
            panic!("one proposal, got {events:?}");
        };
        assert_eq!(proposal.taken[0].name, "Гальванизированный Прицел");
        assert_eq!(proposal.taken[0].rank, Some(10));
    }

    #[test]
    fn a_bracket_that_names_no_rank_stays_part_of_the_name() {
        assert_eq!(
            ranked("Кобра и Журавль Прайм (Комплект)"),
            ("Кобра и Журавль Прайм".to_owned(), None)
        );
    }

    #[test]
    fn each_way_an_exchange_can_end_is_told_apart() {
        assert_eq!(
            read(
                "1.0 Script [Info]: Dialog.lua: Dialog::CreateOk(description=Обмен успешно завершён!, title=)"
            ),
            vec![LogEvent::Settled(Outcome::Done)]
        );
        assert_eq!(
            read(
                "1.0 Script [Info]: Dialog.lua: Dialog::CreateOk(description=Обмен не удался., title=)"
            ),
            vec![LogEvent::Settled(Outcome::Failed)]
        );
        assert_eq!(
            read(
                "1.0 Script [Info]: Dialog.lua: Dialog::CreateOk(description=Обмен был отменён, title=)"
            ),
            vec![LogEvent::Settled(Outcome::Cancelled)]
        );
        assert_eq!(
            read("1.0 Script [Info]: OnTradeAccepted failed"),
            vec![LogEvent::Settled(Outcome::Failed)]
        );
    }

    #[test]
    fn a_private_tab_is_somebody_writing_to_us() {
        assert_eq!(
            read(
                "17566.496 Script [Info]: ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: FTennoAlpha to index 8"
            ),
            vec![LogEvent::Opened {
                player: "TennoAlpha".into()
            }]
        );
    }

    #[test]
    fn a_line_of_ours_going_out_is_told_apart() {
        assert_eq!(
            read(
                "1956.359 Net [Info]: IRC out: PRIVMSG TennoBeta :Hi! I want to buy: \"Thermal Transfer (rank 0)\" for 12 platinum. (warframe.market)"
            ),
            vec![LogEvent::Wrote {
                player: "TennoBeta".into()
            }]
        );
    }

    #[test]
    fn opening_a_chat_ourselves_says_both_things_in_turn() {
        assert_eq!(
            read("1956.358 Script [Info]: ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: FTennoBeta to index 6
1956.358 Script [Info]: ChatRedux.lua: Chat: Filters for FTennoBeta:
1956.359 Net [Info]: IRC out: PRIVMSG TennoBeta :Hi!"),
            vec![
                LogEvent::Opened { player: "TennoBeta".into() },
                LogEvent::Wrote { player: "TennoBeta".into() },
            ]
        );
    }

    #[test]
    fn a_tab_of_any_other_kind_is_not_a_whisper() {
        for line in [
            "1.0 Script [Info]: ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: G_RU to index 2",
            "1.0 Script [Info]: ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: H_ALTCETUSHUB4_RU_0 to index 1",
            "1.0 Script [Info]: ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: S6a90c07e268ca266e806a098 to index 1",
            "1.0 Script [Info]: ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: C64b3dd09a87ac0cf8f0b217e to index 1",
        ] {
            assert_eq!(read(line), vec![], "{line}");
        }
    }

    #[test]
    fn a_badge_the_game_draws_is_not_part_of_the_name() {
        assert_eq!(
            read(
                "1.0 Script [Info]: ChatRedux.lua: ChatRedux::AddTab: Adding tab with channel name: FSomeone\u{e000} to index 3"
            ),
            vec![LogEvent::Opened {
                player: "Someone".into()
            }]
        );
    }

    #[test]
    fn a_dialog_that_names_nobody_is_not_an_exchange() {
        let events = read(
            "1.0 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы хотите принять условия сделки? Вы предлагаете
Платина x 5
, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)",
        );

        assert_eq!(events, vec![]);
    }

    #[test]
    fn a_cycling_session_is_heard_from_opening_to_leaving() {
        let stream = "\
800.001 Sys [Info]: Created /Lotus/Interface/OmegaRerollSelection.swf
800.002 Script [Info]: OmegaRerollSelection.lua: Diorama setup
812.399 Sys [Info]: Created /Lotus/Interface/Dialog.swf
812.400 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы уверены, что хотите изменить характеристики мода Парные Токсоцисты Hexa-visitis за 7,000?, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)
813.100 Script [Info]: Dialog.lua: SendResult_MENU_SELECT()
813.100 Script [Info]: Dialog.lua: Dialog::SendResult(4)
830.100 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Создать выбранный мод Разлома?, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)
830.900 Script [Info]: Dialog.lua: Dialog::SendResult(4)
900.000 AI [Info]: NpcManager::ClearAgents() ReadyToCreateAgents = false";
        let mut reading = Reading::new();

        let events: Vec<LogEvent> = stream
            .lines()
            .filter_map(|line| reading.line(line))
            .collect();

        assert_eq!(
            events,
            vec![
                LogEvent::CycleScreen,
                LogEvent::Cycled(Cycle {
                    weapon: "Парные Токсоцисты".into(),
                    name: "Hexa-visitis".into(),
                    kuva: 7_000,
                }),
                LogEvent::CycleKept,
                LogEvent::CycleScreenGone,
            ]
        );
    }

    #[test]
    fn a_cancelled_cycle_is_not_heard() {
        let stream = "\
800.001 Sys [Info]: Created /Lotus/Interface/OmegaRerollSelection.swf
812.400 Script [Info]: Dialog.lua: Dialog::CreateOkCancel(description=Вы уверены, что хотите изменить характеристики мода Парные Токсоцисты Hexa-visitis за 3,500?, title= leftItem=/Menu/Confirm_Item_Ok, rightItem=/Menu/Confirm_Item_Cancel)
813.100 Script [Info]: Dialog.lua: Dialog::SendResult(1)";
        let mut reading = Reading::new();

        let events: Vec<LogEvent> = stream
            .lines()
            .filter_map(|line| reading.line(line))
            .collect();

        assert_eq!(events, vec![LogEvent::CycleScreen]);
    }

    #[test]
    fn a_session_the_game_wrote_reloads_after_a_keep_and_closes_on_quitting() {
        let stream = include_str!("../../tests/fixtures/ee-cycling.log");
        let mut reading = Reading::new();

        let events: Vec<LogEvent> = stream
            .lines()
            .filter_map(|line| reading.line(line))
            .collect();

        let hexa = || Cycle {
            weapon: "Парные Токсоцисты".into(),
            name: "Hexa-visitis".into(),
            kuva: 3_500,
        };
        assert_eq!(
            events,
            vec![
                LogEvent::CycleScreen,
                LogEvent::Cycled(hexa()),
                LogEvent::CycleKept,
                LogEvent::Cycled(hexa()),
                LogEvent::CycleKept,
                LogEvent::CycleScreenReloading,
                LogEvent::CycleScreen,
                LogEvent::CycleScreenGone,
            ]
        );
    }
}
