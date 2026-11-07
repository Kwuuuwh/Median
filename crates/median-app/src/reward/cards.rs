use median_vision::Area;

use crate::screen::placed;

/// Maximum number of cards on the reward screen, one per squad member.
pub(super) const CARDS: u32 = 4;

/// Card position grids of the reward screen, by card count. A row of two uses positions of the
/// row of four, a single card a position of the row of three.
pub(super) const GRIDS: [u32; 2] = [CARDS, CARDS - 1];

/// Left edge of the first card of a full squad and the distance to the next card, in 1920×1080
/// pixels.
const FIRST: f32 = 478.0;
const STEP: f32 = 243.0;

/// Horizontal centre of the card row, which stays fixed whatever the number of cards.
const CENTRE: f32 = FIRST + STEP * CARDS as f32 / 2.0;

/// Offset of the name plate from the card's left edge, and its width.
const PLATE: (f32, f32) = (4.0, 235.0);

/// Top and height of each line of the plate. A one-line name uses the second and leaves the
/// first empty.
const LINES: [(f32, f32); 2] = [(416.0, 24.0), (438.0, 25.0)];

/// Areas of the two name plate lines of card `card` on a `width`×`height` frame, with `cards`
/// cards on the screen: one per player, and a squad is not always full.
///
/// The game scales its interface uniformly and centres it, so a position measured at 1920×1080
/// maps to any frame by that scale, and a shorter row shifts right by half a step for each
/// missing card.
pub(super) fn plate(width: u32, height: u32, cards: u32, card: u32) -> [Area; 2] {
    let first = CENTRE - STEP * cards as f32 / 2.0;
    let x = first + STEP * card as f32 + PLATE.0;
    LINES.map(|(line, tall)| placed(width, height, [x, line, PLATE.1, tall]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stands_where_it_was_measured() {
        let [one, two] = plate(1920, 1080, CARDS, 0);

        assert_eq!(
            one,
            Area {
                x: 482,
                y: 416,
                width: 235,
                height: 24
            }
        );
        assert_eq!(
            two,
            Area {
                x: 482,
                y: 438,
                width: 235,
                height: 25
            }
        );
    }

    #[test]
    fn steps_one_card_at_a_time() {
        let [one, _] = plate(1920, 1080, CARDS, 0);
        let [four, _] = plate(1920, 1080, CARDS, 3);

        assert_eq!(four.x - one.x, 243 * 3);
    }

    /// A squad of two puts its cards where a full squad puts its second and third.
    #[test]
    fn a_shorter_row_stands_in_the_middle() {
        let [pair, _] = plate(1920, 1080, 2, 0);
        let [full, _] = plate(1920, 1080, CARDS, 1);

        assert_eq!(pair.x, full.x);
    }

    #[test]
    fn the_two_rows_stand_half_a_step_apart() {
        let [even, _] = plate(1920, 1080, GRIDS[0], 1);
        let [odd, _] = plate(1920, 1080, GRIDS[1], 1);
        let [pair, _] = plate(1920, 1080, 2, 0);
        let [lone, _] = plate(1920, 1080, 1, 0);

        assert_eq!(odd.x - even.x, 243 / 2 + 1);
        assert_eq!(pair.x, even.x);
        assert_eq!(lone.x, odd.x);
    }

    #[test]
    fn keeps_the_plate_inside_a_wider_screen() {
        let [one, two] = plate(2560, 1080, CARDS, 2);

        assert!(one.x + one.width <= 2560);
        assert!(two.y + two.height <= 1080);
        assert_eq!(one.height, 24);
    }
}
