mod cards;
mod naming;

use std::sync::Arc;

use median_core::{Instant, Recognition, seconds};

use median_vision::{Frame, Pixels, Reader, VisionError};
use thiserror::Error;
use tracing::{error, info};

use crate::catalog::Catalog;
use crate::market::Store;
use crate::screen::{self, ScreenError};
use crate::state_sink::SharedHoldings;

pub use naming::Found;

/// Seconds past the reward screen's countdown after which the overlay is hidden.
const GRACE: Instant = 3;

/// Seconds the overlay stays up when the game logs no countdown for the reward screen.
const AT_MOST: Instant = 20;

/// Loads the embedded card recognition model; `Recognition::Failed` when it does not load.
pub fn reader() -> Result<Arc<Reader>, Recognition> {
    Reader::new().map(Arc::new).map_err(|why| {
        error!(error = %why, "reward recognition model load failed, reward cards not read");
        Recognition::Failed
    })
}

/// Recognised text of one reward screen card.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// Item name as shown on the card, both lines joined.
    pub text: String,
    /// Lowest recognition confidence of the non-empty lines.
    pub confidence: f32,
}

/// Reason the reward cards could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RewardError {
    /// The card recognition model did not load.
    #[error("the model that reads the screen did not load")]
    NoModel,

    #[error(transparent)]
    Screen(#[from] ScreenError),

    #[error("the screen could not be read: {0}")]
    Unread(String),
}

impl From<VisionError> for RewardError {
    fn from(trouble: VisionError) -> Self {
        Self::Unread(trouble.to_string())
    }
}

/// State of the relic reward screen, from the moment the game opens it until it closes.
pub struct Screen {
    reader: Option<Arc<Reader>>,
    /// Item this account received, named directly in the game log.
    own: Option<String>,
    standing: bool,
    /// When the game opened the screen.
    since: Option<Instant>,
    /// Time at which the overlay is hidden, whatever the next game log lines say.
    until: Option<Instant>,
}

impl Screen {
    /// Reward screen state; cards are read with `reader` when it is present.
    pub fn new(reader: Option<Arc<Reader>>) -> Self {
        Self {
            reader,
            own: None,
            standing: false,
            since: None,
            until: None,
        }
    }

    pub fn opened(&mut self) {
        self.standing = true;
        self.since = Some(seconds());
        self.own = None;
        self.until = Some(seconds() + AT_MOST);
    }

    pub fn took(&mut self, item: String) {
        self.own = Some(item);
    }

    /// Item this account received; `None` until the game log names it.
    pub fn ours(&self) -> Option<&str> {
        self.own.as_deref()
    }

    /// Sets the overlay deadline to the countdown the game logs plus `GRACE`; ignored when the
    /// screen is not open or the countdown is zero.
    pub fn stands_for(&mut self, stands: u32) {
        if !self.standing || stands == 0 {
            return;
        }
        self.until = Some(seconds() + Instant::from(stands) + GRACE);
        info!(countdown_s = stands, "reward screen open");
    }

    /// Whether the reward screen's overlay deadline has passed.
    pub fn overstayed(&self) -> bool {
        self.until.is_some_and(|until| seconds() >= until)
    }

    pub fn gone(&mut self) {
        self.standing = false;
        self.since = None;
        self.own = None;
        self.until = None;
    }

    /// Whether there is a model to read the cards with.
    pub fn reads(&self) -> bool {
        self.reader.is_some()
    }

    /// Whether the market cache fetched the order book of every card with a market pair since
    /// the screen opened.
    pub fn fresh(&self, found: &[Found]) -> bool {
        let Some(since) = self.since else {
            return false;
        };
        found
            .iter()
            .filter(|card| card.pair.is_some())
            .all(|card| card.looked.is_some_and(|looked| looked >= since))
    }

    /// Whether the recognised cards include the reward named in the game log; `None` before it
    /// is named. That card's answer is certain, so it measures how well the others were read.
    pub fn agrees(&self, found: &[Found]) -> Option<bool> {
        let ours = self.own.as_deref()?;
        Some(found.iter().any(|card| card.item() == Some(ours)))
    }

    /// Reads every possible card position and matches the recognised names against the catalog
    /// to find which row is shown.
    pub async fn cards(
        &self,
        catalog: &Catalog,
        market_cache: &Store,
        holdings: &SharedHoldings,
        favourites: &std::collections::HashSet<String>,
        lang: &str,
    ) -> Result<Vec<Found>, RewardError> {
        let read = tokio::task::block_in_place(|| self.read())?;
        Ok(naming::found(
            catalog,
            market_cache,
            holdings,
            favourites,
            lang,
            self.ours(),
            &read,
        )
        .await)
    }

    /// The cards already read, priced again from the current market cache.
    pub async fn repriced(
        &self,
        catalog: &Catalog,
        market_cache: &Store,
        holdings: &SharedHoldings,
        favourites: &std::collections::HashSet<String>,
        lang: &str,
        cards: &[Found],
    ) -> Vec<Found> {
        naming::repriced(catalog, market_cache, holdings, favourites, lang, cards).await
    }

    /// Recognised text of both card position grids. `naming::standing` decides which grid
    /// holds the cards by matching against the dictionary.
    fn read(&self) -> Result<Vec<Vec<Card>>, RewardError> {
        let reader = self.reader.as_ref().ok_or(RewardError::NoModel)?;
        let shot = screen::shot()?;
        let frame = Frame::new(&shot.bytes, shot.width, shot.height, Pixels::Bgra)?;

        rows(reader, &frame, shot.width, shot.height)
    }
}

/// Recognised cards of every position grid, each read left to right.
fn rows(
    reader: &Reader,
    frame: &Frame<'_>,
    width: u32,
    height: u32,
) -> Result<Vec<Vec<Card>>, RewardError> {
    cards::GRIDS
        .iter()
        .map(|&standing| {
            (0..standing)
                .map(|card| {
                    let [first, second] = cards::plate(width, height, standing, card);
                    Ok(joined(
                        reader.read(frame, first)?,
                        reader.read(frame, second)?,
                    ))
                })
                .collect()
        })
        .collect()
}

/// The two lines of a plate as one name. A one-line name leaves the first line empty.
fn joined(first: median_vision::Reading, second: median_vision::Reading) -> Card {
    let scores: Vec<f32> = [
        (&first.text, first.confidence),
        (&second.text, second.confidence),
    ]
    .into_iter()
    .filter(|(text, _)| !text.is_empty())
    .map(|(_, score)| score)
    .collect();

    let text = match first.text.is_empty() {
        true => second.text,
        false => format!("{} {}", first.text, second.text),
    };

    Card {
        text: text.trim().to_owned(),
        confidence: scores.into_iter().fold(f32::MAX, f32::min),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::Pool;
    use median_vision::Reading;

    /// Top-left corner, in the capture, at which the fixture bands were cut.
    const CUT: (u32, u32) = (470, 400);

    /// Catalog names of the items on the fixture screens.
    const NAMES: [(&str, &str); 5] = [
        ("trumna", "Трумна Прайм (Чертеж)"),
        ("banshee-helmet", "Банши Прайм: Нейрооптика (Чертеж)"),
        ("nautilus-carapace", "Наутилус Прайм: Панцирь"),
        ("kestrel-blade", "Кестрел Прайм: Клинок"),
        ("braton-barrel", "Брэйтон Прайм: Ствол"),
    ];

    fn pool() -> Pool {
        Pool::new(
            NAMES
                .iter()
                .map(|(item, name)| ((*item).to_owned(), (*name).to_owned()))
                .collect(),
        )
    }

    /// Both position grids of a fixture cut from a real capture, as recognised by the model.
    fn read_rows(fixture: &str) -> Vec<Vec<Card>> {
        let reader = Reader::new().expect("the model loads");

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);
        let band = image::open(&path)
            .unwrap_or_else(|why| panic!("{}: {why}", path.display()))
            .to_rgba8();

        let (width, height) = (1920u32, 1080u32);
        let mut whole = vec![0u8; width as usize * height as usize * 4];
        for (x, y, pixel) in band.enumerate_pixels() {
            let at = ((CUT.1 + y) as usize * width as usize + (CUT.0 + x) as usize) * 4;
            whole[at..at + 4].copy_from_slice(&pixel.0);
        }

        let frame = Frame::new(&whole, width, height, Pixels::Rgba).expect("the frame");
        rows(&reader, &frame, width, height).expect("the rows")
    }

    /// Item keys of the grid that `naming::standing` chooses for the fixture.
    fn standing(fixture: &str) -> Vec<String> {
        let rows = read_rows(fixture);
        let (_, keys) = naming::standing(&pool(), &rows, None);
        keys.into_iter()
            .map(|key| key.unwrap_or_default())
            .collect()
    }

    #[test]
    fn three_cards_are_read_off_the_shorter_row() {
        assert_eq!(
            standing("three-cards.png"),
            ["trumna", "banshee-helmet", "nautilus-carapace"]
        );
    }

    #[test]
    fn two_cards_are_read_off_the_longer_row() {
        assert_eq!(
            standing("two-cards.png"),
            ["kestrel-blade", "braton-barrel"]
        );
    }

    fn read(text: &str, confidence: f32) -> Reading {
        Reading {
            text: text.to_owned(),
            confidence,
        }
    }

    #[test]
    fn a_one_line_name_stands_on_the_second_line_alone() {
        let card = joined(read("", 0.0), read("Брэйтон Прайм: Ствол", 0.96));

        assert_eq!(card.text, "Брэйтон Прайм: Ствол");
        assert!((card.confidence - 0.96).abs() < 1e-6);
    }

    #[test]
    fn a_two_line_name_joins_into_one() {
        let card = joined(read("Парные Зорены Прайм:", 0.94), read("Рукоять", 0.97));

        assert_eq!(card.text, "Парные Зорены Прайм: Рукоять");
        assert!((card.confidence - 0.94).abs() < 1e-6);
    }
}
