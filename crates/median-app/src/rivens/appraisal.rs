use median_core::{
    Appraised, Polarity as CorePolarity, Riven as CoreRiven, RivenGrade as CoreRivenGrade,
    RivenStat as CoreRivenStat, StatForm as CoreStatForm, StatVerdict as CoreStatVerdict, appraise,
};
use median_ipc::{
    AppraisedStat, Polarity, Riven, RivenAppraisal, RivenGrade, StatForm, StatVerdict,
};

use super::asks::{Armory, pool_stats};
use super::good_rolls::Sheet;
use crate::catalog::{Catalog, Pooled};

/// Grade of `riven` at `rank` against its weapon's good-rolls sheet, with each stat's amount and
/// verdict, labels in `lang`; `None` when the weapon is not in the catalog or the riven has a
/// stat its class cannot roll.
pub async fn appraisal(
    catalog: &Catalog,
    sheet: &Sheet,
    lang: &str,
    riven: Riven,
    rank: u32,
) -> Result<Option<RivenAppraisal>, String> {
    let Some(armory) = Armory::of(catalog, lang, riven.weapon.clone()).await? else {
        return Ok(None);
    };
    let rolls = armory
        .armed
        .family
        .as_deref()
        .and_then(|family| sheet.of(family));
    tokio::task::spawn_blocking(move || {
        let bases = pool_stats(&armory.pool);
        let rank = u8::try_from(rank).unwrap_or(u8::MAX);
        let riven = to_core(&riven);

        let Some(appraisal) = appraise(
            rolls.as_ref(),
            armory.armed.disposition,
            &bases,
            &riven,
            rank,
        ) else {
            return Ok(None);
        };
        Ok(Some(RivenAppraisal {
            grade: appraisal.grade.map(to_grade),
            stats: appraisal
                .stats
                .iter()
                .map(|stat| to_wire(stat, &armory.pool))
                .collect(),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Converts a riven grade to its IPC form.
pub(super) fn to_grade(grade: CoreRivenGrade) -> RivenGrade {
    match grade {
        CoreRivenGrade::Bad => RivenGrade::Bad,
        CoreRivenGrade::Potential => RivenGrade::Potential,
        CoreRivenGrade::Good => RivenGrade::Good,
        CoreRivenGrade::Perfect => RivenGrade::Perfect,
    }
}

/// Converts an IPC riven to the core type; a mastery above 255 saturates.
pub(super) fn to_core(riven: &Riven) -> CoreRiven {
    let stat = |stat: &median_ipc::RivenStat| CoreRivenStat {
        tag: stat.tag.clone(),
        roll: stat.roll,
    };
    CoreRiven {
        weapon: riven.weapon.clone(),
        mastery: u8::try_from(riven.mastery).unwrap_or(u8::MAX),
        rerolls: riven.rerolls,
        polarity: riven.polarity.as_ref().map(|polarity| match polarity {
            Polarity::Madurai => CorePolarity::Madurai,
            Polarity::Vazarin => CorePolarity::Vazarin,
            Polarity::Naramon => CorePolarity::Naramon,
        }),
        buffs: riven.buffs.iter().map(stat).collect(),
        curses: riven.curses.iter().map(stat).collect(),
    }
}

fn to_wire(stat: &Appraised, pool: &[Pooled]) -> AppraisedStat {
    let pooled = pool.iter().find(|pooled| pooled.tag == stat.tag);
    AppraisedStat {
        tag: stat.tag.clone(),
        is_curse: stat.is_curse,
        label: pooled
            .map(|pooled| pooled.label.clone())
            .unwrap_or_default(),
        form: pooled
            .and_then(|pooled| CoreStatForm::from_name(&pooled.form))
            .map_or(StatForm::Percent, to_form),
        amount: stat.amount,
        roll_share: stat.roll_share,
        verdict: stat.verdict.map(|verdict| match verdict {
            CoreStatVerdict::Decisive => StatVerdict::Decisive,
            CoreStatVerdict::Wanted => StatVerdict::Wanted,
            CoreStatVerdict::Useless => StatVerdict::Useless,
            CoreStatVerdict::Harmless => StatVerdict::Harmless,
            CoreStatVerdict::Harmful => StatVerdict::Harmful,
        }),
    }
}

/// Converts a stat form to its IPC form.
pub(super) fn to_form(form: CoreStatForm) -> StatForm {
    match form {
        CoreStatForm::Percent => StatForm::Percent,
        CoreStatForm::Multiplier => StatForm::Multiplier,
        CoreStatForm::Plain => StatForm::Plain,
    }
}
