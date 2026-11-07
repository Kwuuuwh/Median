/// Account's standing and rank with one syndicate, as the inventory dump reports them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affiliation {
    /// Syndicate tag in the game data, e.g. `CetusSyndicate`.
    pub tag: String,
    pub standing: i64,
    /// Rank with the syndicate; below zero while it is hostile.
    pub title: i32,
}

/// Standing still available to earn today from one daily standing pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowance {
    /// Pool suffix the game uses, e.g. `Cetus`; empty for the pool the faction syndicates share.
    pub pool: String,
    pub left: u32,
}

/// Syndicate entry of `KNOWN`: its game tag, the daily pool its standing is earned from, and the
/// vendors that accept that standing.
struct Known {
    tag: &'static str,
    pool: &'static str,
    vendors: &'static [&'static str],
}

/// Standing needed at each rank to reach the next one, starting at rank 0. The dump's standing
/// value is the total earned, these amounts included.
const RANK_CAPS: [i64; 6] = [5_000, 22_000, 44_000, 70_000, 99_000, 132_000];

/// The syndicate whose standing field holds a currency of its own rather than standing.
const STOCKED: &str = "KahlSyndicate";

/// Spendable standing: the total earned minus the standing needed for the ranks reached, never
/// below 0. For `STOCKED`, or at rank 0 or below, the dump value as is.
pub fn spendable(affiliation: &Affiliation) -> i64 {
    if affiliation.tag == STOCKED || affiliation.title <= 0 {
        return affiliation.standing.max(0);
    }
    let reached = (affiliation.title as usize).min(RANK_CAPS.len());
    let held: i64 = RANK_CAPS[..reached].iter().sum();
    (affiliation.standing - held).max(0)
}

/// The pool the six faction syndicates share.
const FACTIONS: &str = "";

const KNOWN: &[Known] = &[
    Known {
        tag: "SteelMeridianSyndicate",
        pool: FACTIONS,
        vendors: &["steel-meridian"],
    },
    Known {
        tag: "ArbitersSyndicate",
        pool: FACTIONS,
        vendors: &["arbiters-of-hexis"],
    },
    Known {
        tag: "CephalonSudaSyndicate",
        pool: FACTIONS,
        vendors: &["cephalon-suda"],
    },
    Known {
        tag: "PerrinSyndicate",
        pool: FACTIONS,
        vendors: &["the-perrin-sequence"],
    },
    Known {
        tag: "RedVeilSyndicate",
        pool: FACTIONS,
        vendors: &["red-veil"],
    },
    Known {
        tag: "NewLokaSyndicate",
        pool: FACTIONS,
        vendors: &["new-loka"],
    },
    Known {
        tag: "LibrarySyndicate",
        pool: "Library",
        vendors: &["cephalon-simaris"],
    },
    Known {
        tag: "CetusSyndicate",
        pool: "Cetus",
        vendors: &["hok"],
    },
    Known {
        tag: "QuillsSyndicate",
        pool: "Quills",
        vendors: &["the-quills"],
    },
    Known {
        tag: "SolarisSyndicate",
        pool: "Solaris",
        vendors: &["legs", "rude-zuud"],
    },
    Known {
        tag: "VentKidsSyndicate",
        pool: "Ventkids",
        vendors: &["ventkids"],
    },
    Known {
        tag: "VoxSyndicate",
        pool: "Vox",
        vendors: &["vox-solaris"],
    },
    Known {
        tag: "EntratiSyndicate",
        pool: "Entrati",
        vendors: &["father"],
    },
    Known {
        tag: "ZarimanSyndicate",
        pool: "Zariman",
        vendors: &["hombask"],
    },
    Known {
        tag: "EntratiLabSyndicate",
        pool: "Cavia",
        vendors: &["bird-3"],
    },
    Known {
        tag: "KahlSyndicate",
        pool: "Kahl",
        vendors: &["kahl"],
    },
    Known {
        tag: "HexSyndicate",
        pool: "Hex",
        vendors: &["amir", "aoi", "quincy", "minerva", "velimir"],
    },
];

/// Account's standing with one syndicate, and the vendors that accept it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allegiance {
    /// Syndicate tag in the game data, e.g. `CetusSyndicate`.
    pub tag: &'static str,
    /// Vendors that accept its standing.
    pub vendors: &'static [&'static str],
    /// Whether the account has any affiliation with the syndicate.
    pub met: bool,
    /// Spendable standing, as `spendable` computes it.
    pub standing: i64,
    /// Rank with the syndicate; below zero while it is hostile.
    pub title: i32,
    /// Standing still available to earn today; `None` until an inventory dump in this run
    /// reports it.
    pub left_today: Option<u32>,
}

/// Every syndicate in `KNOWN`: those the account is affiliated with first, by spendable standing,
/// highest first; then the rest in table order.
pub fn allegiances(affiliations: &[Affiliation], allowances: &[Allowance]) -> Vec<Allegiance> {
    let mut found: Vec<Allegiance> = KNOWN
        .iter()
        .map(|known| {
            let met = affiliations
                .iter()
                .find(|affiliation| affiliation.tag == known.tag);
            Allegiance {
                tag: known.tag,
                vendors: known.vendors,
                met: met.is_some(),
                standing: met.map_or(0, spendable),
                title: met.map_or(0, |affiliation| affiliation.title),
                left_today: allowances
                    .iter()
                    .find(|allowance| allowance.pool == known.pool)
                    .map(|allowance| allowance.left),
            }
        })
        .collect();
    found.sort_by_key(|allegiance| (!allegiance.met, -allegiance.standing));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn affiliated(tag: &str, standing: i64, title: i32) -> Affiliation {
        Affiliation {
            tag: tag.into(),
            standing,
            title,
        }
    }

    fn of<'a>(found: &'a [Allegiance], tag: &str) -> &'a Allegiance {
        found
            .iter()
            .find(|allegiance| allegiance.tag == tag)
            .expect("listed")
    }

    #[test]
    fn the_ranks_reached_hold_standing_that_cannot_be_spent() {
        assert_eq!(
            spendable(&affiliated("EntratiSyndicate", 294_800, 5)),
            54_800
        );
        assert_eq!(spendable(&affiliated("VoxSyndicate", 153_000, 4)), 12_000);
        assert_eq!(spendable(&affiliated("NewLokaSyndicate", 27_000, 2)), 0);
    }

    #[test]
    fn neither_a_hostile_syndicate_nor_kahls_stock_holds_anything_back() {
        assert_eq!(spendable(&affiliated("ArbitersSyndicate", -71_000, -2)), 0);
        assert_eq!(
            spendable(&affiliated("LibrarySyndicate", 57_854, 0)),
            57_854
        );
        assert_eq!(spendable(&affiliated("KahlSyndicate", 50, 5)), 50);
    }

    #[test]
    fn every_known_syndicate_is_listed_and_the_met_ones_lead_by_standing() {
        let met = [
            affiliated("RedVeilSyndicate", 298_500, 5),
            affiliated("EntratiSyndicate", 294_800, 5),
        ];

        let found = allegiances(&met, &[]);

        assert_eq!(found.len(), KNOWN.len());
        assert_eq!(found[0].tag, "RedVeilSyndicate");
        assert_eq!(found[0].standing, 58_500);
        assert_eq!(found[1].tag, "EntratiSyndicate");
        assert!(!of(&found, "QuillsSyndicate").met);
        assert_eq!(of(&found, "QuillsSyndicate").standing, 0);
    }

    #[test]
    fn the_factions_draw_on_one_pool_and_the_rest_on_their_own() {
        let allowances = [
            Allowance {
                pool: String::new(),
                left: 1_000,
            },
            Allowance {
                pool: "Quills".into(),
                left: 25_500,
            },
        ];

        let found = allegiances(&[], &allowances);

        assert_eq!(of(&found, "RedVeilSyndicate").left_today, Some(1_000));
        assert_eq!(of(&found, "SteelMeridianSyndicate").left_today, Some(1_000));
        assert_eq!(of(&found, "QuillsSyndicate").left_today, Some(25_500));
        assert_eq!(of(&found, "HexSyndicate").left_today, None);
    }
}
