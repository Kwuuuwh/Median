use median_core::{
    ListedStat, Polarity as CorePolarity, RIVEN_TOP_RANK, Riven as CoreRiven, Unread, entered,
    riven_name,
};
use median_ipc::Riven;
use tracing::info;

use super::asks::{Armory, pool_stats};
use super::names::affixes;
use crate::catalog::{Catalog, Pooled};
use crate::dictionary::{Pool, apart, letters};

/// One stat line read from a riven card: its number with the sign shown, and the words after it.
#[derive(Debug, Clone, PartialEq)]
pub struct ShownLine {
    pub number: f64,
    pub words: String,
}

/// Text read from a riven card: the weapon and riven name, and the stat lines in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardText {
    pub name: String,
    pub stats: Vec<ShownLine>,
}

impl CardText {
    /// Riven name part of the card: the last word of `name`.
    fn riven_name(&self) -> &str {
        self.name.rsplit(' ').next().unwrap_or_default()
    }
}

/// A riven recognised from a card's text.
#[derive(Debug, Clone, PartialEq)]
pub struct CardRiven {
    pub riven: CoreRiven,
    /// Riven name built from its stats.
    pub name: String,
    /// Edit distance between that name and the name read from the card.
    pub apart: usize,
}

/// Recognises the riven on a card; `held`, the inventory riven shown on the screen, gives the
/// weapon, mastery, rerolls and polarity. `Unread::Unsupported` when the weapon has no riven
/// pool, `Unread::Stats` when the stats are not recognised.
pub async fn card_riven(
    catalog: &Catalog,
    lang: &str,
    held: &Riven,
    rank: u32,
    text: &CardText,
) -> Result<Result<CardRiven, Unread>, String> {
    let Some(armory) = Armory::of(catalog, lang, held.weapon.clone()).await? else {
        info!(
            weapon = held.weapon,
            "riven weapon unsupported, riven not read"
        );
        return Ok(Err(Unread::Unsupported));
    };
    let rank = u8::try_from(rank).unwrap_or(RIVEN_TOP_RANK);
    let read = riven_on(text, &armory.pool, armory.armed.disposition, held, rank);
    if read.is_none() {
        let stats: Vec<&str> = text.stats.iter().map(|line| line.words.as_str()).collect();
        info!(
            weapon = held.weapon,
            name = text.name,
            ?stats,
            "riven card stats not recognised"
        );
    }
    Ok(read.ok_or(Unread::Stats))
}

/// Riven that `text` describes on a weapon with this stat pool and disposition: of the possible
/// buff/curse splits, the one whose built name is closest to the card's name; `None` when no
/// split fits.
fn riven_on(
    text: &CardText,
    pool: &[Pooled],
    disposition: f64,
    held: &Riven,
    rank: u8,
) -> Option<CardRiven> {
    let words = Pool::folded(
        pool.iter()
            .filter(|stat| !stat.label.trim().is_empty())
            .map(|stat| (stat.tag.clone(), without_aside(&stat.label)))
            .collect(),
        letters,
    );
    let tags = text
        .stats
        .iter()
        .map(|line| words.nearest(&line.words).map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    let curses: &[usize] = match tags.len() {
        2 => &[0],
        3 => &[0, 1],
        4 => &[1],
        _ => return None,
    };
    let bases = pool_stats(pool);
    let syllables = affixes(pool);
    curses
        .iter()
        .filter_map(|&curses| {
            let buffs = tags.len() - curses;
            let listed: Vec<ListedStat> = tags
                .iter()
                .zip(&text.stats)
                .enumerate()
                .map(|(at, (tag, line))| ListedStat {
                    tag: tag.clone(),
                    shown: line.number,
                    positive: at < buffs,
                })
                .collect();
            let read = entered(&listed, &bases, disposition, rank)?;
            let core = CoreRiven {
                weapon: held.weapon.clone(),
                mastery: u8::try_from(held.mastery).unwrap_or(u8::MAX),
                rerolls: held.rerolls,
                polarity: held.polarity.clone().map(to_core_polarity),
                buffs: read.buffs,
                curses: read.curses,
            };
            let name = riven_name(&core, &syllables)?;
            Some(CardRiven {
                apart: apart(&name, text.riven_name()),
                riven: core,
                name,
            })
        })
        .min_by_key(|read| read.apart)
}

/// The label without a trailing parenthesised note such as `(x2 для луков)`.
fn without_aside(label: &str) -> String {
    match label.find(" (") {
        Some(at) => label[..at].to_owned(),
        None => label.to_owned(),
    }
}

fn to_core_polarity(polarity: median_ipc::Polarity) -> CorePolarity {
    match polarity {
        median_ipc::Polarity::Madurai => CorePolarity::Madurai,
        median_ipc::Polarity::Vazarin => CorePolarity::Vazarin,
        median_ipc::Polarity::Naramon => CorePolarity::Naramon,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Riven disposition of Dual Toxocyst in the catalog.
    const TOXOCYST: f64 = 1.35;

    fn stat(tag: &str, base: f64, label: &str, prefix: &str, suffix: &str) -> Pooled {
        Pooled {
            tag: tag.into(),
            base,
            form: "percent".into(),
            label: label.into(),
            can_buff: true,
            can_curse: true,
            slug: None,
            prefix: prefix.into(),
            suffix: suffix.into(),
        }
    }

    /// Pistol pool stats that the cards below use, with their catalog values.
    fn pistols() -> Vec<Pooled> {
        vec![
            stat("WeaponDamageAmountMod", 0.0244, "% урона", "visi", "ata"),
            stat(
                "WeaponStunChanceMod",
                0.0099999998,
                "% к шансу статуса",
                "hexa",
                "dex",
            ),
            stat(
                "WeaponCritDamageMod",
                0.0099999998,
                "% крит. урона",
                "acri",
                "tis",
            ),
            stat(
                "WeaponRecoilReductionMod",
                -0.0099999998,
                "% к отдаче",
                "zeti",
                "mag",
            ),
            stat(
                "WeaponFireIterationsMod",
                0.0133,
                "% мультивыстрел",
                "sati",
                "can",
            ),
            stat(
                "WeaponFireRateMod",
                0.0082999999,
                "% к скорострельности",
                "croni",
                "dra",
            ),
            stat("WeaponCritChanceMod", 0.0149, "", "crita", "cron"),
        ]
    }

    fn held() -> Riven {
        Riven {
            weapon: "/Lotus/Weapons/Infested/Pistols/InfVomitGun/InfVomitGunWep".into(),
            mastery: 14,
            rerolls: 117,
            polarity: Some(median_ipc::Polarity::Madurai),
            buffs: Vec::new(),
            curses: Vec::new(),
        }
    }

    fn line(number: f64, words: &str) -> ShownLine {
        ShownLine {
            number,
            words: words.into(),
        }
    }

    #[test]
    fn a_card_read_off_a_screenshot_names_its_riven() {
        let text = CardText {
            name: "Парные Токсоцисты VISItIS".into(),
            stats: vec![
                line(274.4, "уpона"),
                line(121.2, "кшансу статуса"),
                line(105.5, "крит.урона"),
                line(84.4, "к отдаче"),
            ],
        };

        let read = riven_on(&text, &pistols(), TOXOCYST, &held(), 8).expect("read");

        assert_eq!(read.name, "Hexa-visitis");
        assert_eq!(read.riven.buffs.len(), 3);
        assert_eq!(read.riven.curses[0].tag, "WeaponRecoilReductionMod");
    }

    #[test]
    fn the_rolled_card_beside_names_its_own_riven() {
        let text = CardText {
            name: "Парные. Токсоцисты Сrопи-saiis".into(),
            stats: vec![
                line(162.4, "мультивыстрел"),
                line(121.4, "крит.урона"),
                line(102.9, "K скорострельности"),
                line(86.0, "к отдаче"),
            ],
        };

        let read = riven_on(&text, &pistols(), TOXOCYST, &held(), 8).expect("read");

        assert_eq!(read.name, "Croni-satitis");
    }

    #[test]
    fn numbers_no_rank_explains_read_as_nothing() {
        let text = CardText {
            name: "Парные Токсоцисты Hexa-visitis".into(),
            stats: vec![line(900.0, "урона"), line(121.2, "к шансу статуса")],
        };

        assert_eq!(riven_on(&text, &pistols(), TOXOCYST, &held(), 8), None);
    }
}
