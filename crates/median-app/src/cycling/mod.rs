mod cards;
mod text;

use std::collections::HashMap;
use std::sync::Arc;

use median_core::{CycleCard, CycleRoll, CycleSide, Cycling, Instant, Unread};
use median_ipc::{Detail, Riven};
use median_vision::{Frame, Pixels, Reader, lettered_lines, print, print_distance};
use tracing::info;

use crate::catalog::Catalog;
use crate::collection::goods_of;
use crate::dictionary::{apart, letters};
use crate::rivens::{self, CardRiven, CardText, card_riven};
use crate::screen;
use crate::state_sink::{SharedHoldings, to_holdings};

pub use cards::Layout;

/// Maximum time in seconds the cycling screen counts as open without a close line in the game
/// log.
const AT_MOST: Instant = 30 * 60;

/// Minimum fingerprint distance, in cells, at which a card counts as showing new text.
const MOVED: u32 = 6;

/// Seconds to wait for the screen to reload after the player keeps a roll.
const RELOAD_WAIT: Instant = 12;

/// Layout and card fingerprints of one frame of the cycling screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Seen {
    layout: Layout,
    chosen: Option<u64>,
    beside: Option<u64>,
}

impl Seen {
    fn of(frame: &Frame<'_>) -> Self {
        let (width, height) = (frame.width(), frame.height());
        let layout = cards::layout(frame);
        Self {
            layout,
            chosen: layout
                .chosen
                .then(|| print(frame, cards::chosen(width, height)))
                .flatten(),
            beside: layout
                .beside
                .and_then(|side| print(frame, cards::beside(width, height, side))),
        }
    }

    /// Whether the layout or a card fingerprint differs from `before`.
    fn moved_from(&self, before: &Seen) -> bool {
        let apart = |one: Option<u64>, other: Option<u64>| match (one, other) {
            (Some(one), Some(other)) => print_distance(one, other) >= MOVED,
            (one, other) => one.is_some() != other.is_some(),
        };
        self.layout != before.layout
            || apart(self.chosen, before.chosen)
            || apart(self.beside, before.beside)
    }
}

/// Line height in pixel rows on the chosen card, and on the smaller card next to it.
const PITCH: (u32, u32) = (24, 20);

/// Layout and card texts read from a new frame of the cycling screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub layout: Layout,
    /// Text of the chosen card; `None` when it could not be read.
    pub chosen: Option<CardText>,
    /// Text of the other card; `None` when it is absent or could not be read.
    pub beside: Option<CardText>,
}

/// State of the riven cycling screen while it is open, read frame by frame.
pub struct Screen {
    reader: Option<Arc<Reader>>,
    since: Option<Instant>,
    last: Option<Seen>,
    /// Inventory riven on the screen and its rank, once identified.
    held: Option<(Riven, u32)>,
    /// Name of the last cycled riven, as written in the game log.
    cycled: Option<String>,
    /// Content of the overlay over the screen.
    shown: Cycling,
    /// A frame that differs from the last one read, kept until the next frame shows it is stable.
    settling: Option<Seen>,
    /// Card texts already read, keyed by (is the chosen card, fingerprint).
    texts: HashMap<(bool, u64), CardText>,
    /// Deadline for the screen to reload after a keep.
    reload_by: Option<Instant>,
}

impl Screen {
    /// Creates the screen state; without a `reader` no card text is read.
    pub fn new(reader: Option<Arc<Reader>>) -> Self {
        Self {
            reader,
            since: None,
            last: None,
            held: None,
            cycled: None,
            shown: Cycling::Gone,
            settling: None,
            texts: HashMap::new(),
            reload_by: None,
        }
    }

    /// Content of the overlay over the screen.
    pub fn shown(&self) -> Cycling {
        self.shown.clone()
    }

    /// Records that the screen opened at `at`; when it reopens after a keep, the overlay keeps
    /// its rolls and cards.
    pub fn opened(&mut self, at: Instant) {
        let rebuilt = self.since.is_some();
        self.since = Some(at);
        self.last = None;
        self.settling = None;
        self.reload_by = None;
        match (&mut self.shown, rebuilt) {
            (Cycling::Open { reading, .. }, true) => *reading = true,
            _ => {
                self.held = None;
                self.cycled = None;
                self.texts.clear();
                self.shown = Cycling::Open {
                    cards: Vec::new(),
                    rolls: Vec::new(),
                    reading: true,
                    unread: None,
                };
            }
        }
    }

    /// Records that the screen closed and clears all state.
    pub fn gone(&mut self) {
        self.since = None;
        self.last = None;
        self.settling = None;
        self.reload_by = None;
        self.held = None;
        self.cycled = None;
        self.texts.clear();
        self.shown = Cycling::Gone;
    }

    /// Records that the game started reloading the screen at `at`.
    pub fn reloading(&mut self, at: Instant) {
        self.reload_by = Some(at + RELOAD_WAIT);
    }

    /// Whether the reloading screen has missed its `RELOAD_WAIT` deadline at `now`.
    pub fn reload_overdue(&self, now: Instant) -> bool {
        self.reload_by.is_some_and(|by| now >= by)
    }

    /// Records a cycle of the riven `name` for `kuva` and adds an unnamed roll to the overlay.
    pub fn cycled(&mut self, name: String, kuva: u32) {
        self.cycled = Some(name);
        if let Cycling::Open { rolls, reading, .. } = &mut self.shown {
            rolls.insert(0, CycleRoll { kuva, name: None });
            *reading = true;
        }
    }

    /// Whether the screen is open and has been open no longer than `AT_MOST` at `now`.
    pub fn is_open(&self, now: Instant) -> bool {
        self.since.is_some_and(|since| now - since <= AT_MOST)
    }

    /// Captures a frame; returns its card texts once the cards changed since the last read and
    /// the frame is stable, otherwise `None`.
    pub fn look(&mut self) -> Result<Option<Look>, String> {
        let shot = screen::shot().map_err(|e| e.to_string())?;
        let frame = Frame::new(&shot.bytes, shot.width, shot.height, Pixels::Bgra)
            .map_err(|e| e.to_string())?;
        let seen = Seen::of(&frame);
        let Some(layout) = self.saw(seen) else {
            return Ok(None);
        };
        let (width, height) = (shot.width, shot.height);
        let chosen = match (layout.chosen, seen.chosen) {
            (true, Some(print)) => {
                self.text(&frame, cards::chosen(width, height), PITCH.0, (true, print))
            }
            _ => None,
        };
        let beside = match (layout.beside, seen.beside) {
            (Some(side), Some(print)) => self.text(
                &frame,
                cards::beside(width, height, side),
                PITCH.1,
                (false, print),
            ),
            _ => None,
        };
        let look = Look {
            layout,
            chosen,
            beside,
        };
        if let Cycling::Open { reading, .. } = &mut self.shown
            && look.chosen.is_some()
        {
            *reading = true;
        }
        Ok(Some(look))
    }

    /// Recognises the rivens in `look` and shows them on the overlay.
    pub async fn rivens(
        &mut self,
        catalog: &Catalog,
        holdings: &SharedHoldings,
        lang: &str,
        look: &Look,
    ) -> Result<Vec<CardRiven>, String> {
        let read = self.read_rivens(catalog, holdings, lang, look).await;
        if let Cycling::Open {
            cards,
            rolls,
            reading,
            unread,
        } = &mut self.shown
        {
            *reading = false;
            if let Ok(outcome) = &read {
                *unread = outcome.as_ref().err().copied();
            }
            if let Ok(Ok(read)) = &read
                && !read.is_empty()
            {
                let rank = self.held.as_ref().map_or(0, |(_, rank)| *rank);
                *cards = read
                    .iter()
                    .map(|(side, card)| CycleCard {
                        side: *side,
                        riven: card.riven.clone(),
                        rank: u8::try_from(rank).unwrap_or(u8::MAX),
                        name: card.name.clone(),
                        fresh: self
                            .cycled
                            .as_ref()
                            .is_some_and(|cycled| *cycled != card.name),
                    })
                    .collect();
                let fresh = read
                    .iter()
                    .find(|(_, card)| Some(&card.name) != self.cycled.as_ref())
                    .map(|(_, card)| card.name.clone());
                if let Some(roll) = rolls.first_mut()
                    && roll.name.is_none()
                    && cards.len() > 1
                {
                    roll.name = fresh;
                }
            }
        }
        read.map(|outcome| {
            outcome
                .map(|read| read.into_iter().map(|(_, card)| card).collect())
                .unwrap_or_default()
        })
    }

    async fn read_rivens(
        &mut self,
        catalog: &Catalog,
        holdings: &SharedHoldings,
        lang: &str,
        look: &Look,
    ) -> Result<Result<Vec<(CycleSide, CardRiven)>, Unread>, String> {
        if self.reader.is_none() {
            return Ok(Err(Unread::NoReader));
        }
        let Some(chosen) = &look.chosen else {
            return Ok(Ok(Vec::new()));
        };
        if self.held.is_none() {
            self.held = on_screen(catalog, holdings, chosen, self.cycled.as_deref()).await?;
        }
        let Some((held, rank)) = &self.held else {
            return Ok(Err(Unread::NotHeld));
        };
        let beside = look.layout.beside.map(|side| match side {
            cards::Side::Left => CycleSide::Left,
            cards::Side::Right => CycleSide::Right,
        });
        let texts = [
            (Some(CycleSide::Chosen), Some(chosen)),
            (beside, look.beside.as_ref()),
        ];
        let mut read = Vec::new();
        let mut unread = None;
        for (side, text) in texts {
            let (Some(side), Some(text)) = (side, text) else {
                continue;
            };
            match card_riven(catalog, lang, held, *rank, text).await? {
                Ok(riven) => read.push((side, riven)),
                Err(why) => unread = unread.or(Some(why)),
            }
        }
        Ok(match (read.is_empty(), unread) {
            (true, Some(why)) => Err(why),
            _ => Ok(read),
        })
    }

    fn read(&self, frame: &Frame<'_>, area: median_vision::Area, pitch: u32) -> Option<CardText> {
        let reader = self.reader.as_ref()?;
        let lines: Vec<String> = lettered_lines(frame, area, pitch)
            .into_iter()
            .filter_map(|line| reader.read(frame, line).ok())
            .map(|reading| reading.text)
            .collect();
        Some(text::card_text(&lines))
    }

    /// Layout of `seen` when it differs from the last frame read and matches the frame before
    /// it (stable); otherwise `None`.
    fn saw(&mut self, seen: Seen) -> Option<Layout> {
        if self.last.is_some_and(|last| !seen.moved_from(&last)) {
            self.settling = None;
            return None;
        }
        if self.settling.is_none_or(|before| seen.moved_from(&before)) {
            self.settling = Some(seen);
            return None;
        }
        self.settling = None;
        self.last = Some(seen);
        Some(seen.layout)
    }

    /// Text of a card, read once per fingerprint and cached.
    fn text(
        &mut self,
        frame: &Frame<'_>,
        area: median_vision::Area,
        pitch: u32,
        key: (bool, u64),
    ) -> Option<CardText> {
        if let Some(text) = self.texts.get(&key) {
            return Some(text.clone());
        }
        let text = self.read(frame, area, pitch)?;
        self.texts.insert(key, text.clone());
        Some(text)
    }
}

/// Inventory riven shown on a card: among the owned revealed rivens whose weapon name is on the
/// card, the one the game log names as cycled, or else the one whose name is uniquely closest.
async fn on_screen(
    catalog: &Catalog,
    holdings: &SharedHoldings,
    card: &CardText,
    cycled: Option<&str>,
) -> Result<Option<(Riven, u32)>, String> {
    let held: Vec<(Riven, u32)> = to_holdings(&holdings.read())
        .pieces
        .into_iter()
        .filter_map(|piece| match piece.detail {
            Some(Detail::Riven {
                rank,
                revealed: Some(riven),
            }) => Some((riven, rank)),
            _ => None,
        })
        .collect();
    let weapons: Vec<String> = held.iter().map(|(riven, _)| riven.weapon.clone()).collect();
    let mut called = HashMap::<String, Vec<String>>::new();
    for lang in WEAPON_LANGS {
        for (weapon, good) in goods_of(catalog, lang, weapons.clone()).await? {
            called.entry(weapon).or_default().push(good.name);
        }
    }
    let held: Vec<(Riven, u32)> = held
        .into_iter()
        .filter(|(riven, _)| {
            called
                .get(&riven.weapon)
                .is_some_and(|names| names.iter().any(|name| bears(&card.name, name)))
        })
        .collect();
    let names = rivens::names(
        catalog,
        held.iter().map(|(riven, _)| riven.clone()).collect(),
    )
    .await?;
    let read = card.name.rsplit(' ').next().unwrap_or_default();
    let mut ranked: Vec<(usize, usize)> = names
        .iter()
        .enumerate()
        .filter_map(|(at, name)| {
            let name = name.as_deref()?;
            match cycled {
                Some(cycled) => name.eq_ignore_ascii_case(cycled).then_some((0, at)),
                None => Some((apart(name, read), at)),
            }
        })
        .collect();
    ranked.sort_unstable();
    let found = match ranked.as_slice() {
        [(best, at), (next, _), ..] if best < next => held.get(*at).cloned(),
        [(_, at)] => held.get(*at).cloned(),
        _ => None,
    };
    if found.is_none() {
        let named: Vec<&str> = names.iter().flatten().map(String::as_str).collect();
        info!(
            card = card.name,
            cycled,
            held_rivens = held.len(),
            ?named,
            "cycled riven matches no held riven"
        );
    }
    Ok(found)
}

/// Languages to match a weapon's name on a card in: the game's language may differ from the
/// window's.
const WEAPON_LANGS: [&str; 2] = ["ru", "en"];

/// Misread letters allowed in a weapon's name on a card: one per this many letters.
const WEAPON_SLACK: usize = 5;

/// Whether a card's name contains the weapon's name, allowing one misread letter per
/// `WEAPON_SLACK` letters.
fn bears(card: &str, weapon: &str) -> bool {
    let card: Vec<char> = letters(card).chars().collect();
    let weapon = letters(weapon);
    let length = weapon.chars().count();
    if length == 0 || card.len() < length {
        return false;
    }
    let slack = length / WEAPON_SLACK;
    card.windows(length)
        .any(|window| apart(&window.iter().collect::<String>(), &weapon) <= slack)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(fixture: &str) -> Seen {
        let bytes = cards::tests::frame_of(fixture);
        Seen::of(&Frame::new(&bytes, 1920, 1080, Pixels::Rgba).expect("a frame"))
    }

    #[test]
    fn a_frame_is_read_once_it_stands_still_and_again_only_when_the_cards_change() {
        let mut screen = Screen::new(None);
        screen.opened(100);

        assert!(
            screen.saw(seen("cycle-one.png")).is_none(),
            "still settling"
        );
        assert!(screen.saw(seen("cycle-one.png")).is_some());
        assert!(screen.saw(seen("cycle-one.png")).is_none());
        assert!(
            screen.saw(seen("cycle-chose-new.png")).is_none(),
            "still settling"
        );
        assert!(screen.saw(seen("cycle-chose-new.png")).is_some());
    }

    #[test]
    fn a_card_bears_its_weapons_name_through_misread_letters_and_no_other() {
        assert!(bears("3енит Hexa-ampinok", "Зенит"));
        assert!(bears("oHerenine 3eнитHexa-aтpinok", "Зенит"));
        assert!(bears("Zenith Sci-lexitis", "Zenith"));
        assert!(!bears("Зенит Manti-heratis", "Джавлок"));
        assert!(!bears("Зенит Manti-heratis", "Катаболист"));
    }

    #[test]
    fn the_screen_is_let_go_once_it_outlives_its_watch() {
        let mut screen = Screen::new(None);
        screen.opened(100);

        assert!(screen.is_open(100 + AT_MOST));
        assert!(!screen.is_open(101 + AT_MOST));
    }

    #[test]
    fn both_cards_read_their_numbers_off_a_screenshot() {
        let screen = Screen::new(Some(Arc::new(Reader::new().expect("it loads"))));
        let frame_bytes = cards::tests::frame_of("cycle-chose-new.png");
        let frame = Frame::new(&frame_bytes, 1920, 1080, Pixels::Rgba).expect("a frame");
        let numbers = |text: Option<CardText>| -> Vec<f64> {
            text.expect("read")
                .stats
                .iter()
                .map(|line| line.number)
                .collect()
        };

        let chosen = screen.read(&frame, cards::chosen(1920, 1080), PITCH.0);
        let beside = screen.read(
            &frame,
            cards::beside(1920, 1080, cards::Side::Left),
            PITCH.1,
        );

        assert_eq!(numbers(chosen), vec![162.4, 121.4, 102.9, 86.0]);
        assert_eq!(numbers(beside), vec![274.4, 121.2, 105.5, 84.4]);
    }

    #[test]
    fn a_screen_built_again_after_a_keep_keeps_its_rolls() {
        let mut screen = Screen::new(None);
        screen.opened(100);
        screen.cycled("Hexa-visitis".into(), 3_500);
        screen.reloading(130);

        screen.opened(135);

        let Cycling::Open { rolls, reading, .. } = screen.shown() else {
            panic!("the layer stands");
        };
        assert_eq!(rolls.len(), 1);
        assert!(reading);
        assert!(!screen.reload_overdue(200));
    }

    #[test]
    fn a_screen_that_does_not_come_back_is_let_go() {
        let mut screen = Screen::new(None);
        screen.opened(100);
        screen.reloading(130);

        assert!(!screen.reload_overdue(130 + RELOAD_WAIT - 1));
        assert!(screen.reload_overdue(130 + RELOAD_WAIT));
    }
}
