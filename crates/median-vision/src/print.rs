use crate::frame::{Area, Frame};

/// Cells per side of a print's grid.
const CELLS: u32 = 8;

/// An 8×8 brightness fingerprint of an area: bit `row * 8 + column` is set when that cell is
/// brighter than the mean of all cells; `None` when the area is not inside the frame or is
/// smaller than 8×8 pixels.
pub fn print(frame: &Frame<'_>, area: Area) -> Option<u64> {
    if !frame.holds(area) || area.width < CELLS || area.height < CELLS {
        return None;
    }
    let cell = |row: u32, column: u32| {
        let (left, right) = (
            area.x + area.width * column / CELLS,
            area.x + area.width * (column + 1) / CELLS,
        );
        let (top, bottom) = (
            area.y + area.height * row / CELLS,
            area.y + area.height * (row + 1) / CELLS,
        );
        let mut sum = 0u64;
        for y in top..bottom {
            for x in left..right {
                let [red, green, blue] = frame.rgb(x, y);
                sum += u64::from(red) + u64::from(green) + u64::from(blue);
            }
        }
        sum / u64::from((right - left) * (bottom - top))
    };
    let shades: Vec<u64> = (0..CELLS * CELLS)
        .map(|at| cell(at / CELLS, at % CELLS))
        .collect();
    let mean = shades.iter().sum::<u64>() / shades.len() as u64;
    Some(
        shades
            .iter()
            .enumerate()
            .filter(|(_, shade)| **shade > mean)
            .fold(0u64, |print, (at, _)| print | 1 << at),
    )
}

/// How many cells two prints differ in.
pub fn print_distance(one: u64, other: u64) -> u32 {
    (one ^ other).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Pixels;

    fn whole() -> Area {
        Area {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        }
    }

    #[test]
    fn a_bright_left_half_sets_the_left_cells() {
        let mut bytes = vec![0u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..16 {
                let at = (y * 32 + x) * 4;
                bytes[at..at + 3].copy_from_slice(&[200, 200, 200]);
            }
        }
        let frame = Frame::new(&bytes, 32, 32, Pixels::Rgba).unwrap();

        let print = print(&frame, whole()).unwrap();

        assert_eq!(print.count_ones(), 32);
        assert_eq!(print & 0b1111, 0b1111);
    }

    #[test]
    fn prints_apart_by_the_cells_they_differ_in() {
        assert_eq!(print_distance(0b1011, 0b0001), 2);
    }
}
