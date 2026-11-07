use median_vision::{Area, Frame, lettered_rows};

use crate::screen::placed;

/// Name and stats area of the chosen card in the middle, as 1920×1080 `[x, y, width, height]`.
const CHOSEN: [f32; 4] = [820.0, 600.0, 288.0, 222.0];

/// Name and stats area of the other card, left or right of the chosen one.
const BESIDE: [[f32; 4]; 2] = [[482.0, 612.0, 240.0, 183.0], [1200.0, 612.0, 246.0, 183.0]];

/// Stat rows of the chosen card, checked for text to detect the card.
const CHOSEN_PROBE: [f32; 4] = [835.0, 700.0, 260.0, 115.0];

/// Stat rows of the other card on each side, checked for text to detect the card.
const BESIDE_PROBES: [[f32; 4]; 2] = [[500.0, 700.0, 200.0, 80.0], [1215.0, 700.0, 215.0, 80.0]];

/// Minimum share of a probe's pixel rows with text for a card to count as present.
const LETTERED: f32 = 0.3;

/// Position of the other card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The old riven, next to the chosen new one.
    Left,
    /// The new riven, next to the chosen old one.
    Right,
}

impl Side {
    fn at(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }
}

/// Cards visible on a frame of the cycling screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Whether the chosen card can be read; a dialog over the screen hides it.
    pub chosen: bool,
    /// Position of the other card, present after a cycle.
    pub beside: Option<Side>,
}

/// Detects which cards `frame` shows from the share of text rows in each probe area.
pub fn layout(frame: &Frame<'_>) -> Layout {
    let (width, height) = (frame.width(), frame.height());
    let lettered = |rect: [f32; 4]| {
        let area = placed(width, height, rect);
        lettered_rows(frame, area) as f32 >= area.height as f32 * LETTERED
    };
    let beside = [Side::Left, Side::Right]
        .into_iter()
        .find(|side| lettered(BESIDE_PROBES[side.at()]));
    Layout {
        chosen: lettered(CHOSEN_PROBE),
        beside,
    }
}

/// Name and stats of the chosen card on a frame of this size.
pub fn chosen(width: u32, height: u32) -> Area {
    placed(width, height, CHOSEN)
}

/// Name and stats of the other card on a frame of this size.
pub fn beside(width: u32, height: u32, side: Side) -> Area {
    placed(width, height, BESIDE[side.at()])
}

#[cfg(test)]
pub(super) mod tests {
    use std::path::Path;

    use median_vision::Pixels;

    use super::*;

    /// Top-left corner at which the fixture bands were cut from their captures.
    const CUT: (u32, u32) = (470, 590);

    /// A 1920×1080 frame with a band cut from a real capture pasted at `CUT`, black elsewhere.
    pub(in crate::cycling) fn frame_of(fixture: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);
        let band = image::open(&path)
            .unwrap_or_else(|why| panic!("{}: {why}", path.display()))
            .to_rgba8();
        let mut whole = vec![0u8; 1920 * 1080 * 4];
        for (x, y, pixel) in band.enumerate_pixels() {
            let at = ((CUT.1 + y) as usize * 1920 + (CUT.0 + x) as usize) * 4;
            whole[at..at + 4].copy_from_slice(&pixel.0);
        }
        whole
    }

    fn layout_of(fixture: &str) -> Layout {
        let bytes = frame_of(fixture);
        layout(&Frame::new(&bytes, 1920, 1080, Pixels::Rgba).expect("a frame"))
    }

    #[test]
    fn before_a_cycle_one_card_stands() {
        for fixture in ["cycle-one.png", "cycle-locked.png"] {
            assert_eq!(
                layout_of(fixture),
                Layout {
                    chosen: true,
                    beside: None
                },
                "{fixture}"
            );
        }
    }

    #[test]
    fn after_a_cycle_the_other_card_stands_on_its_side() {
        assert_eq!(layout_of("cycle-chose-new.png").beside, Some(Side::Left));
        assert_eq!(layout_of("cycle-chose-old.png").beside, Some(Side::Right));
    }

    #[test]
    fn a_dialog_over_the_screen_hides_the_cards() {
        assert_eq!(
            layout_of("cycle-lock-dialog.png"),
            Layout {
                chosen: false,
                beside: None
            }
        );
    }

    #[test]
    fn the_places_carry_to_a_wider_screen() {
        let area = chosen(2560, 1080);
        assert_eq!((area.x, area.y), (820 + 320, 600));
    }
}
