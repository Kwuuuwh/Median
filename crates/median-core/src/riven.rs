/// Mod polarity of a riven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    Madurai,
    Vazarin,
    Naramon,
}

impl Polarity {
    /// Polarity for a game polarity code such as `AP_ATTACK`; `None` for any other code.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "AP_ATTACK" => Some(Self::Madurai),
            "AP_DEFENSE" => Some(Self::Vazarin),
            "AP_TACTIC" => Some(Self::Naramon),
            _ => None,
        }
    }
}

/// Weapon class a riven's stats are rolled for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RivenClass {
    Pistol,
    Rifle,
    Shotgun,
    Melee,
    Archgun,
    Kitgun,
    Zaw,
}

impl RivenClass {
    /// Class for a catalog class name such as `rifle`; `None` for an unknown name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "pistol" => Some(Self::Pistol),
            "rifle" => Some(Self::Rifle),
            "shotgun" => Some(Self::Shotgun),
            "melee" => Some(Self::Melee),
            "archgun" => Some(Self::Archgun),
            "kitgun" => Some(Self::Kitgun),
            "zaw" => Some(Self::Zaw),
            _ => None,
        }
    }

    /// Kind under which the game's weekly trade report lists veiled rivens of this class, e.g.
    /// `Rifle Riven Mod`.
    pub fn trade_kind(self) -> &'static str {
        match self {
            Self::Pistol => "Pistol Riven Mod",
            Self::Rifle => "Rifle Riven Mod",
            Self::Shotgun => "Shotgun Riven Mod",
            Self::Melee => "Melee Riven Mod",
            Self::Archgun => "Archgun Riven Mod",
            Self::Kitgun => "Kitgun Riven Mod",
            Self::Zaw => "Zaw Riven Mod",
        }
    }
}

/// Display form of a stat value in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatForm {
    /// `+274.4%`.
    Percent,
    /// `x1.58`.
    Multiplier,
    /// `+2.7`.
    Plain,
}

impl StatForm {
    /// Form for a catalog form name such as `percent`; `None` for an unknown name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "percent" => Some(Self::Percent),
            "multiplier" => Some(Self::Multiplier),
            "plain" => Some(Self::Plain),
            _ => None,
        }
    }

    /// Fraction for a value displayed in this form: `274.4` → `2.744`.
    pub fn amount(self, shown: f64) -> f64 {
        match self {
            Self::Percent => shown / 100.0,
            Self::Multiplier => shown - 1.0,
            Self::Plain => shown,
        }
    }

    /// Maximum change of the fraction caused by rounding the displayed value.
    pub fn rounding(self) -> f64 {
        match self {
            Self::Percent => 0.0005,
            Self::Multiplier => 0.005,
            Self::Plain => 0.05,
        }
    }

    /// Displayed value of a fraction in this form, rounded as the game rounds it: `2.744` →
    /// `274.4`.
    pub fn shown(self, amount: f64) -> f64 {
        match self {
            Self::Percent => (amount * 1000.0).round() / 10.0,
            Self::Multiplier => ((1.0 + amount) * 100.0).round() / 100.0,
            Self::Plain => (amount * 10.0).round() / 10.0,
        }
    }
}

/// Last week's trade prices of one riven kind, from the game's weekly riven trade report.
#[derive(Debug, Clone, PartialEq)]
pub struct RivenTrades {
    /// Riven kind as the game names it, e.g. `Rifle Riven Mod`.
    pub kind: String,
    /// English name of the weapon the rivens are for; `None` for veiled rivens.
    pub weapon: Option<String>,
    pub rerolled: bool,
    pub median: f64,
    pub average: f64,
    /// Standard deviation of the prices.
    pub deviation: f64,
    pub low: f64,
    pub high: f64,
    /// Popularity relative to other kinds, as the game rates it.
    pub popularity: f64,
}

/// One stat of a riven, with its roll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stat {
    /// Game stat key, e.g. `WeaponCritDamageMod`.
    pub tag: String,
    /// Position of the stat within its range, from 0 to 2³⁰.
    pub roll: u32,
}

/// Revealed (unveiled) riven, as the game data describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Riven {
    /// Game key of the weapon the riven is for.
    pub weapon: String,
    /// Mastery rank required to equip it.
    pub mastery: u8,
    /// How many times it has been rerolled.
    pub rerolls: u32,
    pub polarity: Option<Polarity>,
    /// Positive stats.
    pub buffs: Vec<Stat>,
    /// Negative stats.
    pub curses: Vec<Stat>,
}

/// Name syllables a stat contributes to a riven's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Affixes<'a> {
    pub tag: &'a str,
    pub prefix: &'a str,
    pub suffix: &'a str,
}

/// Name the game gives `riven`: buffs ordered by roll, highest first; the prefixes of all but the
/// last buff joined by hyphens, then the last buff's suffix, e.g. `Hexa-visitis`, `Critacron`.
/// `None` when a buff has no entry in `affixes`.
pub fn riven_name(riven: &Riven, affixes: &[Affixes]) -> Option<String> {
    let mut buffs: Vec<&Stat> = riven.buffs.iter().collect();
    buffs.sort_by_key(|stat| std::cmp::Reverse(stat.roll));
    let syllables = |stat: &Stat| affixes.iter().find(|each| each.tag == stat.tag);
    let (last, leading) = buffs.split_last()?;
    let mut name = leading
        .iter()
        .map(|stat| syllables(stat).map(|each| each.prefix))
        .collect::<Option<Vec<_>>>()?
        .join("-");
    name.push_str(syllables(last)?.suffix);
    (!name.is_empty()).then(|| riven_cased(&name))
}

/// Riven name capitalised as the game writes it: first letter upper case, the rest unchanged;
/// `hexa-visitis` becomes `Hexa-visitis`.
pub fn riven_cased(name: &str) -> String {
    let mut letters = name.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

/// Number of buffs and curses on a riven; the layout sets the scale factor of every stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    TwoBuffs,
    TwoBuffsOneCurse,
    ThreeBuffs,
    ThreeBuffsOneCurse,
}

impl Layout {
    /// Layout for this many buffs and curses; `None` for a combination the game never rolls.
    pub fn of(buffs: usize, curses: usize) -> Option<Self> {
        match (buffs, curses) {
            (2, 0) => Some(Self::TwoBuffs),
            (2, 1) => Some(Self::TwoBuffsOneCurse),
            (3, 0) => Some(Self::ThreeBuffs),
            (3, 1) => Some(Self::ThreeBuffsOneCurse),
            _ => None,
        }
    }

    /// Scale factor of a buff in this layout.
    pub fn buff_factor(self) -> f64 {
        match self {
            Self::TwoBuffs => 0.99,
            Self::TwoBuffsOneCurse => 1.2375,
            Self::ThreeBuffs => 0.75,
            Self::ThreeBuffsOneCurse => 0.9375,
        }
    }

    /// Scale factor of a curse in this layout, negative because a curse inverts its stat; `None`
    /// for a layout without a curse.
    pub fn curse_factor(self) -> Option<f64> {
        match self {
            Self::TwoBuffsOneCurse => Some(-0.495),
            Self::ThreeBuffsOneCurse => Some(-0.75),
            Self::TwoBuffs | Self::ThreeBuffs => None,
        }
    }
}

/// Maximum riven rank.
pub const RIVEN_TOP_RANK: u8 = 8;

/// Multiplier from the catalog's per-stat base to the stat value at the top rank.
const STAT_SCALE: f64 = 90.0;

/// Maximum deviation of a roll from the middle of the stat's range, as a fraction (±10%).
const ROLL_SPREAD: f64 = 0.1;

/// Roll value at the top of a stat's range.
const ROLL_TOP: f64 = (1u32 << 30) as f64;

/// Scale factor a roll applies to its stat: 0.9 at the bottom of the range, 1.1 at the top.
pub fn roll_factor(roll: u32) -> f64 {
    1.0 - ROLL_SPREAD + 2.0 * ROLL_SPREAD * f64::from(roll) / ROLL_TOP
}

/// Roll that gives the scale factor `factor`, clamped to the range.
pub fn roll_of(factor: f64) -> u32 {
    let share = (factor - (1.0 - ROLL_SPREAD)) / (2.0 * ROLL_SPREAD);
    (share.clamp(0.0, 1.0) * ROLL_TOP).round() as u32
}

/// Position of a roll within its stat's range, from 0.0 (bottom) to 1.0 (top).
pub fn roll_share(roll: u32) -> f64 {
    f64::from(roll) / ROLL_TOP
}

/// Stat value on a riven as a fraction: `2.744` is +274.4%. `base` is the class's per-stat base
/// from the catalog, `factor` the layout's factor for the stat's side (buff or curse).
pub fn stat_amount(base: f64, disposition: f64, factor: f64, roll: u32, rank: u8) -> f64 {
    let ranked = f64::from(rank.min(RIVEN_TOP_RANK) + 1) / f64::from(RIVEN_TOP_RANK + 1);
    base * STAT_SCALE * disposition * factor * roll_factor(roll) * ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    const AFFIXES: &[Affixes] = &[
        Affixes {
            tag: "WeaponDamageAmountMod",
            prefix: "visi",
            suffix: "ata",
        },
        Affixes {
            tag: "WeaponStunChanceMod",
            prefix: "hexa",
            suffix: "dex",
        },
        Affixes {
            tag: "WeaponCritDamageMod",
            prefix: "acri",
            suffix: "tis",
        },
        Affixes {
            tag: "WeaponCritChanceMod",
            prefix: "crita",
            suffix: "cron",
        },
    ];

    fn named(buffs: &[(&str, u32)]) -> Option<String> {
        let riven = Riven {
            weapon: "/Gun".into(),
            mastery: 8,
            rerolls: 0,
            polarity: None,
            buffs: buffs
                .iter()
                .map(|(tag, roll)| Stat {
                    tag: (*tag).into(),
                    roll: *roll,
                })
                .collect(),
            curses: Vec::new(),
        };
        riven_name(&riven, AFFIXES)
    }

    #[test]
    fn a_riven_is_named_by_its_buffs_from_the_highest_roll_down() {
        assert_eq!(
            named(&[
                ("WeaponDamageAmountMod", 469_376_964),
                ("WeaponStunChanceMod", 880_276_416),
                ("WeaponCritDamageMod", 141_367_479),
            ])
            .as_deref(),
            Some("Hexa-visitis")
        );
        assert_eq!(
            named(&[("WeaponCritDamageMod", 10), ("WeaponCritChanceMod", 20)]).as_deref(),
            Some("Critatis")
        );
        assert_eq!(
            named(&[("WeaponZoomFovMod", 1), ("WeaponCritChanceMod", 2)]),
            None
        );
    }

    const DUAL_TOXOCYST: f64 = 1.35;

    fn shown(base: f64, factor: f64, roll: u32) -> f64 {
        (stat_amount(base, DUAL_TOXOCYST, factor, roll, RIVEN_TOP_RANK) * 1000.0).round() / 10.0
    }

    #[test]
    fn a_riven_comes_to_the_numbers_the_game_shows_for_it() {
        let layout = Layout::of(3, 1).expect("a layout the game rolls");

        assert_eq!(shown(0.0244, layout.buff_factor(), 469_376_964), 274.4);
        assert_eq!(
            shown(0.0099999998, layout.buff_factor(), 880_276_416),
            121.2
        );
        assert_eq!(
            shown(0.0099999998, layout.buff_factor(), 141_367_479),
            105.5
        );
        assert_eq!(
            shown(
                -0.0099999998,
                layout.curse_factor().expect("a curse"),
                141_767_491
            ),
            84.4
        );
    }

    #[test]
    fn a_rank_zero_riven_holds_a_ninth_of_its_top() {
        let top = stat_amount(0.01, 1.0, 1.0, 0, RIVEN_TOP_RANK);

        assert!((stat_amount(0.01, 1.0, 1.0, 0, 0) - top / 9.0).abs() < 1e-12);
    }

    #[test]
    fn a_roll_spans_a_tenth_either_way() {
        assert!((roll_factor(0) - 0.9).abs() < 1e-12);
        assert!((roll_factor(1 << 30) - 1.1).abs() < 1e-12);
    }

    #[test]
    fn only_the_layouts_the_game_rolls_have_a_layout() {
        assert_eq!(Layout::of(1, 0), None);
        assert_eq!(Layout::of(3, 2), None);
        assert_eq!(Layout::of(2, 1), Some(Layout::TwoBuffsOneCurse));
    }

    #[test]
    fn a_factor_gives_back_its_roll() {
        assert_eq!(roll_of(roll_factor(123_456_789)), 123_456_789);
        assert_eq!(roll_of(0.5), 0);
        assert_eq!(roll_of(1.5), 1 << 30);
    }

    #[test]
    fn a_stat_is_shown_as_the_market_lists_it() {
        assert_eq!(StatForm::Percent.shown(2.7441), 274.4);
        assert_eq!(StatForm::Percent.shown(-0.6214), -62.1);
        assert_eq!(StatForm::Multiplier.shown(0.4213), 1.42);
        assert_eq!(StatForm::Multiplier.shown(-0.4588), 0.54);
        assert_eq!(StatForm::Plain.shown(3.44), 3.4);
    }
}
