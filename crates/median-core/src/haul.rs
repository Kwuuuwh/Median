use std::collections::BTreeMap;

use crate::account::{Holding, Variant};
use crate::time::Instant;

/// Items the account gained since the haul started: for each inventory dump, the copies it holds
/// beyond the dump before it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Haul {
    /// Start of the haul; `None` until an inventory dump sets the baseline.
    pub started: Option<Instant>,
    /// Copies gained, by item and variant.
    pub gained: BTreeMap<(String, Variant), u32>,
    /// Items claimed from the foundry, counted by blueprint.
    pub claimed: BTreeMap<String, u32>,
}

/// Suffix of a relic's item key for each refinement.
const REFINEMENTS: [&str; 4] = ["Bronze", "Silver", "Gold", "Platinum"];

/// Key that copies of an item are netted under: a relic key without its refinement suffix, any
/// other item key unchanged, so all variants of an item net together.
fn kin(item: &str) -> &str {
    if !item.contains("/Projections/") {
        return item;
    }
    REFINEMENTS
        .iter()
        .find_map(|refinement| item.strip_suffix(refinement))
        .unwrap_or(item)
}

/// Holdings that `after` has more copies of than `before`, each with the number of extra copies.
/// Copies that only changed form inside the account (a relic refined, a mod ranked, a statue
/// upgraded) are not gains.
pub fn gains(before: &[Holding], after: &[Holding]) -> Vec<Holding> {
    let held: BTreeMap<(&str, &Variant), u32> = before
        .iter()
        .map(|holding| ((holding.item.as_str(), &holding.variant), holding.count))
        .collect();
    let mut net: BTreeMap<&str, i64> = BTreeMap::new();
    for holding in before {
        *net.entry(kin(&holding.item)).or_default() -= i64::from(holding.count);
    }
    for holding in after {
        *net.entry(kin(&holding.item)).or_default() += i64::from(holding.count);
    }
    after
        .iter()
        .filter_map(|holding| {
            let was = held
                .get(&(holding.item.as_str(), &holding.variant))
                .copied()
                .unwrap_or(0);
            let left = net.get_mut(kin(&holding.item))?;
            let more = i64::from(holding.count.saturating_sub(was)).min(*left);
            if more <= 0 {
                return None;
            }
            *left -= more;
            Some(Holding {
                count: more as u32,
                ..holding.clone()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holding(item: &str, count: u32) -> Holding {
        Holding {
            item: item.into(),
            variant: Variant::default(),
            detail: None,
            count,
        }
    }

    #[test]
    fn what_is_held_more_of_is_gained_and_what_was_spent_is_not() {
        let before = [holding("/Relic", 4), holding("/Part", 1)];
        let after = [
            holding("/Relic", 3),
            holding("/Part", 2),
            holding("/Mod", 1),
        ];

        let gained: Vec<(String, u32)> = gains(&before, &after)
            .into_iter()
            .map(|holding| (holding.item, holding.count))
            .collect();

        assert_eq!(gained, [("/Part".into(), 1), ("/Mod".into(), 1)]);
    }

    fn ranked(item: &str, variant: &str, count: u32) -> Holding {
        Holding {
            variant: Variant(variant.into()),
            ..holding(item, count)
        }
    }

    #[test]
    fn a_relic_refined_is_nothing_gained_and_one_dropped_beside_it_is() {
        let before = [
            holding("/Lotus/Types/Game/Projections/T1VoidProjectionABronze", 3),
            holding("/Lotus/Types/Game/Projections/T1VoidProjectionBBronze", 1),
        ];
        let after = [
            holding("/Lotus/Types/Game/Projections/T1VoidProjectionABronze", 2),
            holding("/Lotus/Types/Game/Projections/T1VoidProjectionAPlatinum", 1),
            holding("/Lotus/Types/Game/Projections/T1VoidProjectionBBronze", 2),
        ];

        let gained: Vec<(String, u32)> = gains(&before, &after)
            .into_iter()
            .map(|holding| (holding.item, holding.count))
            .collect();

        assert_eq!(
            gained,
            [(
                "/Lotus/Types/Game/Projections/T1VoidProjectionBBronze".into(),
                1
            )]
        );
    }

    #[test]
    fn a_copy_ranked_up_is_nothing_gained() {
        let before = [ranked("/Mod", "r0", 2)];
        let after = [ranked("/Mod", "r0", 1), ranked("/Mod", "r5", 1)];

        assert!(gains(&before, &after).is_empty());
    }

    #[test]
    fn nothing_moved_is_nothing_gained() {
        let held = [holding("/Part", 2)];

        assert!(gains(&held, &held).is_empty());
    }
}
