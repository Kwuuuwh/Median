use median_core::{StatForm as CoreStatForm, riven_name, rolled_stats};
use median_ipc::{Polarity, Riven};
use median_market::{AuctionDraft, AuctionStat};

use super::appraisal::to_core;
use super::asks::{Armory, pool_stats};
use super::names::affixes;
use crate::catalog::Catalog;

/// Builds the warframe.market auction for `riven` at `rank` with starting price `starting` and
/// an optional buyout price; `Err` when the weapon, its market slug, the polarity, the name
/// affixes or a stat's market slug is missing.
pub async fn draft(
    catalog: &Catalog,
    riven: Riven,
    rank: u32,
    starting: u32,
    buyout: Option<u32>,
) -> Result<AuctionDraft, String> {
    let weapon = riven.weapon.clone();
    let armory = Armory::of(catalog, "en", weapon.clone())
        .await?
        .ok_or_else(|| format!("median does not value rivens of {weapon}"))?;
    let slug = armory
        .armed
        .slug
        .clone()
        .ok_or_else(|| format!("the market does not list rivens of {weapon}"))?;
    let polarity = riven
        .polarity
        .as_ref()
        .map(|polarity| match polarity {
            Polarity::Madurai => "madurai",
            Polarity::Vazarin => "vazarin",
            Polarity::Naramon => "naramon",
        })
        .ok_or("the riven has no polarity")?;
    let core = to_core(&riven);
    let name = riven_name(&core, &affixes(&armory.pool))
        .ok_or("the catalog lacks the syllables of the riven's name")?;

    let bases = pool_stats(&armory.pool);
    let rolled = rolled_stats(
        &core,
        &bases,
        armory.armed.disposition,
        u8::try_from(rank).unwrap_or(u8::MAX),
    )
    .ok_or("the riven holds a stat its class cannot roll")?;
    let stats = rolled
        .into_iter()
        .map(|(stat, is_curse, amount)| {
            let pooled = armory
                .pool
                .iter()
                .find(|pooled| pooled.tag == stat.tag)
                .ok_or_else(|| format!("the pool lacks {}", stat.tag))?;
            let slug = pooled
                .slug
                .clone()
                .ok_or_else(|| format!("the market has no slug for {}", stat.tag))?;
            let form = CoreStatForm::from_name(&pooled.form).unwrap_or(CoreStatForm::Percent);
            Ok(AuctionStat {
                slug,
                value: form.shown(amount),
                positive: !is_curse,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(AuctionDraft {
        weapon: slug,
        name: name.to_lowercase(),
        mastery: riven.mastery,
        rerolls: riven.rerolls,
        rank,
        polarity: polarity.to_owned(),
        stats,
        starting_price: starting,
        buyout_price: buyout,
        minimal_reputation: 0,
        visible: true,
        note: String::new(),
    })
}
