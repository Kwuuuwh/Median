use crate::frame::{Area, Frame};

/// Highest channel value of a dark background pixel, out of 255.
const GROUND: u8 = 76;

/// Range of the highest channel value of a pale text pixel, out of 255.
const LETTERS: (u8, u8) = (102, 217);

/// Largest gap between the highest and lowest channel of a grey text pixel, out of 255.
const GREY: u8 = 64;

/// Minimum number of text pixels for a row to count as text.
const LEAST_LETTERS: u32 = 5;

/// Number of rows in `area` with pale text on a mostly dark background; zero when `area` is not
/// inside the frame.
pub fn lettered_rows(frame: &Frame<'_>, area: Area) -> u32 {
    if !frame.holds(area) {
        return 0;
    }
    (area.y..area.y + area.height)
        .filter(|&y| {
            let (mut letters, mut ground) = (0, 0);
            for x in area.x..area.x + area.width {
                let [red, green, blue] = frame.rgb(x, y);
                let (most, least) = (red.max(green).max(blue), red.min(green).min(blue));
                if most <= GROUND {
                    ground += 1;
                } else if (LETTERS.0..=LETTERS.1).contains(&most) && most - least <= GREY {
                    letters += 1;
                }
            }
            letters >= LEAST_LETTERS && ground * 2 > area.width
        })
        .count() as u32
}

/// Maximum blank rows between two runs of text rows that still belong to one line.
const JOINED_GAP: u32 = 4;

/// Rows of padding added above and below a line.
const PAD: u32 = 3;

/// Areas of the pale text lines in `area`, top to bottom, for lines `pitch` rows tall; empty
/// when `area` is not inside the frame or `pitch` is zero.
pub fn lettered_lines(frame: &Frame<'_>, area: Area, pitch: u32) -> Vec<Area> {
    if !frame.holds(area) || pitch == 0 {
        return Vec::new();
    }
    let inked = |y: u32| {
        (area.x..area.x + area.width)
            .filter(|&x| {
                let [red, green, blue] = frame.rgb(x, y);
                let (most, least) = (red.max(green).max(blue), red.min(green).min(blue));
                most >= LETTERS.0 && most - least <= GREY
            })
            .count() as u32
            >= LEAST_LETTERS
    };
    let mut runs: Vec<(u32, u32)> = Vec::new();
    let mut start = None;
    for y in area.y..=area.y + area.height {
        match (y < area.y + area.height && inked(y), start) {
            (true, None) => start = Some(y),
            (false, Some(from)) => {
                match runs.last_mut() {
                    Some(last) if from - last.1 <= JOINED_GAP && y - last.0 <= pitch * 5 / 4 => {
                        last.1 = y;
                    }
                    _ => runs.push((from, y)),
                }
                start = None;
            }
            _ => {}
        }
    }
    runs.into_iter()
        .filter(|(from, to)| (to - from) * 20 >= pitch * 7)
        .flat_map(|(from, to)| {
            let tall = to - from;
            let parts = ((tall + pitch / 2) / pitch).max(1);
            (0..parts).map(move |part| {
                let top = (from + tall * part / parts).saturating_sub(PAD).max(area.y);
                let bottom = (from + tall * (part + 1) / parts + PAD).min(area.y + area.height);
                Area {
                    x: area.x,
                    y: top,
                    width: area.width,
                    height: bottom - top,
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Pixels;

    /// A dark frame with pale strokes on every other row of its top half.
    fn lettered(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; (width * height * 4) as usize];
        for y in (0..height / 2).step_by(2) {
            for x in 10..30 {
                let at = ((y * width + x) * 4) as usize;
                bytes[at..at + 4].copy_from_slice(&[180, 170, 175, 255]);
            }
        }
        bytes
    }

    #[test]
    fn pale_strokes_on_dark_ground_count_row_by_row() {
        let bytes = lettered(64, 40);
        let frame = Frame::new(&bytes, 64, 40, Pixels::Rgba).unwrap();
        let whole = Area {
            x: 0,
            y: 0,
            width: 64,
            height: 40,
        };

        assert_eq!(lettered_rows(&frame, whole), 10);
    }

    #[test]
    fn bright_ground_is_not_lettering() {
        let bytes = vec![200u8; 64 * 40 * 4];
        let frame = Frame::new(&bytes, 64, 40, Pixels::Rgba).unwrap();
        let whole = Area {
            x: 0,
            y: 0,
            width: 64,
            height: 40,
        };

        assert_eq!(lettered_rows(&frame, whole), 0);
    }

    #[test]
    fn strokes_a_pitch_apart_read_as_lines() {
        let (width, height) = (64u32, 100u32);
        let mut bytes = vec![0u8; (width * height * 4) as usize];
        for top in [10u32, 40, 70] {
            for y in top..top + 18 {
                for x in 10..40 {
                    let at = ((y * width + x) * 4) as usize;
                    bytes[at..at + 4].copy_from_slice(&[230, 220, 235, 255]);
                }
            }
        }
        let frame = Frame::new(&bytes, width, height, Pixels::Rgba).unwrap();
        let whole = Area {
            x: 0,
            y: 0,
            width,
            height,
        };

        let lines = lettered_lines(&frame, whole, 24);

        assert_eq!(
            lines.iter().map(|line| line.y).collect::<Vec<_>>(),
            vec![7, 37, 67]
        );
    }
}
