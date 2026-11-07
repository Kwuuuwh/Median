use std::ops::{BitOr, BitOrAssign};

/// Slice of application state that changes independently; one bit in `Changed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Settings,
    World,
    Quarantine,
    Catalog,
    Collector,
    Tasks,
    Account,
    Inventory,
    Market,
    Membership,
    Policy,
    Agent,
    Game,
}

impl Domain {
    /// Every domain, in bit order.
    pub const ALL: [Domain; 13] = [
        Domain::Settings,
        Domain::World,
        Domain::Quarantine,
        Domain::Catalog,
        Domain::Collector,
        Domain::Tasks,
        Domain::Account,
        Domain::Inventory,
        Domain::Market,
        Domain::Membership,
        Domain::Policy,
        Domain::Agent,
        Domain::Game,
    ];

    /// Bit mask of this domain; the bit index is the variant's declaration order.
    const fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

/// Set of domains an update changed, as a bit mask.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Changed(u32);

impl Changed {
    /// Empty set.
    pub const NOTHING: Self = Self(0);

    /// Set holding only `domain`.
    pub const fn of(domain: Domain) -> Self {
        Self(domain.bit())
    }

    /// Whether the set holds `domain`.
    pub const fn has(self, domain: Domain) -> bool {
        self.0 & domain.bit() != 0
    }

    /// Whether the set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Changed {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOrAssign for Changed {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

#[cfg(test)]
mod tests {
    use super::{Changed, Domain};

    #[test]
    fn nothing_holds_no_domain() {
        assert!(Changed::NOTHING.is_empty());
        for domain in Domain::ALL {
            assert!(!Changed::NOTHING.has(domain), "{domain:?} should be absent");
        }
    }

    #[test]
    fn a_domain_stands_for_itself_alone() {
        for domain in Domain::ALL {
            let changed = Changed::of(domain);
            assert!(!changed.is_empty());
            for other in Domain::ALL {
                assert_eq!(changed.has(other), other == domain, "{other:?}");
            }
        }
    }

    #[test]
    fn a_domain_takes_the_bit_of_its_place() {
        for (place, domain) in Domain::ALL.into_iter().enumerate() {
            assert_eq!(Changed::of(domain), Changed(1 << place), "{domain:?}");
        }
    }

    #[test]
    fn every_domain_fits_the_mask() {
        assert!(Domain::ALL.len() <= 32, "the mask holds 32 domains");
    }

    #[test]
    fn a_union_keeps_both_sides() {
        let mut changed = Changed::of(Domain::Account);
        changed |= Changed::of(Domain::Catalog);

        assert!(changed.has(Domain::Account));
        assert!(changed.has(Domain::Catalog));
        assert!(!changed.has(Domain::World));
    }

    #[test]
    fn a_union_with_nothing_changes_nothing() {
        let changed = Changed::of(Domain::World);

        assert_eq!(changed | Changed::NOTHING, changed);
        assert_eq!(Changed::NOTHING | changed, changed);
    }
}
