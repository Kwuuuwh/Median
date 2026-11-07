/// Riven stat tags for one good-rolls sheet abbreviation, e.g. `DMG`, matched case-insensitively;
/// `None` for an unknown abbreviation.
fn tags_of(abbreviation: &str) -> Option<&'static [&'static str]> {
    let tags: &'static [&'static str] = match abbreviation.to_ascii_uppercase().as_str() {
        "CD" => &["WeaponCritDamageMod"],
        "CC" => &["WeaponCritChanceMod"],
        "DMG" => &["WeaponDamageAmountMod", "WeaponMeleeDamageMod"],
        "MS" => &["WeaponFireIterationsMod"],
        "FR" | "AS" => &["WeaponFireRateMod"],
        "RLS" => &["WeaponReloadSpeedMod"],
        "TOX" => &["WeaponToxinDamageMod"],
        "HEAT" => &["WeaponFireDamageMod"],
        "COLD" => &["WeaponFreezeDamageMod"],
        "ELEC" => &["WeaponElectricityDamageMod"],
        "ELEMENT" => &[
            "WeaponToxinDamageMod",
            "WeaponFireDamageMod",
            "WeaponFreezeDamageMod",
            "WeaponElectricityDamageMod",
        ],
        "DTC" => &[
            "WeaponFactionDamageCorpus",
            "WeaponMeleeFactionDamageCorpus",
        ],
        "DTI" => &[
            "WeaponFactionDamageInfested",
            "WeaponMeleeFactionDamageInfested",
        ],
        "DTG" => &[
            "WeaponFactionDamageGrineer",
            "WeaponMeleeFactionDamageGrineer",
        ],
        "PUNC" => &["WeaponArmorPiercingDamageMod"],
        "IMP" => &["WeaponImpactDamageMod"],
        "SLASH" => &["WeaponSlashDamageMod"],
        "MAG" => &["WeaponClipMaxMod"],
        "REC" | "RECOIL" => &["WeaponRecoilReductionMod"],
        "SC" => &["WeaponStunChanceMod"],
        "SD" => &["WeaponProcTimeMod"],
        "PT" => &["WeaponPunctureDepthMod"],
        "PFS" => &["WeaponProjectileSpeedMod"],
        "IC" => &["WeaponMeleeComboInitialBonusMod"],
        "EFF" => &["WeaponMeleeComboEfficiencyMod"],
        "SLIDE" => &["SlideAttackCritChanceMod"],
        "FIN" => &["WeaponMeleeFinisherDamageMod"],
        "ZOOM" => &["WeaponZoomFovMod"],
        "RANGE" => &["WeaponMeleeRangeIncMod"],
        "AMMO" => &["WeaponAmmoMaxMod"],
        _ => return None,
    };
    Some(tags)
}

/// One good stat combination for a riven: mandatory stats, each satisfied by any of its tags, and
/// optional stats that may fill the remaining slots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoodRoll {
    pub mandatory: Vec<Vec<&'static str>>,
    pub optional: Vec<&'static str>,
}

impl GoodRoll {
    fn is_mandatory(&self, tag: &str) -> bool {
        self.mandatory.iter().any(|slot| slot.contains(&tag))
    }

    fn is_optional(&self, tag: &str) -> bool {
        self.optional.contains(&tag)
    }

    /// Whether `buffs` include every mandatory stat and nothing outside the mandatory and
    /// optional stats.
    fn is_met_by(&self, buffs: &[&str]) -> bool {
        self.mandatory
            .iter()
            .all(|slot| slot.iter().any(|tag| buffs.contains(tag)))
            && buffs
                .iter()
                .all(|buff| self.is_mandatory(buff) || self.is_optional(buff))
    }
}

/// Good rolls for one weapon's riven, and the curses that do no harm.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoodRolls {
    pub rolls: Vec<GoodRoll>,
    pub harmless: Vec<&'static str>,
}

/// Usefulness of one riven stat on its weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatVerdict {
    /// A buff that is mandatory in some good roll.
    Decisive,
    /// A buff that is optional in some good roll.
    Wanted,
    /// A buff that no good roll includes, or a curse on a stat that no good roll includes.
    Useless,
    /// A curse the sheet marks as harmless.
    Harmless,
    /// A curse on a stat that some good roll includes.
    Harmful,
}

/// Overall grade of a riven for its weapon, from `Bad` to `Perfect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RivenGrade {
    Bad,
    Potential,
    Good,
    Perfect,
}

impl GoodRolls {
    /// Parses one sheet row from its positive and negative cells; also returns the tokens it
    /// could not parse.
    pub fn parse(positive: &str, negative: &str) -> (Self, Vec<String>) {
        let mut unknown = Vec::new();
        let mut read = |token: &str| -> Option<&'static [&'static str]> {
            let found = tags_of(token);
            if found.is_none() {
                unknown.push(token.to_owned());
            }
            found
        };

        let mut rolls = Vec::new();
        for variant in split_variants(positive) {
            let mut roll = GoodRoll::default();
            for token in variant.split_whitespace() {
                if token.contains('/') {
                    for part in token.split('/').filter(|part| !part.is_empty()) {
                        if let Some(tags) = read(part) {
                            roll.optional.extend_from_slice(tags);
                        }
                    }
                } else if let Some(tags) = read(token) {
                    roll.mandatory.push(tags.to_vec());
                }
            }
            if !roll.mandatory.is_empty() || !roll.optional.is_empty() {
                rolls.push(roll);
            }
        }

        let mut harmless = Vec::new();
        for part in negative.split(['/', ' ']).filter(|part| !part.is_empty()) {
            if let Some(tags) = read(part) {
                harmless.extend_from_slice(tags);
            }
        }
        (Self { rolls, harmless }, unknown)
    }

    /// Verdict for `tag` as a buff.
    pub fn buff(&self, tag: &str) -> StatVerdict {
        if self.rolls.iter().any(|roll| roll.is_mandatory(tag)) {
            StatVerdict::Decisive
        } else if self.rolls.iter().any(|roll| roll.is_optional(tag)) {
            StatVerdict::Wanted
        } else {
            StatVerdict::Useless
        }
    }

    /// Verdict for `tag` as a curse.
    pub fn curse(&self, tag: &str) -> StatVerdict {
        if self.harmless.contains(&tag) {
            StatVerdict::Harmless
        } else if self.buff(tag) == StatVerdict::Useless {
            StatVerdict::Useless
        } else {
            StatVerdict::Harmful
        }
    }

    /// Grade of a riven with these buffs and curses.
    pub fn grade(&self, buffs: &[&str], curses: &[&str]) -> RivenGrade {
        let helping = buffs
            .iter()
            .filter(|buff| self.buff(buff) != StatVerdict::Useless)
            .count();
        let is_met = self.rolls.iter().any(|roll| roll.is_met_by(buffs));
        let verdicts: Vec<StatVerdict> = curses.iter().map(|curse| self.curse(curse)).collect();

        if verdicts.contains(&StatVerdict::Harmful) {
            match (is_met && helping >= 2) || helping >= 3 {
                true => RivenGrade::Potential,
                false => RivenGrade::Bad,
            }
        } else if verdicts.contains(&StatVerdict::Useless) {
            match (is_met || helping >= 2, helping >= 1) {
                (true, _) => RivenGrade::Good,
                (false, true) => RivenGrade::Potential,
                (false, false) => RivenGrade::Bad,
            }
        } else if is_met {
            match helping >= 2 && !curses.is_empty() {
                true => RivenGrade::Perfect,
                false => RivenGrade::Good,
            }
        } else {
            match helping {
                0 => RivenGrade::Bad,
                1 => RivenGrade::Potential,
                _ => RivenGrade::Good,
            }
        }
    }
}

/// Variants of a positive cell, split on the word `or` in any case.
fn split_variants(positive: &str) -> Vec<String> {
    let mut variants = vec![String::new()];
    for word in positive.split_whitespace() {
        if word.eq_ignore_ascii_case("or") {
            variants.push(String::new());
        } else if let Some(last) = variants.last_mut() {
            last.push(' ');
            last.push_str(word);
        }
    }
    variants
}

#[cfg(test)]
mod tests {
    use super::*;

    const CD: &str = "WeaponCritDamageMod";
    const CC: &str = "WeaponCritChanceMod";
    const MS: &str = "WeaponFireIterationsMod";
    const TOX: &str = "WeaponToxinDamageMod";
    const ZOOM: &str = "WeaponZoomFovMod";
    const IMP: &str = "WeaponImpactDamageMod";
    const RANGE: &str = "WeaponMeleeRangeIncMod";
    const AS: &str = "WeaponFireRateMod";
    const ELEC: &str = "WeaponElectricityDamageMod";
    const SLIDE: &str = "SlideAttackCritChanceMod";

    fn soma() -> GoodRolls {
        GoodRolls::parse("CD MS/TOX/DMG/CC/FR", "IMP/ZOOM/REC").0
    }

    #[test]
    fn a_row_reads_into_stats_it_must_and_may_carry() {
        let (rolls, unknown) = GoodRolls::parse("CD AS RANGE or CD/RANGE AS ELEC", "EFF/SLIDE/FIN");
        assert!(unknown.is_empty());
        assert_eq!(rolls.rolls.len(), 2);
        assert_eq!(
            rolls.rolls[0].mandatory,
            vec![vec![CD], vec![AS], vec![RANGE]]
        );
        assert_eq!(rolls.rolls[1].mandatory, vec![vec![AS], vec![ELEC]]);
        assert_eq!(rolls.rolls[1].optional, vec![CD, RANGE]);
        assert!(rolls.harmless.contains(&SLIDE));
    }

    #[test]
    fn an_unknown_token_is_answered_and_the_rest_read() {
        let (rolls, unknown) = GoodRolls::parse("CD MS/WHAT", "ZOOM");
        assert_eq!(unknown, vec!["WHAT".to_owned()]);
        assert_eq!(rolls.rolls[0].optional, vec![MS]);
    }

    #[test]
    fn a_wanted_roll_with_a_harmless_curse_is_perfect() {
        assert_eq!(soma().grade(&[CD, MS, TOX], &[ZOOM]), RivenGrade::Perfect);
        assert_eq!(soma().grade(&[CD, MS], &[]), RivenGrade::Good);
    }

    #[test]
    fn a_curse_on_a_wanted_stat_drags_the_grade_down() {
        assert_eq!(soma().curse(CC), StatVerdict::Harmful);
        assert_eq!(soma().grade(&[CD, MS], &[CC]), RivenGrade::Potential);
        assert_eq!(soma().grade(&[CD, IMP], &[CC]), RivenGrade::Bad);
    }

    #[test]
    fn stats_nobody_asks_for_make_a_bad_riven() {
        assert_eq!(soma().buff(IMP), StatVerdict::Useless);
        assert_eq!(soma().grade(&[IMP, RANGE], &[ZOOM]), RivenGrade::Bad);
        assert_eq!(soma().grade(&[IMP, MS], &[ZOOM]), RivenGrade::Potential);
    }
}
