/// Unix seconds from an RFC 3339 timestamp such as `2026-08-16T17:00:00.000+00:00`; `None`
/// when the text is not one.
pub fn seconds(text: &str) -> Option<i64> {
    let field = |from: usize, to: usize| text.get(from..to)?.parse::<i64>().ok();

    if text.len() < 19 || text.as_bytes().get(10) != Some(&b'T') {
        return None;
    }
    let year = field(0, 4)?;
    let month = field(5, 7)?;
    let day = field(8, 10)?;
    let hour = field(11, 13)?;
    let minute = field(14, 16)?;
    let second = field(17, 19)?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let offset = offset(&text[19..])?;
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second - offset)
}

/// Offset of the timestamp's zone ahead of UTC, in seconds; `None` when the suffix is not a
/// valid offset.
fn offset(tail: &str) -> Option<i64> {
    let tail = tail.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    match tail.as_bytes().first() {
        None => Some(0),
        Some(b'Z' | b'z') => Some(0),
        Some(sign @ (b'+' | b'-')) => {
            let hours: i64 = tail.get(1..3)?.parse().ok()?;
            let minutes: i64 = tail.get(4..6)?.parse().ok()?;
            let seconds = hours * 3_600 + minutes * 60;
            Some(if *sign == b'-' { -seconds } else { seconds })
        }
        Some(_) => None,
    }
}

/// The UTC calendar date of a Unix time, as `YYYY-MM-DD`.
pub fn day(seconds: i64) -> String {
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

/// The calendar date of a day number counted from 1970-01-01; the inverse of `days_from_civil`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let months = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * months + 2) / 5 + 1;
    let month = if months < 10 { months + 3 } else { months - 9 };
    let year = year_of_era + era * 400;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Days between 1970-01-01 and the given civil date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moment_names_the_day_it_falls_on() {
        assert_eq!(day(0), "1970-01-01");
        assert_eq!(day(86_399), "1970-01-01");
        assert_eq!(day(86_400), "1970-01-02");
        assert_eq!(
            day(seconds("2026-08-23T00:00:00.000+00:00").expect("parses")),
            "2026-08-23"
        );
        assert_eq!(
            day(seconds("2026-08-23T23:59:59Z").expect("parses")),
            "2026-08-23"
        );
    }

    #[test]
    fn every_day_of_a_leap_year_round_trips() {
        for step in 0..800 {
            let at = seconds("2024-01-01T12:00:00Z").expect("parses") + step * 86_400;
            let named = day(at);
            let (year, month, of) = (
                named[0..4].parse::<i64>().expect("year"),
                named[5..7].parse::<i64>().expect("month"),
                named[8..10].parse::<i64>().expect("day"),
            );
            assert_eq!(days_from_civil(year, month, of), at.div_euclid(86_400));
        }
    }

    #[test]
    fn the_epoch_is_zero() {
        assert_eq!(seconds("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn known_moments_land_where_they_should() {
        assert_eq!(seconds("2000-01-01T00:00:00Z"), Some(946_684_800));
        assert_eq!(seconds("2024-02-29T00:00:00Z"), Some(1_709_164_800));
    }

    #[test]
    fn the_shapes_the_market_sends_are_both_read() {
        // Buckets have milliseconds and an explicit `+00:00` offset, orders a bare `Z`.
        assert_eq!(
            seconds("2026-08-16T17:00:00.000+00:00"),
            seconds("2026-08-16T17:00:00Z")
        );
    }

    #[test]
    fn a_zone_moves_the_moment() {
        assert_eq!(
            seconds("2026-08-16T18:00:00+02:00"),
            seconds("2026-08-16T16:00:00Z")
        );
        assert_eq!(
            seconds("2026-08-16T14:00:00-02:00"),
            seconds("2026-08-16T16:00:00Z")
        );
    }

    #[test]
    fn an_hour_apart_is_an_hour_apart() {
        let earlier = seconds("2026-08-16T17:00:00.000+00:00").expect("parses");
        let later = seconds("2026-08-16T18:00:00.000+00:00").expect("parses");
        assert_eq!(later - earlier, 3_600);
    }

    #[test]
    fn nonsense_is_refused_rather_than_guessed() {
        assert_eq!(seconds(""), None);
        assert_eq!(seconds("2026-08-16"), None);
        assert_eq!(seconds("2026-13-01T00:00:00Z"), None);
        assert_eq!(seconds("2026-08-16 17:00:00Z"), None);
        assert_eq!(seconds("2026-08-16T17:00:00 somewhere"), None);
    }
}
