/// One way to obtain a key of an item: its blueprint, a component's blueprint, or the item itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Way {
    /// A permanent vendor sells it for credits only.
    Credits { vendor: String, credits: u32 },
    /// A permanent vendor sells it for syndicate standing or another currency.
    Vendor {
        vendor: String,
        /// Currency key as the catalog spells it.
        currency: String,
        /// Currency name in the requested language, when the currency is an item.
        coin: Option<String>,
        cost: Option<u32>,
        /// Syndicate rank required for the offer.
        rank: Option<u32>,
        area: Option<String>,
    },
    /// Researched in a clan dojo lab.
    Lab { lab: String },
    /// Rewarded by relics; `vaulted` when none of those relics drops any more.
    Relics { vaulted: bool },
    /// Dropped at a location or by an enemy; `chance` is in tenths of a percent (387 = 38.7%).
    Drop {
        place: String,
        area: Option<String>,
        enemy: bool,
        chance: u32,
        /// Planet of the location; `None` when the star chart has none.
        planet: Option<String>,
        /// Mission type at the location, e.g. "Hive"; for a bounty, the bounty activity.
        mission: Option<String>,
        level_from: Option<u32>,
        level_to: Option<u32>,
        /// Planets the enemy appears on, the most frequent first.
        haunts: Vec<String>,
    },
    /// A rotating vendor sells it during its visits.
    Visit { vendor: String },
}

impl Way {
    /// Effort rank, from 0 (credits) to 6 (vaulted relics); lower is easier.
    pub fn effort(&self) -> u8 {
        match self {
            Way::Credits { .. } => 0,
            Way::Vendor { .. } => 1,
            Way::Lab { .. } => 2,
            Way::Relics { vaulted: false } => 3,
            Way::Drop { .. } => 4,
            Way::Visit { .. } => 5,
            Way::Relics { vaulted: true } => 6,
        }
    }

    /// Area the item is obtained in; `None` when the catalog lists none.
    pub fn area(&self) -> Option<&str> {
        match self {
            Way::Vendor { area, .. } | Way::Drop { area, .. } => area.as_deref(),
            _ => None,
        }
    }

    /// Drop chance that breaks ties between ways of equal effort (higher wins); 0 for non-drops.
    pub fn odds(&self) -> u32 {
        match self {
            Way::Drop { chance, .. } => *chance,
            _ => 0,
        }
    }
}

/// Lowest-effort way among `ways`, the likelier drop on a tie; `None` when `ways` is empty.
pub fn easiest(ways: impl IntoIterator<Item = Way>) -> Option<Way> {
    ways.into_iter().min_by(|one, other| {
        one.effort()
            .cmp(&other.effort())
            .then_with(|| other.odds().cmp(&one.odds()))
    })
}

/// Highest-effort way among an item's keys, since the item needs all of them; `None` when any
/// key has no known way.
pub fn hardest(keys: impl IntoIterator<Item = Option<Way>>) -> Option<Way> {
    let mut found: Option<Way> = None;
    for key in keys {
        let way = key?;
        if found
            .as_ref()
            .is_none_or(|held| way.effort() > held.effort())
        {
            found = Some(way);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop(place: &str, chance: u32) -> Way {
        Way::Drop {
            place: place.into(),
            area: None,
            enemy: false,
            chance,
            planet: None,
            mission: None,
            level_from: None,
            level_to: None,
            haunts: Vec::new(),
        }
    }

    fn credits() -> Way {
        Way::Credits {
            vendor: "Market".into(),
            credits: 25_000,
        }
    }

    #[test]
    fn a_key_sold_for_credits_is_got_that_way_whatever_else_drops_it() {
        assert_eq!(
            easiest([drop("Ceres/Ker", 900), credits()]),
            Some(credits())
        );
    }

    #[test]
    fn of_two_drops_the_likelier_one_is_taken() {
        assert_eq!(
            easiest([drop("Mars/War", 40), drop("Earth/Lith", 220)]),
            Some(drop("Earth/Lith", 220))
        );
    }

    #[test]
    fn an_item_takes_what_its_hardest_key_takes() {
        let lab = Way::Lab {
            lab: "Tenno".into(),
        };

        assert_eq!(
            hardest([Some(credits()), Some(drop("Saturn/Tethys", 387)), Some(lab)]),
            Some(drop("Saturn/Tethys", 387))
        );
    }

    #[test]
    fn a_key_with_no_way_leaves_the_item_unknown() {
        assert_eq!(hardest([Some(credits()), None]), None);
    }

    #[test]
    fn vaulted_relics_are_harder_than_anything_still_dropping() {
        let vaulted = Way::Relics { vaulted: true };

        assert!(vaulted.effort() > drop("Void/Mot", 100).effort());
        assert!(Way::Relics { vaulted: false }.effort() < drop("Void/Mot", 100).effort());
    }
}
