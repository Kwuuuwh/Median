/// Endo needed to rank a mod of this rarity from 0 to `max_rank`; `None` for a rarity that cannot
/// be fused. Example: legendary to rank 10 takes 40,920 endo.
pub fn endo_to_max(rarity: &str, max_rank: u32) -> Option<u64> {
    let base: u64 = match rarity {
        "common" => 10,
        "uncommon" | "peculiar" => 20,
        "rare" => 30,
        "legendary" | "archon" => 40,
        _ => return None,
    };
    let ranks = 1u64.checked_shl(max_rank)?.checked_sub(1)?;
    base.checked_mul(ranks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_legendary_mod_takes_forty_thousand_endo_to_rank_ten() {
        assert_eq!(endo_to_max("legendary", 10), Some(40_920));
        assert_eq!(endo_to_max("archon", 10), Some(40_920));
        assert_eq!(endo_to_max("uncommon", 10), Some(20_460));
        assert_eq!(endo_to_max("rare", 5), Some(930));
    }

    #[test]
    fn a_rarity_that_is_not_fused_takes_nothing() {
        assert_eq!(endo_to_max("riven", 8), None);
        assert_eq!(endo_to_max("", 3), None);
    }
}
