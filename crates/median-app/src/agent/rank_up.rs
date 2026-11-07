use median_core::{Instant, endo_to_max};

use crate::account;
use crate::agent::gear::Gear;
use crate::market::{Sighted, Variant, sighted};

/// Market reading of the max-rank copy of a rank-0 mod, and the endo needed to rank it up;
/// `None` when the policy does not rank up, the pair is not a rank-0 mod, or the reading fails.
pub(crate) async fn ranked_resale(
    gear: &Gear,
    slug: &str,
    of: &Variant,
    now: Instant,
) -> Option<(Sighted, u64)> {
    if !gear.policy().rank_up || of.rank != Some(0) {
        return None;
    }
    let (rarity, top) = gear.market_cache.fused(slug).await.ok().flatten()?;
    let endo = endo_to_max(&rarity, u32::from(top))?;
    let ranked = Variant {
        rank: Some(top),
        ..of.clone()
    };
    let seen = sighted(&gear.market_cache, slug, &ranked, now).await.ok()?;
    Some((seen, endo))
}

/// Endo the account holds less the endo needed to rank up the rank-0 copies it holds and the
/// rank-0 copies the agent bids on; `None` when the endo balance or positions are unknown.
pub(crate) async fn endo_left(gear: &Gear, standing: &[account::Standing]) -> Option<u64> {
    let held = gear.holdings.read().inventory.purse.endo?;
    let positions = gear.history.open_positions().await.ok()?;
    let unranked = |variant: &str| Variant::parse(variant).rank == Some(0);
    let pending: Vec<(String, u32)> = positions
        .iter()
        .filter(|position| unranked(&position.variant))
        .map(|position| (position.slug.clone(), position.quantity))
        .chain(
            standing
                .iter()
                .filter(|order| order.by_agent && !order.selling && unranked(&order.variant))
                .filter_map(|order| order.slug.clone().map(|slug| (slug, order.quantity))),
        )
        .collect();

    let mut promised: u64 = 0;
    for (slug, copies) in pending {
        if let Ok(Some((rarity, top))) = gear.market_cache.fused(&slug).await
            && let Some(endo) = endo_to_max(&rarity, u32::from(top))
        {
            promised = promised.saturating_add(endo.saturating_mul(u64::from(copies)));
        }
    }
    Some(held.saturating_sub(promised))
}
