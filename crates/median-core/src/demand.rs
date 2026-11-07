/// How readily rivens of a weapon sell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demand {
    Lively,
    Slow,
    Dead,
    /// Too little data to classify.
    Unclear,
}

/// Minimum popularity for `Lively`, on the game's scale where the most-traded weapon is 100.
const LIVELY: f64 = 5.0;

/// `Lively` requires fewer than this many listed lots per point of popularity.
const LIVELY_GLUT: f64 = 50.0;

/// Listed lots per point of popularity at or above which a weapon is `Dead`.
const DEAD_GLUT: f64 = 100.0;

/// Classifies riven demand for a weapon from last week's trade popularity and the number of lots
/// listed now. `popularity` of `None` or zero means no trades last week (`Dead`); `lots` of `None`
/// means the listings were not counted, and the class then rests on popularity alone.
pub fn demand(popularity: Option<f64>, lots: Option<u32>) -> Demand {
    let Some(popularity) = popularity.filter(|popularity| *popularity > 0.0) else {
        return Demand::Dead;
    };
    match lots.map(|lots| f64::from(lots) / popularity) {
        Some(glut) if glut >= DEAD_GLUT => Demand::Dead,
        Some(glut) if popularity >= LIVELY && glut < LIVELY_GLUT => Demand::Lively,
        Some(_) => Demand::Slow,
        None if popularity >= LIVELY => Demand::Lively,
        None if popularity > 1.0 => Demand::Slow,
        None => Demand::Unclear,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_weapon_traded_often_and_listed_little_is_lively() {
        assert_eq!(demand(Some(100.0), Some(136)), Demand::Lively);
        assert_eq!(demand(Some(19.0), Some(500)), Demand::Lively);
    }

    #[test]
    fn a_weapon_listed_far_beyond_its_trades_is_dead() {
        assert_eq!(demand(Some(2.0), Some(513)), Demand::Dead);
        assert_eq!(demand(Some(1.0), Some(362)), Demand::Dead);
    }

    #[test]
    fn a_weapon_between_is_slow() {
        assert_eq!(demand(Some(8.0), Some(501)), Demand::Slow);
        assert_eq!(demand(Some(3.0), Some(37)), Demand::Slow);
    }

    #[test]
    fn a_weapon_not_traded_last_week_is_dead() {
        assert_eq!(demand(None, Some(3)), Demand::Dead);
        assert_eq!(demand(None, None), Demand::Dead);
    }

    #[test]
    fn without_lots_counted_only_popularity_speaks_and_the_least_is_unclear() {
        assert_eq!(demand(Some(40.0), None), Demand::Lively);
        assert_eq!(demand(Some(3.0), None), Demand::Slow);
        assert_eq!(demand(Some(1.0), None), Demand::Unclear);
    }
}
