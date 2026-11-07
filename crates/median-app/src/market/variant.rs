use median_core::Detail;
use median_market::{ClosedBucket, LiveBucket, Mine, Order, Selector};

/// Variant of an item by the dimensions warframe.market prices separately: rank, subtype and
/// amber and cyan stars.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variant {
    pub rank: Option<u8>,
    pub subtype: Option<String>,
    pub amber_stars: Option<u8>,
    pub cyan_stars: Option<u8>,
}

/// Dimension names as warframe.market writes them.
const RANK: &str = "mod_rank";
const SUBTYPE: &str = "subtype";
const AMBER: &str = "amber_stars";
const CYAN: &str = "cyan_stars";

impl Variant {
    pub fn of_closed(bucket: &ClosedBucket) -> Self {
        Self {
            rank: bucket.mod_rank,
            subtype: bucket.subtype.clone(),
            amber_stars: bucket.amber_stars,
            cyan_stars: bucket.cyan_stars,
        }
    }

    pub fn of_live(bucket: &LiveBucket) -> Self {
        Self {
            rank: bucket.mod_rank,
            subtype: bucket.subtype.clone(),
            amber_stars: bucket.amber_stars,
            cyan_stars: bucket.cyan_stars,
        }
    }

    /// Market request filter for this variant alone, also used to pick its rows out of an
    /// item's book. Stars have no request parameter, so an item with stars is requested whole
    /// and filtered here.
    pub fn selector(&self) -> Selector {
        match (self.rank, &self.subtype) {
            (Some(rank), Some(subtype)) => Selector::Both {
                rank,
                subtype: subtype.clone(),
            },
            (Some(rank), None) => Selector::Rank(rank),
            (None, Some(subtype)) => Selector::Subtype(subtype.clone()),
            (None, None) => Selector::Whole,
        }
    }

    pub fn of_order(order: &Order) -> Self {
        Self {
            rank: order.rank,
            subtype: order.subtype.clone(),
            amber_stars: order.amber_stars,
            cyan_stars: order.cyan_stars,
        }
    }

    /// The variant of one of the account's own orders.
    pub fn of_mine(order: &Mine) -> Self {
        Self {
            rank: order.rank,
            subtype: order.subtype.clone(),
            amber_stars: order.amber_stars,
            cyan_stars: order.cyan_stars,
        }
    }

    /// The variant a `key` describes. Unknown `name=value` pairs are skipped, so a key written
    /// by a later build still parses.
    pub fn parse(key: &str) -> Self {
        let mut variant = Self::default();
        for pair in key.split(';').filter(|pair| !pair.is_empty()) {
            let Some((name, value)) = pair.split_once('=') else {
                continue;
            };
            match name {
                RANK => variant.rank = value.parse().ok(),
                SUBTYPE => variant.subtype = Some(value.to_owned()),
                AMBER => variant.amber_stars = value.parse().ok(),
                CYAN => variant.cyan_stars = value.parse().ok(),
                _ => {}
            }
        }
        variant
    }

    /// Key of the variant: `name=value` pairs joined by `;` in a fixed order, for example
    /// `mod_rank=10`; empty for an item without variants. A dimension the market adds later
    /// extends the key of new rows only; old rows keep their key and stay valid.
    pub fn key(&self) -> String {
        let mut pairs = Vec::new();
        if let Some(rank) = self.rank {
            pairs.push(format!("{RANK}={rank}"));
        }
        if let Some(subtype) = &self.subtype {
            pairs.push(format!("{SUBTYPE}={subtype}"));
        }
        if let Some(amber) = self.amber_stars {
            pairs.push(format!("{AMBER}={amber}"));
        }
        if let Some(cyan) = self.cyan_stars {
            pairs.push(format!("{CYAN}={cyan}"));
        }
        pairs.join(";")
    }

    /// The variant of a copy as the game describes it; `None` when the game's detail is not a
    /// rank, since the market distinguishes such copies differently.
    pub fn of_detail(detail: Option<&Detail>) -> Option<Self> {
        match detail {
            None => Some(Self::default()),
            Some(Detail::Rank(rank)) => Some(Self {
                rank: Some(*rank),
                ..Self::default()
            }),
            Some(_) => None,
        }
    }

    /// Whether the order is of this variant.
    pub fn holds(&self, order: &Order) -> bool {
        *self == Self::of_order(order)
    }

    /// Whether a copy read from the game belongs to this variant. A missing rank counts as
    /// rank 0, and a dimension the game does not report matches any value.
    pub fn covers(&self, copy: &Self) -> bool {
        self.rank.unwrap_or(0) == copy.rank.unwrap_or(0)
            && (copy.subtype.is_none() || copy.subtype == self.subtype)
            && (copy.amber_stars.is_none() || copy.amber_stars == self.amber_stars)
            && (copy.cyan_stars.is_none() || copy.cyan_stars == self.cyan_stars)
    }
}

/// Variants of an item to request from the market: rank 0 and max rank (when the max rank is
/// above 0) crossed with each subtype it declares; the whole item when it has neither.
pub fn variants(max_rank: Option<u8>, subtypes: &[String]) -> Vec<Variant> {
    let ranks: Vec<Option<u8>> = match max_rank {
        Some(max) if max > 0 => vec![Some(0), Some(max)],
        _ => vec![None],
    };
    if subtypes.is_empty() {
        return ranks
            .into_iter()
            .map(|rank| Variant {
                rank,
                ..Variant::default()
            })
            .collect();
    }

    let mut asked = Vec::with_capacity(ranks.len() * subtypes.len());
    for subtype in subtypes {
        for rank in &ranks {
            asked.push(Variant {
                rank: *rank,
                subtype: Some(subtype.clone()),
                ..Variant::default()
            });
        }
    }
    asked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_reads_back_into_the_variant_it_was_written_from() {
        for variant in [
            Variant::default(),
            Variant {
                rank: Some(5),
                ..Variant::default()
            },
            Variant {
                subtype: Some("radiant".into()),
                ..Variant::default()
            },
            Variant {
                amber_stars: Some(0),
                cyan_stars: Some(2),
                ..Variant::default()
            },
        ] {
            assert_eq!(Variant::parse(&variant.key()), variant);
        }
    }

    #[test]
    fn a_pair_this_build_does_not_know_is_stepped_over() {
        assert_eq!(
            Variant::parse("mod_rank=3;shiny=yes"),
            Variant {
                rank: Some(3),
                ..Variant::default()
            }
        );
    }

    fn closed(variant: &str) -> ClosedBucket {
        let json = format!(
            r#"{{"datetime":"2026-08-15T00:00:00.000+00:00","volume":7,"min_price":1.0,
                "max_price":2.0,"avg_price":1.5,"median":1.5{variant}}}"#
        );
        serde_json::from_str(&json).expect("bucket parses")
    }

    #[test]
    fn each_dimension_reaches_the_key() {
        assert_eq!(
            Variant::of_closed(&closed(r#","mod_rank":5"#)).key(),
            "mod_rank=5"
        );
        assert_eq!(
            Variant::of_closed(&closed(r#","subtype":"radiant""#)).key(),
            "subtype=radiant"
        );
        assert_eq!(
            Variant::of_closed(&closed(r#","amber_stars":2,"cyan_stars":2"#)).key(),
            "amber_stars=2;cyan_stars=2"
        );
    }

    #[test]
    fn dimensions_that_meet_share_one_key() {
        let variant = Variant {
            rank: Some(10),
            subtype: Some("regular".into()),
            ..Variant::default()
        };
        assert_eq!(variant.key(), "mod_rank=10;subtype=regular");
    }

    #[test]
    fn two_variants_of_one_item_do_not_share_a_key() {
        let bare = Variant::of_closed(&closed(r#","mod_rank":0"#));
        let maxed = Variant::of_closed(&closed(r#","mod_rank":10"#));
        assert_ne!(bare.key(), maxed.key());
    }

    #[test]
    fn an_item_without_variants_is_asked_about_whole() {
        assert_eq!(variants(None, &[]), vec![Variant::default()]);
        assert_eq!(variants(Some(0), &[]), vec![Variant::default()]);
        assert_eq!(Variant::of_closed(&closed("")).key(), "");
    }

    #[test]
    fn a_ranked_item_is_asked_about_at_both_ends() {
        let asked = variants(Some(10), &[]);
        assert_eq!(asked.len(), 2);
        assert_eq!(asked[0].key(), "mod_rank=0");
        assert_eq!(asked[1].key(), "mod_rank=10");
    }

    #[test]
    fn an_item_with_subtypes_is_asked_about_each_of_them() {
        let declared = ["intact".to_string(), "radiant".to_string()];
        let asked = variants(Some(0), &declared);
        assert_eq!(asked.len(), 2);
        assert_eq!(asked[0].key(), "subtype=intact");
        assert_eq!(asked[1].key(), "subtype=radiant");
    }

    #[test]
    fn an_item_that_has_both_ranks_and_subtypes_is_asked_about_by_both() {
        let declared = ["regular".to_string(), "atragraph".to_string()];

        let asked = variants(Some(10), &declared);

        assert_eq!(asked.len(), 4);
        assert_eq!(asked[0].key(), "mod_rank=0;subtype=regular");
        assert_eq!(asked[1].key(), "mod_rank=10;subtype=regular");
        assert_eq!(asked[3].key(), "mod_rank=10;subtype=atragraph");
    }

    #[test]
    fn a_variant_tells_its_own_rows_from_the_rest_of_the_book() {
        assert_eq!(Variant::default().selector(), Selector::Whole);
        assert_eq!(
            Variant {
                rank: Some(10),
                ..Variant::default()
            }
            .selector(),
            Selector::Rank(10)
        );
    }

    #[test]
    fn a_rank_the_game_read_becomes_the_market_variant() {
        assert_eq!(
            Variant::of_detail(Some(&Detail::Rank(5))),
            Some(Variant {
                rank: Some(5),
                ..Variant::default()
            })
        );
        assert_eq!(Variant::of_detail(None), Some(Variant::default()));
    }
}
