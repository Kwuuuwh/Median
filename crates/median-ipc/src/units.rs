/// Converts a share between 0 and 1 to tenths of a percent, rounded: `0.1234` gives `123`.
pub fn permille(share: f64) -> u32 {
    (share * 1000.0).round() as u32
}

/// Converts a value to tenths, rounded: `2.74` gives `27`.
pub fn tenths(value: f64) -> u32 {
    (value * 10.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_share_rounds_to_the_nearest_tenth_of_a_percent() {
        assert_eq!(permille(0.1234), 123);
        assert_eq!(permille(0.12351), 124);
    }

    #[test]
    fn a_negative_or_unknown_figure_becomes_zero() {
        assert_eq!(tenths(-3.0), 0);
        assert_eq!(tenths(f64::NAN), 0);
    }
}
