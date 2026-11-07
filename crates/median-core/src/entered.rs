use crate::appraisal::PoolStat;
use crate::asks::ListedStat;
use crate::riven::{Layout, RIVEN_TOP_RANK, Stat, roll_factor, roll_of, stat_amount};

/// Lowest and highest roll value of a stat's range.
const ROLL_BOUNDS: [u32; 2] = [0, 1 << 30];

/// Stat rolls recovered from the values a riven listing shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entered {
    pub buffs: Vec<Stat>,
    pub curses: Vec<Stat>,
}

/// Stat rolls behind the values `stats` shows on a riven at `rank`. When the values do not fit
/// `rank`, they are read at the top rank, since a seller may enter top-rank values for a
/// lower-rank riven. `None` when neither rank fits or a listed stat is not in `pool`.
pub fn entered(
    stats: &[ListedStat],
    pool: &[PoolStat],
    disposition: f64,
    rank: u8,
) -> Option<Entered> {
    [rank, RIVEN_TOP_RANK]
        .into_iter()
        .find_map(|reading| read_at(stats, pool, disposition, reading))
}

/// Stat rolls behind `stats` read at `rank`; `None` unless every value falls within its stat's
/// range, allowing for display rounding.
fn read_at(stats: &[ListedStat], pool: &[PoolStat], disposition: f64, rank: u8) -> Option<Entered> {
    let buffs = stats.iter().filter(|stat| stat.positive).count();
    let layout = Layout::of(buffs, stats.len() - buffs)?;
    let mut read = Entered {
        buffs: Vec::new(),
        curses: Vec::new(),
    };
    for stat in stats {
        let pooled = pool.iter().find(|pooled| pooled.tag == stat.tag)?;
        let factor = if stat.positive {
            layout.buff_factor()
        } else {
            layout.curse_factor()?
        };
        let at = |roll| stat_amount(pooled.base, disposition, factor, roll, rank);
        let [bottom, top] = ROLL_BOUNDS.map(at);
        let slack = pooled.form.rounding();
        let amount = pooled.form.amount(stat.shown);
        if amount < bottom.min(top) - slack || amount > bottom.max(top) + slack {
            return None;
        }
        let roll = roll_of(amount / at(0) * roll_factor(0));
        let side = if stat.positive {
            &mut read.buffs
        } else {
            &mut read.curses
        };
        side.push(Stat {
            tag: stat.tag.clone(),
            roll,
        });
    }
    Some(read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riven::StatForm;

    const DISPOSITION: f64 = 1.35;

    fn pool() -> Vec<PoolStat<'static>> {
        let stat = |tag, base, form| PoolStat {
            tag,
            base,
            can_buff: true,
            can_curse: true,
            form,
        };
        vec![
            stat("WeaponDamageAmountMod", 1.8079, StatForm::Percent),
            stat("WeaponStunChanceMod", 0.01, StatForm::Percent),
            stat("WeaponFactionDamageCorpus", 0.005, StatForm::Multiplier),
        ]
    }

    fn shown(tag: &str, roll: u32, rank: u8, positive: bool) -> ListedStat {
        let pool = pool();
        let pooled = pool.iter().find(|pooled| pooled.tag == tag).unwrap();
        let layout = Layout::TwoBuffsOneCurse;
        let factor = if positive {
            layout.buff_factor()
        } else {
            layout.curse_factor().unwrap()
        };
        ListedStat {
            tag: tag.to_owned(),
            shown: pooled
                .form
                .shown(stat_amount(pooled.base, DISPOSITION, factor, roll, rank)),
            positive,
        }
    }

    fn lot(rank: u8) -> Vec<ListedStat> {
        vec![
            shown("WeaponDamageAmountMod", 700_000_000, rank, true),
            shown("WeaponStunChanceMod", 200_000_000, rank, true),
            shown("WeaponFactionDamageCorpus", 900_000_000, rank, false),
        ]
    }

    /// Values `read` shows at `rank`, in the form a listing carries them.
    fn shows(read: &Entered, rank: u8) -> Vec<ListedStat> {
        let stat = |stat: &Stat, positive| shown(&stat.tag, stat.roll, rank, positive);
        read.buffs
            .iter()
            .map(|each| stat(each, true))
            .chain(read.curses.iter().map(|each| stat(each, false)))
            .collect()
    }

    #[test]
    fn a_shown_riven_gives_back_rolls_that_show_the_same() {
        let read = entered(&lot(8), &pool(), DISPOSITION, 8).unwrap();

        assert_eq!(shows(&read, 8), lot(8));
    }

    #[test]
    fn top_numbers_entered_for_rank_zero_are_read_at_the_top() {
        let read = entered(&lot(8), &pool(), DISPOSITION, 0).unwrap();

        assert_eq!(shows(&read, 8), lot(8));
    }

    #[test]
    fn numbers_no_rank_explains_read_as_nothing() {
        let mut stats = lot(8);
        stats[0].shown *= 3.0;

        assert_eq!(entered(&stats, &pool(), DISPOSITION, 8), None);
    }
}
