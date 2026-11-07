use crate::good_rolls::{GoodRolls, RivenGrade, StatVerdict};
use crate::riven::{Layout, Riven, Stat as RivenStat, StatForm, roll_share, stat_amount};

/// Stat a riven class can roll, with its base value from the catalog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolStat<'a> {
    pub tag: &'a str,
    pub base: f64,
    /// Whether a riven can roll it as a buff.
    pub can_buff: bool,
    /// Whether a riven can roll it as a curse.
    pub can_curse: bool,
    pub form: StatForm,
}

/// One stat of an appraised riven.
#[derive(Debug, Clone, PartialEq)]
pub struct Appraised {
    pub tag: String,
    pub is_curse: bool,
    /// Stat value as a fraction: `2.744` is +274.4%.
    pub amount: f64,
    /// Position of the roll within the stat's range, as a fraction from bottom to top.
    pub roll_share: f64,
    /// How useful the stat is on the weapon; `None` when the good-rolls list has no entry for
    /// the weapon.
    pub verdict: Option<StatVerdict>,
}

/// Appraisal of a riven for its weapon: an overall grade and a verdict per stat.
#[derive(Debug, Clone, PartialEq)]
pub struct Appraisal {
    /// `None` when the good-rolls list has no entry for the weapon.
    pub grade: Option<RivenGrade>,
    /// Buffs, then curses, in the order the game lists them.
    pub stats: Vec<Appraised>,
}

/// Each stat of `riven` at `rank` with whether it is a curse and its value as a fraction. `None`
/// when the riven has a stat outside its class pool or a buff/curse count the game never rolls.
pub fn rolled_stats<'r>(
    riven: &'r Riven,
    pool: &[PoolStat],
    disposition: f64,
    rank: u8,
) -> Option<Vec<(&'r RivenStat, bool, f64)>> {
    let layout = Layout::of(riven.buffs.len(), riven.curses.len())?;
    let sides = riven
        .buffs
        .iter()
        .map(|stat| (stat, false, Some(layout.buff_factor())))
        .chain(
            riven
                .curses
                .iter()
                .map(|stat| (stat, true, layout.curse_factor())),
        );
    sides
        .map(|(stat, is_curse, factor)| {
            let pooled = pool.iter().find(|pooled| pooled.tag == stat.tag)?;
            Some((
                stat,
                is_curse,
                stat_amount(pooled.base, disposition, factor?, stat.roll, rank),
            ))
        })
        .collect()
}

/// Appraises `riven` at `rank` against the good rolls of its weapon; `None` when the riven has a
/// stat outside its class pool or a buff/curse count the game never rolls.
pub fn appraise(
    rolls: Option<&GoodRolls>,
    disposition: f64,
    pool: &[PoolStat],
    riven: &Riven,
    rank: u8,
) -> Option<Appraisal> {
    let stats = rolled_stats(riven, pool, disposition, rank)?
        .into_iter()
        .map(|(stat, is_curse, amount)| Appraised {
            tag: stat.tag.clone(),
            is_curse,
            amount,
            roll_share: roll_share(stat.roll),
            verdict: rolls.map(|rolls| match is_curse {
                true => rolls.curse(&stat.tag),
                false => rolls.buff(&stat.tag),
            }),
        })
        .collect();
    Some(Appraisal {
        grade: rolls.map(|rolls| grade(rolls, riven)),
        stats,
    })
}

/// Grade of `riven` by the good rolls of its weapon.
pub fn grade(rolls: &GoodRolls, riven: &Riven) -> RivenGrade {
    let buffs: Vec<&str> = riven.buffs.iter().map(|stat| stat.tag.as_str()).collect();
    let curses: Vec<&str> = riven.curses.iter().map(|stat| stat.tag.as_str()).collect();
    rolls.grade(&buffs, &curses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riven::{RIVEN_TOP_RANK, Stat};

    const POOL: &[PoolStat] = &[
        PoolStat {
            tag: "WeaponDamageAmountMod",
            base: 0.0244,
            can_buff: true,
            can_curse: true,
            form: StatForm::Percent,
        },
        PoolStat {
            tag: "WeaponStunChanceMod",
            base: 0.0099999998,
            can_buff: true,
            can_curse: true,
            form: StatForm::Percent,
        },
        PoolStat {
            tag: "WeaponCritDamageMod",
            base: 0.0099999998,
            can_buff: true,
            can_curse: true,
            form: StatForm::Percent,
        },
        PoolStat {
            tag: "WeaponRecoilReductionMod",
            base: -0.0099999998,
            can_buff: true,
            can_curse: true,
            form: StatForm::Percent,
        },
    ];

    fn hexa_visitis() -> Riven {
        let stat = |tag: &str, roll| Stat {
            tag: tag.into(),
            roll,
        };
        Riven {
            weapon: "/Lotus/Weapons/Infested/Pistols/InfVomitGun/InfVomitGunWep".into(),
            mastery: 14,
            rerolls: 114,
            polarity: None,
            buffs: vec![
                stat("WeaponDamageAmountMod", 469_376_964),
                stat("WeaponStunChanceMod", 880_276_416),
                stat("WeaponCritDamageMod", 141_367_479),
            ],
            curses: vec![stat("WeaponRecoilReductionMod", 141_767_491)],
        }
    }

    #[test]
    fn a_riven_is_appraised_at_the_numbers_the_game_shows() {
        let (rolls, _) = GoodRolls::parse("CD MS/TOX/DMG/SC", "REC/ZOOM");
        let appraisal = appraise(Some(&rolls), 1.35, POOL, &hexa_visitis(), RIVEN_TOP_RANK)
            .expect("a riven the class rolls");

        let shown: Vec<f64> = appraisal
            .stats
            .iter()
            .map(|stat| (stat.amount * 1000.0).round() / 10.0)
            .collect();
        assert_eq!(shown, vec![274.4, 121.2, 105.5, 84.4]);
        assert!(appraisal.stats[3].is_curse);
        assert_eq!(appraisal.stats[2].verdict, Some(StatVerdict::Decisive));
        assert_eq!(appraisal.stats[3].verdict, Some(StatVerdict::Harmless));
        assert_eq!(appraisal.grade, Some(RivenGrade::Perfect));
    }

    #[test]
    fn a_weapon_the_good_rolls_leave_out_is_appraised_without_a_grade() {
        let appraisal =
            appraise(None, 1.35, POOL, &hexa_visitis(), RIVEN_TOP_RANK).expect("appraised");
        assert_eq!(appraisal.grade, None);
        assert!(appraisal.stats.iter().all(|stat| stat.verdict.is_none()));
    }

    #[test]
    fn a_stat_the_class_cannot_roll_leaves_the_riven_unappraised() {
        let mut riven = hexa_visitis();
        riven.buffs[0].tag = "WeaponZoomFovMod".into();
        assert_eq!(appraise(None, 1.35, POOL, &riven, RIVEN_TOP_RANK), None);
    }
}
