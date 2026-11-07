use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::effect::Effect;
use crate::mastery::{Intrinsic, Mission};
use crate::riven::Riven;
use crate::syndicate::{Affiliation, Allowance};
use crate::time::{Instant, seconds};

/// How the count of an inventory family is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counted {
    /// Each entry has a count field.
    Field,
    /// Each entry is one copy; copies are listed one by one.
    Copy,
}

/// Property that distinguishes copies of the same item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// A mod's rank, or a riven's rolled stats.
    Fingerprint,
    /// Number of stars installed in an Ayatan sculpture.
    Sockets,
}

/// Top-level key of the inventory dump that lists items the account owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub key: &'static str,
    pub counted: Counted,
    pub mark: Option<Mark>,
}

const fn counted(key: &'static str, counted: Counted) -> Family {
    Family {
        key,
        counted,
        mark: None,
    }
}

const fn marked(key: &'static str, counted: Counted, mark: Mark) -> Family {
    Family {
        key,
        counted,
        mark: Some(mark),
    }
}

/// Every key of the inventory dump that lists owned items. The rest of the dump is progression,
/// settings and history.
pub const FAMILIES: &[Family] = &[
    counted("MiscItems", Counted::Field),
    counted("Recipes", Counted::Field),
    counted("RawUpgrades", Counted::Field),
    counted("Consumables", Counted::Field),
    counted("ShipDecorations", Counted::Field),
    counted("EmailItems", Counted::Field),
    counted("CrewShipRawSalvage", Counted::Field),
    counted("CrewShipAmmo", Counted::Field),
    counted("LevelKeys", Counted::Field),
    marked("FusionTreasures", Counted::Field, Mark::Sockets),
    marked("Upgrades", Counted::Copy, Mark::Fingerprint),
    marked("CrewShipWeaponSkins", Counted::Copy, Mark::Fingerprint),
    counted("FlavourItems", Counted::Copy),
    counted("WeaponSkins", Counted::Copy),
    counted("Suits", Counted::Copy),
    counted("LongGuns", Counted::Copy),
    counted("Pistols", Counted::Copy),
    counted("Melee", Counted::Copy),
    counted("Sentinels", Counted::Copy),
    counted("SentinelWeapons", Counted::Copy),
    counted("SpaceSuits", Counted::Copy),
    counted("SpaceGuns", Counted::Copy),
    counted("SpaceMelee", Counted::Copy),
    counted("OperatorAmps", Counted::Copy),
    counted("OperatorSuits", Counted::Copy),
    counted("KubrowPets", Counted::Copy),
    counted("DataKnives", Counted::Copy),
    counted("Motorcycles", Counted::Copy),
    counted("Horses", Counted::Copy),
    counted("Scoops", Counted::Copy),
    counted("Ships", Counted::Copy),
    counted("DrifterMelee", Counted::Copy),
    counted("CrewShips", Counted::Copy),
    counted("CrewShipHarnesses", Counted::Copy),
    counted("SpecialItems", Counted::Copy),
    counted("CrewMembers", Counted::Copy),
];

/// Family of a top-level dump key; `None` for a key that lists no owned items.
pub fn family(key: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|family| family.key == key)
}

/// Variant key that distinguishes copies of the same item; empty when the copies are
/// interchangeable.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variant(pub String);

impl Variant {
    /// Whether the variant is empty, i.e. the copies are interchangeable.
    pub fn is_plain(&self) -> bool {
        self.0.is_empty()
    }
}

/// Decoded meaning of a variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detail {
    /// A mod at this rank.
    Rank(u8),
    /// An Ayatan sculpture with this many stars installed.
    Stars(u8),
    /// A riven at this rank, with its stats once revealed; `None` while veiled.
    Riven { rank: u8, revealed: Option<Riven> },
}

/// One item the account owns, with its variant and count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    /// Game item key.
    pub item: String,
    pub variant: Variant,
    /// Decoded variant; `None` when this build cannot decode it.
    pub detail: Option<Detail>,
    pub count: u32,
}

/// Account an inventory dump belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Account creation time in milliseconds; unlike the name, it never changes.
    pub created: i64,
    /// Current account name.
    pub name: String,
}

/// BLAKE3 digest of the account's holdings and purse; the rest of the dump does not affect it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digest([u8; 32]);

impl Digest {
    /// Digest from its stored bytes.
    pub fn of(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Digest bytes as stored on disk.
    pub fn bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Digest as lowercase hex, as the window shows it.
    pub fn hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// First 12 hex characters of the digest, enough to tell two apart in a log line.
    pub fn short(&self) -> String {
        self.hex()[..12].to_string()
    }
}

/// Builds holdings from dump entries, merging copies of the same item and variant.
#[derive(Debug, Default)]
pub struct Tally(Vec<Holding>);

impl Tally {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a holding; its count is added to an earlier holding of the same item and variant.
    pub fn add(&mut self, held: Holding) {
        match self
            .0
            .iter_mut()
            .find(|kept| kept.item == held.item && kept.variant == held.variant)
        {
            Some(kept) => kept.count = kept.count.saturating_add(held.count),
            None => self.0.push(held),
        }
    }

    /// Subtracts `count` copies from the holding of `item` without a variant, not below 0.
    pub fn commit(&mut self, item: &str, count: u32) {
        if let Some(kept) = self
            .0
            .iter_mut()
            .find(|kept| kept.item == item && kept.variant.is_plain())
        {
            kept.count = kept.count.saturating_sub(count);
        }
    }

    /// Holdings sorted by item and variant, the order the digest requires; holdings with no
    /// copies left are dropped.
    pub fn finish(mut self) -> Vec<Holding> {
        self.0.retain(|held| held.count > 0);
        self.0
            .sort_by(|a, b| (&a.item, &a.variant).cmp(&(&b.item, &b.variant)));
        self.0
    }
}

/// Holdings of `after` whose count differs from `before`, plus holdings missing from `after`
/// with a count of 0.
pub fn changes(before: &[Holding], after: &[Holding]) -> Vec<Holding> {
    let held: BTreeMap<(&str, &Variant), u32> = before
        .iter()
        .map(|held| ((held.item.as_str(), &held.variant), held.count))
        .collect();

    let mut lines: Vec<Holding> = after
        .iter()
        .filter(|now| held.get(&(now.item.as_str(), &now.variant)) != Some(&now.count))
        .cloned()
        .collect();

    let present: BTreeSet<(&str, &Variant)> = after
        .iter()
        .map(|now| (now.item.as_str(), &now.variant))
        .collect();

    lines.extend(
        before
            .iter()
            .filter(|gone| !present.contains(&(gone.item.as_str(), &gone.variant)))
            .map(|gone| Holding {
                count: 0,
                ..gone.clone()
            }),
    );

    lines
}

/// Affinity the game recorded for one item; mastery is derived from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mastered {
    pub item: String,
    pub xp: u64,
}

/// Copy of an item installed on gear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equipped {
    pub item: String,
    pub variant: Variant,
}

/// Account currencies and daily trades left, besides its holdings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Purse {
    pub platinum: Option<u32>,
    pub credits: Option<u64>,
    pub endo: Option<u64>,
    /// Daily trades left today.
    pub trades_left: Option<u32>,
}

/// One item being built in the foundry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crafting {
    pub id: String,
    /// Blueprint used for the build.
    pub item: String,
    pub ready: Instant,
}

/// When the account got its first copy of an item from a `Counted::Copy` family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acquired {
    pub item: String,
    pub at: Instant,
}

/// Highest Forma count on one held copy of an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formed {
    pub item: String,
    pub forma: u32,
}

/// Account progression from the inventory dump.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progression {
    /// Mastery rank; `None` when the dump does not include it.
    pub rank: Option<u32>,
    /// When the next mastery rank test becomes available, as set by the last attempt.
    pub next_test: Option<Instant>,
    pub missions: Vec<Mission>,
    pub intrinsics: Vec<Intrinsic>,
    /// Foundry builds; `None` when the dump has no foundry data, empty when nothing is being
    /// built.
    pub crafting: Option<Vec<Crafting>>,
    /// Acquisition time of the oldest copy of each `Counted::Copy` item, one entry per item.
    pub acquired: Vec<Acquired>,
    /// Items with Forma on a held copy, one entry per item.
    pub formed: Vec<Formed>,
    /// Standing and rank with each syndicate the account is affiliated with.
    pub affiliations: Vec<Affiliation>,
    /// Standing still available to earn today, per pool, as the last dump of this run reported.
    pub allowances: Vec<Allowance>,
}

/// One parsed inventory dump: the account and what it holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inventory {
    /// `None` when the dump has nothing that identifies the account.
    pub profile: Option<Profile>,
    pub holdings: Vec<Holding>,
    /// Marker the game changes on each inventory sync.
    pub synced: Option<String>,
    pub purse: Purse,
    /// Affinity per item, for every item the account has ever levelled.
    pub mastery: Vec<Mastered>,
    /// Copies installed on gear, in any loadout.
    pub equipped: Vec<Equipped>,
    pub progression: Progression,
}

impl Inventory {
    /// Every item key the account holds, sorted and deduplicated.
    pub fn items(&self) -> Vec<String> {
        let mut items: Vec<String> = self
            .holdings
            .iter()
            .map(|holding| holding.item.clone())
            .collect();
        items.sort();
        items.dedup();
        items
    }

    /// BLAKE3 digest of the holdings and of the purse's platinum, credits and trades left.
    pub fn digest(&self) -> Digest {
        let mut hasher = blake3::Hasher::new();
        for held in &self.holdings {
            hasher.update(held.item.as_bytes());
            hasher.update(&[0]);
            hasher.update(held.variant.0.as_bytes());
            hasher.update(&[0]);
            hasher.update(&held.count.to_le_bytes());
            hasher.update(&[0]);
        }
        for value in [
            self.purse.platinum.map(u64::from),
            self.purse.credits,
            self.purse.trades_left.map(u64::from),
        ] {
            hasher.update(&[u8::from(value.is_some())]);
            hasher.update(&value.unwrap_or_default().to_le_bytes());
        }
        Digest(*hasher.finalize().as_bytes())
    }
}

/// State of the account the collector reports on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    /// `None` until a dump identifies the account.
    pub profile: Option<Profile>,
    /// Number of holdings in the last readable dump.
    pub positions: u32,
    /// Dumps read since the application started.
    pub read: u32,
    /// Of those, dumps whose digest differed from the previous one.
    pub distinct: u32,
    /// Dumps received that could not be parsed.
    pub unreadable: u32,
    /// Dumps parsed without anything that identifies the account.
    pub unidentified: u32,
    pub digest: Option<Digest>,
    /// Sync marker of the last readable dump.
    pub synced: Option<String>,
    /// When a dump last differed from the previous one. The data is as old as this; an
    /// identical dump does not update it.
    pub moved: Option<Instant>,
}

/// Inventory dump event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountEvent {
    /// A dump was parsed.
    Read(Arc<Inventory>),
    /// A dump was parsed but has nothing that identifies the account.
    Unidentified,
    /// A dump was received and could not be parsed.
    Unreadable,
}

/// Account state after `event`; when the holdings changed, also returns the new inventory and a
/// `StoreSnapshot` effect to save it. A dump of another account (different creation time)
/// resets the state.
pub(crate) fn update(
    account: &Account,
    event: AccountEvent,
) -> (Account, Option<Arc<Inventory>>, Vec<Effect>) {
    let mut next = account.clone();
    let mut held = None;
    let mut effects = Vec::new();

    match event {
        AccountEvent::Read(inventory) => {
            let Some(profile) = inventory.profile.clone() else {
                next.unidentified += 1;
                return (next, held, effects);
            };

            if next
                .profile
                .as_ref()
                .is_some_and(|known| known.created != profile.created)
            {
                next = Account::default();
            }
            next.profile = Some(profile);

            let digest = inventory.digest();
            if next.digest != Some(digest) {
                next.distinct += 1;
                next.moved = Some(seconds());
                effects.push(Effect::StoreSnapshot(Arc::clone(&inventory)));
                held = Some(Arc::clone(&inventory));
            }
            next.positions = inventory.holdings.len() as u32;
            next.read += 1;
            next.digest = Some(digest);
            next.synced = inventory.synced.clone();
        }
        AccountEvent::Unidentified => next.unidentified += 1,
        AccountEvent::Unreadable => next.unreadable += 1,
    }

    (next, held, effects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(holdings: Vec<Holding>) -> Inventory {
        Inventory {
            profile: None,
            holdings,
            synced: None,
            purse: Purse::default(),
            ..Inventory::default()
        }
    }

    fn held(item: &str, count: u32) -> Holding {
        Holding {
            item: item.into(),
            variant: Variant::default(),
            detail: None,
            count,
        }
    }

    fn owned(created: i64, holdings: Vec<Holding>) -> Inventory {
        Inventory {
            profile: Some(Profile {
                created,
                name: "Tenno".into(),
            }),
            holdings,
            synced: None,
            purse: Purse::default(),
            ..Inventory::default()
        }
    }

    fn read(inventory: Inventory) -> AccountEvent {
        AccountEvent::Read(Arc::new(inventory))
    }

    #[test]
    fn every_family_is_named_once() {
        for family in FAMILIES {
            let named = FAMILIES.iter().filter(|other| other.key == family.key);
            assert_eq!(named.count(), 1, "{} is named twice", family.key);
        }
    }

    #[test]
    fn a_key_the_document_holds_nothing_by_is_not_a_family() {
        for key in ["ChallengeProgress", "Missions", "DailyFocus", "QuestKeys"] {
            assert!(family(key).is_none(), "{key} was taken for a family");
        }
    }

    #[test]
    fn copies_of_one_item_add_up() {
        let mut tally = Tally::new();
        tally.add(held("Excalibur", 1));
        tally.add(held("Excalibur", 1));

        assert_eq!(tally.finish(), vec![held("Excalibur", 2)]);
    }

    #[test]
    fn what_tells_copies_apart_keeps_them_apart() {
        let mut tally = Tally::new();
        for rolled in ["rolled once", "rolled twice"] {
            tally.add(Holding {
                item: "Riven".into(),
                variant: Variant(rolled.into()),
                detail: None,
                count: 1,
            });
        }

        assert_eq!(tally.finish().len(), 2);
    }

    #[test]
    fn the_order_entries_arrived_in_does_not_reach_the_fingerprint() {
        let mut one = Tally::new();
        one.add(held("Rubedo", 24928));
        one.add(held("Ferrite", 12));

        let mut other = Tally::new();
        other.add(held("Ferrite", 12));
        other.add(held("Rubedo", 24928));

        assert_eq!(
            inventory(one.finish()).digest(),
            inventory(other.finish()).digest()
        );
    }

    #[test]
    fn a_count_that_moved_moves_the_fingerprint() {
        let before = inventory(vec![held("Rubedo", 24928)]);
        let after = inventory(vec![held("Rubedo", 24929)]);

        assert_ne!(before.digest(), after.digest());
    }

    #[test]
    fn the_same_holdings_fingerprint_alike() {
        let before = inventory(vec![held("Rubedo", 24928), held("Ferrite", 12)]);
        let after = inventory(vec![held("Rubedo", 24928), held("Ferrite", 12)]);

        assert_eq!(before.digest(), after.digest());
    }

    #[test]
    fn a_dump_that_changed_nothing_is_read_but_not_counted_as_distinct() {
        let holdings = vec![held("Rubedo", 24928)];
        let (first, _, _) = update(&Account::default(), read(owned(1, holdings.clone())));

        let (second, _, _) = update(&first, read(owned(1, holdings)));

        assert_eq!(second.read, 2);
        assert_eq!(second.distinct, 1);
    }

    #[test]
    fn a_dump_that_changed_something_counts_as_distinct() {
        let (first, _, _) = update(&Account::default(), read(owned(1, vec![])));

        let (second, _, _) = update(&first, read(owned(1, vec![held("Rubedo", 1)])));

        assert_eq!(second.distinct, 2);
    }

    #[test]
    fn a_dump_that_could_not_be_read_leaves_the_holdings_alone() {
        let holdings = vec![held("Rubedo", 24928)];
        let digest = inventory(holdings.clone()).digest();
        let (first, _, _) = update(&Account::default(), read(owned(1, holdings)));

        let (second, held, _) = update(&first, AccountEvent::Unreadable);

        assert_eq!(second.unreadable, 1);
        assert_eq!(second.digest, Some(digest));
        assert_eq!(second.read, 1);
        assert!(held.is_none());
    }

    #[test]
    fn holdings_that_moved_are_asked_to_be_kept_and_handed_over() {
        let reading = owned(1, vec![held("Rubedo", 24928)]);

        let (_, held, effects) = update(&Account::default(), read(reading.clone()));

        assert_eq!(
            effects,
            vec![Effect::StoreSnapshot(Arc::new(reading.clone()))]
        );
        assert_eq!(held, Some(Arc::new(reading)));
    }

    #[test]
    fn holdings_that_stayed_put_are_neither_kept_twice_nor_handed_over_again() {
        let holdings = vec![held("Rubedo", 24928)];
        let (first, _, _) = update(&Account::default(), read(owned(1, holdings.clone())));

        let (_, held, effects) = update(&first, read(owned(1, holdings)));

        assert!(effects.is_empty());
        assert!(held.is_none());
    }

    #[test]
    fn a_dump_that_says_nobody_moves_only_its_own_counter() {
        let (first, _, _) = update(&Account::default(), read(owned(1, vec![held("R", 1)])));

        let (second, held, effects) = update(&first, read(inventory(vec![held("R", 2)])));

        assert_eq!(second.unidentified, 1);
        assert_eq!(second.read, first.read);
        assert_eq!(second.positions, first.positions);
        assert_eq!(second.digest, first.digest);
        assert_eq!(second.profile, first.profile);
        assert!(held.is_none());
        assert!(effects.is_empty());
    }

    #[test]
    fn a_dump_nobody_could_be_named_for_is_counted_on_its_own() {
        let (next, held, effects) = update(&Account::default(), AccountEvent::Unidentified);

        assert_eq!(next.unidentified, 1);
        assert_eq!(next.read, 0);
        assert!(held.is_none());
        assert!(effects.is_empty());
    }

    #[test]
    fn another_account_starts_the_counters_over() {
        let (first, _, _) = update(&Account::default(), read(owned(1, vec![held("R", 1)])));
        let (first, _, _) = update(&first, read(owned(1, vec![held("R", 2)])));

        let (second, _, _) = update(&first, read(owned(2, vec![held("R", 9)])));

        assert_eq!(second.read, 1);
        assert_eq!(second.distinct, 1);
        assert_eq!(second.profile.map(|profile| profile.created), Some(2));
    }

    #[test]
    fn a_count_that_moved_is_a_line() {
        let before = vec![held("Rubedo", 24928), held("Ferrite", 12)];
        let after = vec![held("Rubedo", 24929), held("Ferrite", 12)];

        assert_eq!(changes(&before, &after), vec![held("Rubedo", 24929)]);
    }

    #[test]
    fn holdings_that_stayed_put_are_not_lines() {
        let holdings = vec![held("Rubedo", 24928)];

        assert!(changes(&holdings, &holdings).is_empty());
    }

    #[test]
    fn what_vanished_is_written_as_none_left() {
        let before = vec![held("Rubedo", 1), held("Ferrite", 12)];
        let after = vec![held("Ferrite", 12)];

        assert_eq!(changes(&before, &after), vec![held("Rubedo", 0)]);
    }

    #[test]
    fn a_copy_told_apart_moves_on_its_own() {
        let ranked = |rank: &str, count| Holding {
            item: "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod".into(),
            variant: Variant(rank.into()),
            detail: None,
            count,
        };
        let before = vec![ranked(r#"{"lvl":0}"#, 1), ranked(r#"{"lvl":5}"#, 1)];
        let after = vec![ranked(r#"{"lvl":0}"#, 1), ranked(r#"{"lvl":5}"#, 2)];

        assert_eq!(changes(&before, &after), vec![ranked(r#"{"lvl":5}"#, 2)]);
    }

    #[test]
    fn what_a_variant_says_stays_out_of_the_fingerprint() {
        let ranked = |detail| Holding {
            item: "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod".into(),
            variant: Variant(r#"{"lvl":2}"#.into()),
            detail,
            count: 1,
        };

        let told = inventory(vec![ranked(Some(Detail::Rank(2)))]);
        let mute = inventory(vec![ranked(None)]);

        assert_eq!(told.digest(), mute.digest());
    }

    #[test]
    fn a_reading_that_differs_only_in_platinum_has_another_digest() {
        let before = inventory(vec![held("/Lotus/Types/Items/MiscItems/Rubedo", 1)]);
        let after = Inventory {
            purse: Purse {
                platinum: Some(340),
                ..Purse::default()
            },
            ..before.clone()
        };

        assert_ne!(before.digest(), after.digest());
    }
}
