use std::path::Path;

use median_vision::{Area, Frame, Pixels, Reader};

/// Rows the first line of a name plate stands in.
const FIRST: (u32, u32) = (0, 24);

/// Rows the second line stands in; a one-line name uses this one and leaves the first empty.
const SECOND: (u32, u32) = (22, 25);

/// Lowest score a line the model actually read comes back with.
const READ: f32 = 0.9;

/// The two lines of one name plate, first then second.
fn plate(reader: &Reader, fixture: &str) -> [(String, f32); 2] {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let picture = image::open(&path)
        .unwrap_or_else(|trouble| panic!("{}: {trouble}", path.display()))
        .to_rgba8();
    let (width, height) = picture.dimensions();
    let frame = Frame::new(picture.as_raw(), width, height, Pixels::Rgba).expect("the frame");

    [FIRST, SECOND].map(|(y, rows)| {
        let reading = reader
            .read(
                &frame,
                Area {
                    x: 0,
                    y,
                    width,
                    height: rows,
                },
            )
            .expect("a reading");
        (reading.text, reading.confidence)
    })
}

#[test]
fn reads_every_name_on_the_reward_screen() {
    let reader = Reader::new().expect("the model loads");

    let expected = [
        ("slot0.png", "", "Брэйтон Прайм: Ствол"),
        ("slot1.png", "Парные Зорены Прайм:", "Рукоять"),
        ("slot2.png", "Парные Зорены Прайм:", "Рукоять"),
        ("slot3.png", "", "Чертёж: Дайкю Прайм"),
    ];

    for (fixture, first, second) in expected {
        let [one, two] = plate(&reader, fixture);
        assert_eq!(one.0, first, "first line of {fixture}");
        assert_eq!(two.0, second, "second line of {fixture}");
        assert!(two.1 >= READ, "{fixture} read at {}", two.1);
    }
}

#[test]
fn an_empty_line_reads_empty() {
    let reader = Reader::new().expect("the model loads");

    let [one, _] = plate(&reader, "slot0.png");

    assert_eq!(one.0, "");
    assert_eq!(one.1, 0.0);
}
