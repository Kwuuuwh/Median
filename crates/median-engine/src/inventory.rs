use std::collections::HashMap;
use std::fmt;

use median_core::{
    Acquired, Affiliation, Allowance, Counted, Crafting, Detail, Equipped, Formed, Holding,
    Instant, Intrinsic, Inventory, Mark, Mastered, Mission, Polarity, Profile, Progression, Purse,
    Riven, RivenStat, Tally, Variant, family,
};
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// Inventory key of the account's creation date.
const CREATED: &str = "Created";

/// Inventory key of the game's inventory sync marker.
const SYNCED: &str = "LastInventorySync";

/// Key of the document when the game nests it in one more object.
const WRAPPED: &str = "InventoryJson";

/// Inventory key of the builds in the foundry.
const BUILDING: &str = "PendingRecipes";

/// Inventory key of the affinity the account has earned per item.
const LEVELLED: &str = "XPInfo";

/// Inventory key of the account's mastery rank.
const RANK: &str = "PlayerLevel";

/// Inventory key of the time the next mastery test unlocks.
const NEXT_TEST: &str = "TrainingDate";

/// Inventory key of the nodes the account has completed.
const MISSIONS: &str = "Missions";

/// Inventory key of the account's skills.
const SKILLS: &str = "PlayerSkills";

/// Prefix of the skills that hold an intrinsic's rank rather than a pool of points.
const INTRINSIC: &str = "LPS_";

/// Inventory key of the syndicates the account has met.
const AFFILIATIONS: &str = "Affiliations";

/// Key prefix of the standing still earnable today, one key per pool.
const DAILY: &str = "DailyAffiliation";

/// Inventory key of the trades left today.
const LEFT: &str = "TradesRemaining";

/// Inventory key of the account's platinum.
const PLATINUM: &str = "PremiumCredits";

/// Inventory key of the account's credits.
const CREDITS: &str = "RegularCredits";

/// Inventory key of the account's endo.
const ENDO: &str = "FusionPoints";

/// Key of the sync marker's value.
const OID: &str = "$oid";

/// Maximum distance in bytes from the marker key to its value, allowing for the escaping the
/// dump adds.
const WITHIN: usize = 64;

/// Length of the marker the game writes, in hex digits.
const MARKER: usize = 24;

/// The sync marker a dump carries, found without parsing the document around it; `None` unless
/// the value is 24 hex digits. Escaping cannot change a hexadecimal value.
pub fn marker(payload: &str) -> Option<&str> {
    let key = payload.find(SYNCED)? + SYNCED.len();
    let rest = &payload[key..];

    let value = rest.find(OID)?;
    if value > WITHIN {
        return None;
    }

    let tail = &rest[value + OID.len()..];
    let start = tail.find(|character: char| character.is_ascii_hexdigit())?;
    let run = tail[start..]
        .chars()
        .take_while(char::is_ascii_hexdigit)
        .count();

    (run == MARKER).then(|| &tail[start..start + run])
}

/// Why an inventory dump could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InventoryError {
    #[error("the envelope is not one median wrote: {0}")]
    MalformedEnvelope(String),
    #[error("the inventory is not a document: {0}")]
    MalformedInventory(String),
    #[error("the inventory holds nothing, so it is not one this build understands")]
    NothingHeld,
}

/// A parsed inventory dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub inventory: Inventory,
    /// Every top-level key the dump carried, whether or not it holds anything.
    pub keys: Vec<String>,
}

/// Parses the inventory in a collector envelope's payload; `Ok(None)` when the payload has no
/// inventory, `Err(NothingHeld)` when the inventory has no holdings.
pub fn read(payload: &str) -> Result<Option<Reading>, InventoryError> {
    let carried: Payload = serde_json::from_str(payload)
        .map_err(|e| InventoryError::MalformedEnvelope(e.to_string()))?;

    let (feeds, name) = match carried.info {
        Some(info) => info.feeds(),
        None => return Ok(None),
    };

    let Some(document) = feeds else {
        return Ok(None);
    };

    let read: Document = serde_json::from_str(&document)
        .map_err(|e| InventoryError::MalformedInventory(e.to_string()))?;

    let reading = read.into_reading(name);
    if reading.inventory.holdings.is_empty() {
        return Err(InventoryError::NothingHeld);
    }
    Ok(Some(reading))
}

#[derive(Deserialize)]
struct Payload {
    info: Option<Info>,
}

/// Both shapes of a collector feed: a periodic poll nests everything under `res`, a pushed
/// update does not.
#[derive(Deserialize)]
struct Info {
    #[serde(default)]
    res: Option<Feeds>,
    #[serde(default)]
    match_info: Option<MatchInfo>,
    #[serde(default)]
    game_info: Option<GameInfo>,
}

impl Info {
    /// The inventory document and the account name, from either shape.
    fn feeds(self) -> (Option<String>, Option<String>) {
        let (match_info, game_info) = match self.res {
            Some(res) => (res.match_info, res.game_info),
            None => (self.match_info, self.game_info),
        };
        (
            match_info.and_then(|feed| feed.inventory),
            game_info.and_then(|feed| feed.username),
        )
    }
}

#[derive(Deserialize)]
struct Feeds {
    #[serde(default)]
    match_info: Option<MatchInfo>,
    #[serde(default)]
    game_info: Option<GameInfo>,
}

#[derive(Deserialize)]
struct MatchInfo {
    /// The whole inventory, as a JSON document in a string.
    #[serde(default)]
    inventory: Option<String>,
}

#[derive(Deserialize)]
struct GameInfo {
    #[serde(default)]
    username: Option<String>,
}

/// One entry of an inventory family; which optional fields are set depends on the family.
#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "ItemType")]
    item: String,
    #[serde(rename = "ItemCount")]
    count: Option<i64>,
    #[serde(rename = "UpgradeFingerprint")]
    fingerprint: Option<String>,
    #[serde(rename = "Sockets")]
    sockets: Option<i64>,
    /// Id that gear configs use for this copy; only individually listed copies have one.
    #[serde(rename = "ItemId")]
    id: Option<Synced>,
    /// Loadouts of a piece of gear, each listing what is installed on it.
    #[serde(rename = "Configs")]
    configs: Option<Vec<Config>>,
    /// Forma applied to the copy; `None` when none is.
    #[serde(rename = "Polarized")]
    polarized: Option<i64>,
}

/// One loadout of a piece of gear: what is in its upgrade slots. A slot holds either the id of
/// an identified copy or the item key, for a copy taken from an unranked stack.
#[derive(Default)]
struct Config(Vec<String>);

/// The game writes an unused loadout as an empty array instead of an object, so anything but an
/// object parses as an empty loadout.
impl<'de> Deserialize<'de> for Config {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Slots)
    }
}

struct Slots;

impl<'de> Visitor<'de> for Slots {
    type Value = Config;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a loadout of a piece of gear")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Config, M::Error> {
        let mut slots = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "Upgrades" => {
                    slots = map
                        .next_value::<Vec<Option<String>>>()?
                        .into_iter()
                        .flatten()
                        .collect();
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        Ok(Config(slots))
    }

    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Config, S::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Config::default())
    }
}

/// One item's affinity, as the game counts it towards mastery.
#[derive(Deserialize)]
struct Levelled {
    #[serde(rename = "ItemType")]
    item: String,
    #[serde(rename = "XP")]
    xp: Option<i64>,
}

/// One build in the foundry; the item is the blueprint it consumed.
#[derive(Deserialize)]
struct Building {
    #[serde(rename = "ItemType")]
    item: String,
    #[serde(rename = "ItemId")]
    id: Option<Synced>,
    #[serde(rename = "CompletionDate")]
    ready: Option<Dated>,
}

/// One syndicate the account has met.
#[derive(Deserialize)]
struct Affiliated {
    #[serde(rename = "Tag")]
    tag: String,
    #[serde(rename = "Standing")]
    standing: Option<i64>,
    #[serde(rename = "Title")]
    title: Option<i64>,
}

/// One node the account has completed.
#[derive(Deserialize)]
struct Completed {
    #[serde(rename = "Tag")]
    tag: String,
    #[serde(rename = "Completes")]
    completes: Option<i64>,
    #[serde(rename = "Tier")]
    tier: Option<i64>,
}

#[derive(Deserialize)]
struct Dated {
    #[serde(rename = "$date")]
    date: Millis,
}

impl Dated {
    fn seconds(&self) -> Option<Instant> {
        self.date
            .value
            .parse::<i64>()
            .ok()
            .map(|millis| millis / 1_000)
    }
}

#[derive(Deserialize)]
struct Millis {
    #[serde(rename = "$numberLong")]
    value: String,
}

#[derive(Deserialize)]
struct Synced {
    #[serde(rename = "$oid")]
    oid: String,
}

impl Synced {
    /// Creation time of the copy with this id: the first eight hex digits of an id are its Unix
    /// time in seconds; `None` when they are missing or not hex.
    fn minted(&self) -> Option<Instant> {
        i64::from_str_radix(self.oid.get(..8)?, 16).ok()
    }
}

/// Records the highest Forma count among the item's copies.
fn form(formed: &mut HashMap<String, u32>, item: String, forma: u32) {
    let most = formed.entry(item).or_default();
    *most = (*most).max(forma);
}

/// Records the earliest creation time among the item's copies.
fn acquire(acquired: &mut HashMap<String, Instant>, item: String, at: Instant) {
    acquired
        .entry(item)
        .and_modify(|kept| *kept = (*kept).min(at))
        .or_insert(at);
}

#[derive(Default)]
struct Document {
    created: Option<i64>,
    synced: Option<String>,
    tally: Tally,
    /// Builds in the foundry, one entry per build; `None` when the key is absent.
    building: Option<Vec<Building>>,
    /// Affinity per item, as the game counts it.
    mastery: Vec<Mastered>,
    /// Creation time of the oldest copy of each individually listed item.
    acquired: HashMap<String, Instant>,
    /// The highest Forma count on one copy of each item.
    formed: HashMap<String, u32>,
    rank: Option<u32>,
    next_test: Option<Instant>,
    missions: Vec<Mission>,
    intrinsics: Vec<Intrinsic>,
    affiliations: Vec<Affiliation>,
    allowances: Vec<Allowance>,
    /// What gear configs hold in their slots: a copy id, or the item key.
    installed: Vec<String>,
    /// Item and variant of each identified upgrade, by its id.
    identified: HashMap<String, (String, Variant)>,
    left: Option<u32>,
    platinum: Option<u32>,
    credits: Option<u64>,
    endo: Option<u64>,
    keys: Vec<String>,
}

impl Document {
    /// Merges in the fields parsed from a document nested inside this one.
    fn absorb(&mut self, nested: Document) {
        self.created = self.created.or(nested.created);
        self.synced = self.synced.take().or(nested.synced);
        self.tally = nested.tally;
        if let Some(builds) = nested.building {
            self.building.get_or_insert_default().extend(builds);
        }
        self.mastery.extend(nested.mastery);
        for (item, at) in nested.acquired {
            acquire(&mut self.acquired, item, at);
        }
        for (item, forma) in nested.formed {
            form(&mut self.formed, item, forma);
        }
        self.rank = self.rank.or(nested.rank);
        self.next_test = self.next_test.or(nested.next_test);
        self.missions.extend(nested.missions);
        self.intrinsics.extend(nested.intrinsics);
        self.affiliations.extend(nested.affiliations);
        self.allowances.extend(nested.allowances);
        self.installed.extend(nested.installed);
        self.identified.extend(nested.identified);
        self.left = self.left.or(nested.left);
        self.platinum = self.platinum.or(nested.platinum);
        self.credits = self.credits.or(nested.credits);
        self.endo = self.endo.or(nested.endo);
        self.keys.extend(nested.keys);
    }

    fn into_reading(mut self, name: Option<String>) -> Reading {
        let building = self.building.take();
        for build in building.iter().flatten() {
            self.tally.commit(&build.item, 1);
        }
        let crafting = building.map(|builds| {
            builds
                .iter()
                .filter_map(|build| {
                    Some(Crafting {
                        id: build.id.as_ref()?.oid.clone(),
                        item: build.item.clone(),
                        ready: build.ready.as_ref()?.seconds()?,
                    })
                })
                .collect()
        });
        let holdings: Vec<Holding> = self.tally.finish();
        let equipped = equipped(&self.installed, &self.identified);
        let mut acquired: Vec<Acquired> = self
            .acquired
            .drain()
            .map(|(item, at)| Acquired { item, at })
            .collect();
        acquired.sort_by(|one, other| one.item.cmp(&other.item));
        let mut formed: Vec<Formed> = self
            .formed
            .drain()
            .map(|(item, forma)| Formed { item, forma })
            .collect();
        formed.sort_by(|one, other| one.item.cmp(&other.item));

        Reading {
            inventory: Inventory {
                profile: self.created.map(|created| Profile {
                    created,
                    name: name.unwrap_or_default(),
                }),
                holdings,
                synced: self.synced,
                purse: Purse {
                    platinum: self.platinum,
                    credits: self.credits,
                    endo: self.endo,
                    trades_left: self.left,
                },
                mastery: self.mastery,
                equipped,
                progression: Progression {
                    rank: self.rank,
                    next_test: self.next_test,
                    missions: self.missions,
                    intrinsics: self.intrinsics,
                    crafting,
                    acquired,
                    formed,
                    affiliations: self.affiliations,
                    allowances: self.allowances,
                },
            },
            keys: self.keys,
        }
    }
}

/// Parses the inventory families of the document and skips everything else without
/// allocating it.
impl<'de> Deserialize<'de> for Document {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(Keys { nested: true })
    }
}

/// `nested` is true while a document nested inside this one may still be unwrapped. The game
/// nests at most one level, so only one level is unwrapped.
struct Keys {
    nested: bool,
}

impl<'de> Visitor<'de> for Keys {
    type Value = Document;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an inventory document")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Document, M::Error> {
        let mut document = Document::default();

        while let Some(key) = map.next_key::<String>()? {
            match family(&key) {
                Some(family) => {
                    for entry in map.next_value::<Vec<Entry>>()? {
                        let count = match family.counted {
                            Counted::Field => entry.count.unwrap_or_default().max(0) as u32,
                            Counted::Copy => 1,
                        };
                        for config in entry.configs.iter().flatten() {
                            document.installed.extend(config.0.iter().cloned());
                        }
                        if count > 0 {
                            let variant = tells(family.mark, &entry);
                            if let Some(id) = &entry.id {
                                document
                                    .identified
                                    .insert(id.oid.clone(), (entry.item.clone(), variant.clone()));
                            }
                            let minted = entry.id.as_ref().and_then(Synced::minted);
                            if let (Counted::Copy, None, Some(at)) =
                                (family.counted, family.mark, minted)
                            {
                                acquire(&mut document.acquired, entry.item.clone(), at);
                            }
                            if let Some(forma) = entry.polarized.filter(|&forma| forma > 0) {
                                form(&mut document.formed, entry.item.clone(), amount(forma));
                            }
                            document.tally.add(Holding {
                                detail: detail(&variant),
                                item: entry.item,
                                variant,
                                count,
                            });
                        }
                    }
                }
                None if key == CREATED => {
                    let created = map.next_value::<Dated>()?;
                    document.created = created.date.value.parse().ok();
                }
                None if key == RANK => {
                    document.rank = map.next_value::<Option<i64>>()?.map(amount);
                }
                None if key == NEXT_TEST => {
                    document.next_test = map
                        .next_value::<Option<Dated>>()?
                        .and_then(|dated| dated.seconds());
                }
                None if key == MISSIONS => {
                    for entry in map.next_value::<Vec<Completed>>()? {
                        document.missions.push(Mission {
                            tag: entry.tag,
                            completes: entry.completes.map(amount).unwrap_or_default(),
                            tier: entry.tier.map(amount),
                        });
                    }
                }
                None if key == SKILLS => {
                    let skills = map.next_value::<HashMap<String, serde_json::Value>>()?;
                    let mut intrinsics: Vec<Intrinsic> = skills
                        .into_iter()
                        .filter(|(school, _)| school.starts_with(INTRINSIC))
                        .filter_map(|(school, rank)| {
                            Some(Intrinsic {
                                school,
                                rank: amount(rank.as_i64()?),
                            })
                        })
                        .collect();
                    intrinsics.sort_by(|one, other| one.school.cmp(&other.school));
                    document.intrinsics = intrinsics;
                }
                None if key == AFFILIATIONS => {
                    for entry in map.next_value::<Vec<Affiliated>>()? {
                        document.affiliations.push(Affiliation {
                            tag: entry.tag,
                            standing: entry.standing.unwrap_or_default(),
                            title: entry
                                .title
                                .unwrap_or_default()
                                .clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                                as i32,
                        });
                    }
                }
                None if key.starts_with(DAILY) => {
                    if let Some(left) = map.next_value::<Option<i64>>()? {
                        document.allowances.push(Allowance {
                            pool: key[DAILY.len()..].to_owned(),
                            left: amount(left),
                        });
                    }
                }
                None if key == SYNCED => {
                    document.synced = Some(map.next_value::<Synced>()?.oid);
                }
                None if key == LEVELLED => {
                    for entry in map.next_value::<Vec<Levelled>>()? {
                        document.mastery.push(Mastered {
                            item: entry.item,
                            xp: entry.xp.unwrap_or_default().max(0) as u64,
                        });
                    }
                }
                None if key == BUILDING => {
                    document
                        .building
                        .get_or_insert_default()
                        .extend(map.next_value::<Vec<Building>>()?);
                }
                None if key == LEFT => {
                    document.left = map.next_value::<Option<i64>>()?.map(amount);
                }
                None if key == PLATINUM => {
                    document.platinum = map.next_value::<Option<i64>>()?.map(amount);
                }
                None if key == CREDITS => {
                    document.credits = map
                        .next_value::<Option<i64>>()?
                        .map(|value| value.max(0) as u64);
                }
                None if key == ENDO => {
                    document.endo = map
                        .next_value::<Option<i64>>()?
                        .map(|value| value.max(0) as u64);
                }
                None if key == WRAPPED && self.nested => {
                    let wrapped = map.next_value::<String>()?;
                    let mut inner = serde_json::Deserializer::from_str(&wrapped);
                    let read = inner
                        .deserialize_map(Keys { nested: false })
                        .map_err(serde::de::Error::custom)?;
                    document.absorb(read);
                }
                None => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
            document.keys.push(key);
        }

        Ok(document)
    }
}

/// A count from the dump as a `u32`, clamped to its range.
fn amount(value: i64) -> u32 {
    value.clamp(0, i64::from(u32::MAX)) as u32
}

/// The variant that distinguishes this entry from others of the same item: the upgrade
/// fingerprint or the socket count, as the family's mark selects; empty when it has none.
fn tells(mark: Option<Mark>, entry: &Entry) -> Variant {
    let told = match mark {
        Some(Mark::Fingerprint) => entry.fingerprint.clone(),
        Some(Mark::Sockets) => entry.sockets.map(|stars| stars.to_string()),
        None => None,
    };
    Variant(told.unwrap_or_default())
}

/// The fields of an upgrade fingerprint that median reads; the game writes more.
#[derive(Deserialize)]
struct Fingeprint {
    #[serde(default)]
    lvl: Option<i64>,
    /// The riven's weapon; `None` until the riven is revealed.
    #[serde(default)]
    compat: Option<String>,
    /// The challenge that reveals a riven; `None` once it is revealed.
    #[serde(default)]
    challenge: Option<IgnoredAny>,
    #[serde(default, rename = "lvlReq")]
    mastery: i64,
    #[serde(default)]
    rerolls: i64,
    #[serde(default)]
    pol: Option<String>,
    #[serde(default)]
    buffs: Vec<Rolled>,
    #[serde(default)]
    curses: Vec<Rolled>,
}

/// One stat as a riven's fingerprint encodes it.
#[derive(Deserialize)]
struct Rolled {
    #[serde(rename = "Tag")]
    tag: String,
    #[serde(rename = "Value")]
    roll: i64,
}

impl From<Rolled> for RivenStat {
    fn from(rolled: Rolled) -> Self {
        Self {
            tag: rolled.tag,
            roll: amount(rolled.roll),
        }
    }
}

/// The detail a variant encodes: ayatan stars, a riven (revealed or not) or a mod rank; `None`
/// for a plain variant, one that does not parse, or a fingerprint without a level.
pub fn detail(variant: &Variant) -> Option<Detail> {
    if variant.is_plain() {
        return None;
    }
    if let Ok(stars) = variant.0.parse::<i64>() {
        return Some(Detail::Stars(level(stars)));
    }

    let read: Fingeprint = serde_json::from_str(&variant.0).ok()?;
    let rank = level(read.lvl.unwrap_or(0));
    match (read.compat, read.challenge) {
        (Some(weapon), _) => Some(Detail::Riven {
            rank,
            revealed: Some(Riven {
                weapon,
                mastery: level(read.mastery),
                rerolls: amount(read.rerolls),
                polarity: read.pol.as_deref().and_then(Polarity::from_code),
                buffs: read.buffs.into_iter().map(RivenStat::from).collect(),
                curses: read.curses.into_iter().map(RivenStat::from).collect(),
            }),
        }),
        (None, Some(_)) => Some(Detail::Riven {
            rank,
            revealed: None,
        }),
        (None, None) => read.lvl.map(|lvl| Detail::Rank(level(lvl))),
    }
}

/// The copies installed on gear, deduplicated and sorted. A slot holds the item key when the
/// copy came from an unranked stack, and the copy's id when the game lists copies individually.
fn equipped(
    installed: &[String],
    identified: &HashMap<String, (String, Variant)>,
) -> Vec<Equipped> {
    let mut found: Vec<Equipped> = Vec::new();
    for slot in installed {
        let copy = match identified.get(slot) {
            Some((item, variant)) => Equipped {
                item: item.clone(),
                variant: variant.clone(),
            },
            None if slot.starts_with('/') => Equipped {
                item: slot.clone(),
                variant: Variant::default(),
            },
            None => continue,
        };
        if !found.contains(&copy) {
            found.push(copy);
        }
    }
    found.sort_by(|one, other| (&one.item, &one.variant).cmp(&(&other.item, &other.variant)));
    found
}

/// A level from the dump as a `u8`, clamped to its range.
fn level(value: i64) -> u8 {
    value.clamp(0, i64::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMP: &str = include_str!("../tests/fixtures/inventory.json");

    /// Wraps an inventory document as the collector wraps it in a periodic poll.
    fn polled(inventory: &str) -> String {
        let quoted = serde_json::to_string(inventory).expect("a string");
        format!(
            r#"{{"source":"poll","info":{{"res":{{"game_info":{{"username":"Tenno"}},"match_info":{{"inventory":{quoted}}}}}}}}}"#
        )
    }

    /// Wraps an inventory document as a pushed update arrives.
    fn pushed(inventory: &str) -> String {
        let quoted = serde_json::to_string(inventory).expect("a string");
        format!(r#"{{"source":"info","info":{{"match_info":{{"inventory":{quoted}}}}}}}"#)
    }

    fn holding<'a>(reading: &'a Reading, item: &str) -> Option<&'a Holding> {
        reading
            .inventory
            .holdings
            .iter()
            .find(|held| held.item == item)
    }

    fn read_dump() -> Reading {
        read(&polled(DUMP))
            .expect("a reading")
            .expect("an inventory")
    }

    /// A document with the account's mastery progress and one blueprint.
    const PROGRESSED: &str = r#"{"PlayerLevel":18,
        "TrainingDate":{"$date":{"$numberLong":"1788871622000"}},
        "Missions":[{"Tag":"SolNode27","Completes":49,"Tier":1},
                    {"Tag":"EarthToVenusJunction","Completes":1}],
        "PlayerSkills":{"LPP_SPACE":33845,"LPS_PILOTING":6,"LPS_DRIFT_RIDING":7},
        "PendingRecipes":[{"ItemType":"/Lotus/Types/Recipes/Weapons/PolearmPrimeBlueprint",
                           "CompletionDate":{"$date":{"$numberLong":"1788358770000"}},
                           "ItemId":{"$oid":"66e0a1b2c3d4e5f6a7b8c9d0"}}],
        "Recipes":[{"ItemType":"/Lotus/Types/Recipes/Weapons/PolearmPrimeBlueprint","ItemCount":2}]}"#;

    /// A document with the account's standing with several syndicates.
    const AFFILIATED: &str = r#"{"Affiliations":[
            {"Standing":298500,"Title":5,"Tag":"RedVeilSyndicate"},
            {"Standing":-71000,"Title":-2,"Tag":"ArbitersSyndicate"},
            {"Standing":25,"Tag":"VentKidsSyndicate"}],
        "DailyAffiliation":0,"DailyAffiliationQuills":25500,
        "MiscItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/Kuva","ItemCount":1602}]}"#;

    #[test]
    fn the_dump_tells_where_the_account_stands_with_each_syndicate() {
        let reading = read(&polled(AFFILIATED))
            .expect("a reading")
            .expect("an inventory");
        let progression = &reading.inventory.progression;

        assert_eq!(
            progression.affiliations,
            vec![
                Affiliation {
                    tag: "RedVeilSyndicate".into(),
                    standing: 298_500,
                    title: 5,
                },
                Affiliation {
                    tag: "ArbitersSyndicate".into(),
                    standing: -71_000,
                    title: -2,
                },
                Affiliation {
                    tag: "VentKidsSyndicate".into(),
                    standing: 25,
                    title: 0,
                },
            ]
        );
        assert_eq!(
            progression.allowances,
            vec![
                Allowance {
                    pool: String::new(),
                    left: 0,
                },
                Allowance {
                    pool: "Quills".into(),
                    left: 25_500,
                },
            ]
        );
    }

    #[test]
    fn the_dump_tells_how_far_the_account_has_come() {
        let reading = read(&polled(PROGRESSED))
            .expect("a reading")
            .expect("an inventory");
        let progression = &reading.inventory.progression;

        assert_eq!(progression.rank, Some(18));
        assert_eq!(progression.next_test, Some(1_788_871_622));
        assert_eq!(
            progression.missions,
            vec![
                Mission {
                    tag: "SolNode27".into(),
                    completes: 49,
                    tier: Some(1),
                },
                Mission {
                    tag: "EarthToVenusJunction".into(),
                    completes: 1,
                    tier: None,
                },
            ]
        );
        assert_eq!(
            progression.intrinsics,
            vec![
                Intrinsic {
                    school: "LPS_DRIFT_RIDING".into(),
                    rank: 7,
                },
                Intrinsic {
                    school: "LPS_PILOTING".into(),
                    rank: 6,
                },
            ]
        );
        assert_eq!(
            progression.crafting,
            Some(vec![Crafting {
                id: "66e0a1b2c3d4e5f6a7b8c9d0".into(),
                item: "/Lotus/Types/Recipes/Weapons/PolearmPrimeBlueprint".into(),
                ready: 1_788_358_770,
            }])
        );
        assert_eq!(
            holding(
                &reading,
                "/Lotus/Types/Recipes/Weapons/PolearmPrimeBlueprint"
            )
            .map(|held| held.count),
            Some(1)
        );
    }

    #[test]
    fn the_most_formed_copy_of_an_item_says_how_much_forma_it_carries() {
        let document = r#"{"Pistols":[
            {"ItemType":"/Lotus/Weapons/Grineer/KuvaLich/Secondaries/Kraken/KuvaKraken"},
            {"ItemType":"/Lotus/Weapons/Grineer/KuvaLich/Secondaries/Nukor/KuvaNukor","Polarized":2},
            {"ItemType":"/Lotus/Weapons/Grineer/KuvaLich/Secondaries/Nukor/KuvaNukor","Polarized":5}]}"#;

        let reading = read(&polled(document))
            .expect("a reading")
            .expect("an inventory");

        assert_eq!(
            reading.inventory.progression.formed,
            vec![Formed {
                item: "/Lotus/Weapons/Grineer/KuvaLich/Secondaries/Nukor/KuvaNukor".into(),
                forma: 5,
            }]
        );
    }

    #[test]
    fn the_oldest_copy_of_an_item_says_when_the_account_got_it() {
        let document = r#"{"LongGuns":[
            {"ItemType":"/Lotus/Weapons/Tenno/Rifle/Braton","ItemId":{"$oid":"66e0a1b2c3d4e5f6a7b8c9d0"}},
            {"ItemType":"/Lotus/Weapons/Tenno/Rifle/Braton","ItemId":{"$oid":"5fadacad88d2db551909309e"}},
            {"ItemType":"/Lotus/Weapons/Tenno/Rifle/Soma","ItemId":{"$oid":"7"}}],
            "Upgrades":[{"ItemType":"/Lotus/Upgrades/Mods/Rifle/Serration",
                         "ItemId":{"$oid":"66e0a1b2c3d4e5f6a7b8c9d0"},
                         "UpgradeFingerprint":"{\"lvl\":1}"}]}"#;

        let reading = read(&polled(document))
            .expect("a reading")
            .expect("an inventory");

        assert_eq!(
            reading.inventory.progression.acquired,
            vec![Acquired {
                item: "/Lotus/Weapons/Tenno/Rifle/Braton".into(),
                at: 0x5fad_acad,
            }]
        );
    }

    #[test]
    fn progression_survives_the_document_the_game_wraps() {
        let quoted = serde_json::to_string(PROGRESSED).expect("a string");
        let wrapped = format!(r#"{{"InventoryJson":{quoted}}}"#);

        let reading = read(&polled(&wrapped))
            .expect("a reading")
            .expect("an inventory");

        assert_eq!(reading.inventory.progression.rank, Some(18));
        assert_eq!(reading.inventory.progression.missions.len(), 2);
        assert_eq!(
            reading
                .inventory
                .progression
                .crafting
                .as_ref()
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn a_dump_silent_on_the_foundry_says_nothing_of_it_and_an_idle_one_says_so() {
        let silent =
            r#"{"MiscItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/Rubedo","ItemCount":1}]}"#;
        let idle = r#"{"PendingRecipes":[],
                       "MiscItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/Rubedo","ItemCount":1}]}"#;

        let crafting = |document: &str| {
            read(&polled(document))
                .expect("a reading")
                .expect("an inventory")
                .inventory
                .progression
                .crafting
        };

        assert_eq!(crafting(silent), None);
        assert_eq!(crafting(idle), Some(Vec::new()));
    }

    #[test]
    fn an_envelope_carrying_no_inventory_is_not_an_error() {
        let payload = r#"{"source":"events","info":[{"name":"kill"}]}"#;

        assert_eq!(read(payload), Ok(None));
    }

    /// Wraps the document as the game does from version 310.2.9 on.
    fn wrapped(inventory: &str) -> String {
        let quoted = serde_json::to_string(inventory).expect("a string");
        format!(r#"{{"InventoryJson":{quoted},"MissionRewards":[]}}"#)
    }

    #[test]
    fn the_document_is_read_through_the_wrapper_the_game_puts_round_it() {
        let plain = read_dump();

        let through = read(&polled(&wrapped(DUMP)))
            .expect("a reading")
            .expect("an inventory");

        assert_eq!(through.inventory, plain.inventory);
        assert!(through.keys.iter().any(|key| key == "MissionRewards"));
    }

    #[test]
    fn a_shape_that_holds_nothing_is_refused_rather_than_read_as_empty() {
        let strange = r#"{"SomethingNew":{"holdings":[]},"MissionRewards":[]}"#;

        assert_eq!(read(&polled(strange)), Err(InventoryError::NothingHeld));
    }

    #[test]
    fn an_inventory_that_is_not_a_document_is_refused() {
        let payload = polled("not a document");

        assert!(matches!(
            read(&payload),
            Err(InventoryError::MalformedInventory(_))
        ));
    }

    #[test]
    fn both_shapes_a_feed_arrives_in_are_read() {
        let polled = read(&polled(DUMP))
            .expect("a reading")
            .expect("an inventory");
        let pushed = read(&pushed(DUMP))
            .expect("a reading")
            .expect("an inventory");

        assert_eq!(polled.inventory.holdings, pushed.inventory.holdings);
    }

    #[test]
    fn a_family_counted_by_its_own_field_keeps_that_count() {
        let reading = read_dump();

        let rubedo = holding(&reading, "/Lotus/Types/Items/MiscItems/Rubedo");
        assert_eq!(rubedo.map(|held| held.count), Some(24928));
    }

    #[test]
    fn a_family_listed_one_copy_at_a_time_counts_the_copies() {
        let reading = read_dump();

        let excalibur = holding(&reading, "/Lotus/Powersuits/Excalibur/Excalibur");
        assert_eq!(excalibur.map(|held| held.count), Some(2));
    }

    #[test]
    fn a_mod_carries_the_rank_that_tells_it_from_another_copy() {
        let reading = read_dump();

        let ranked: Vec<&Holding> = reading
            .inventory
            .holdings
            .iter()
            .filter(|held| held.item == "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod")
            .collect();
        assert_eq!(ranked.len(), 2, "the two ranks were run together");
        assert!(ranked.iter().all(|held| !held.variant.is_plain()));
    }

    #[test]
    fn an_ayatan_carries_the_stars_set_into_it() {
        let reading = read_dump();

        let oro = holding(&reading, "/Lotus/Types/Items/FusionTreasures/OroFusexD");
        assert_eq!(oro.map(|held| held.variant.0.as_str()), Some("7"));
    }

    #[test]
    fn the_account_is_named_by_what_no_rename_changes() {
        let reading = read_dump();

        let profile = reading.inventory.profile.expect("an identified account");
        assert_eq!(profile.created, 1_700_000_000_000);
        assert_eq!(profile.name, "Tenno");
    }

    #[test]
    fn a_dump_without_a_creation_date_identifies_nobody() {
        let reading = read(&polled(
            r#"{"MiscItems":[{"ItemType":"Rubedo","ItemCount":1}]}"#,
        ))
        .expect("a reading")
        .expect("an inventory");

        assert_eq!(reading.inventory.profile, None);
        assert_eq!(reading.inventory.holdings.len(), 1);
    }

    #[test]
    #[ignore = "reads a dump off the machine that made it"]
    fn a_dump_off_the_machine_reads() {
        let path = std::env::var("MEDIAN_DUMP").expect("MEDIAN_DUMP names a dump");
        let raw = std::fs::read_to_string(path).expect("the dump reads");

        let reading = read(&raw).expect("a reading").expect("an inventory");

        println!(
            "holdings {} equipped {} mastery {}",
            reading.inventory.holdings.len(),
            reading.inventory.equipped.len(),
            reading.inventory.mastery.len()
        );
        assert!(!reading.inventory.holdings.is_empty());
    }

    #[test]
    fn a_blueprint_standing_in_the_foundry_is_not_held() {
        let reading = read_dump();

        let held = holding(
            &reading,
            "/Lotus/Types/Recipes/Weapons/BurstonPrimeBlueprint",
        )
        .expect("the spare copy");
        assert_eq!(held.count, 1);
    }

    #[test]
    fn the_last_copy_taken_by_a_build_leaves_no_holding() {
        let reading = read(&polled(
            r#"{"Recipes":[{"ItemType":"/Lotus/Types/Recipes/Components/FormaBlueprint","ItemCount":1},
                           {"ItemType":"Rubedo","ItemCount":1}],
                "PendingRecipes":[{"ItemType":"/Lotus/Types/Recipes/Components/FormaBlueprint"}]}"#,
        ))
        .expect("a reading")
        .expect("an inventory");

        assert!(holding(&reading, "/Lotus/Types/Recipes/Components/FormaBlueprint").is_none());
        assert!(holding(&reading, "Rubedo").is_some());
    }

    #[test]
    fn a_copy_a_loadout_names_is_read_as_installed() {
        let reading = read_dump();

        assert_eq!(
            reading.inventory.equipped,
            vec![
                Equipped {
                    item: "/Lotus/Upgrades/Mods/Warframe/AvatarHealthMaxMod".into(),
                    variant: Variant::default(),
                },
                Equipped {
                    item: "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod".into(),
                    variant: Variant(r#"{"lvl":5}"#.into()),
                },
            ]
        );
    }

    #[test]
    fn the_affinity_the_game_counted_comes_through() {
        let reading = read_dump();

        let burston = reading
            .inventory
            .mastery
            .iter()
            .find(|levelled| levelled.item == "/Lotus/Weapons/Tenno/Rifle/BurstonPrime")
            .expect("the rifle was levelled");
        assert_eq!(burston.xp, 450_000);
        assert_eq!(reading.inventory.mastery.len(), 2);
    }

    #[test]
    fn a_key_this_build_does_not_read_neither_stops_it_nor_disappears() {
        let reading = read_dump();

        assert!(holding(&reading, "ChallengeProgress").is_none());
        assert!(reading.keys.iter().any(|key| key == "ChallengeProgress"));
    }

    #[test]
    fn the_marker_is_found_through_every_wrapper_the_dump_arrives_in() {
        let found = "6a7306dd0785235c0a0ddea1";

        assert_eq!(marker(&polled(DUMP)), Some(found));
        assert_eq!(marker(&pushed(DUMP)), Some(found));
        assert_eq!(marker(&polled(&wrapped(DUMP))), Some(found));
    }

    #[test]
    fn the_marker_the_document_carries_is_the_one_it_is_read_by() {
        let reading = read_dump();

        assert_eq!(reading.inventory.synced.as_deref(), marker(&polled(DUMP)));
    }

    #[test]
    fn a_dump_without_a_marker_has_none_rather_than_a_wrong_one() {
        let payload = polled(r#"{"MiscItems":[{"ItemType":"Rubedo","ItemCount":1}]}"#);

        assert_eq!(marker(&payload), None);
    }

    #[test]
    fn a_marker_that_is_not_the_shape_it_was_reads_as_none() {
        let strange = DUMP.replace(r#""$oid": "6a7306dd0785235c0a0ddea1""#, r#""$oid": "7""#);
        assert_ne!(strange, DUMP, "the fixture lost the field under test");

        assert_eq!(marker(&polled(&strange)), None);
    }

    #[test]
    fn what_moves_between_missions_stays_out_of_the_fingerprint() {
        let restless = DUMP.replace(r#""RewardSeed": -1234"#, r#""RewardSeed": 9999"#);
        assert_ne!(restless, DUMP, "the fixture lost the field under test");

        let before = read_dump().inventory.digest();
        let after = read(&polled(&restless))
            .expect("a reading")
            .expect("an inventory")
            .inventory
            .digest();

        assert_eq!(before, after);
    }

    #[test]
    fn a_ranked_mod_says_which_rank_it_is() {
        let reading = read_dump();

        let ranks: Vec<Option<Detail>> = reading
            .inventory
            .holdings
            .iter()
            .filter(|held| held.item == "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod")
            .map(|held| held.detail.clone())
            .collect();

        assert_eq!(ranks, vec![Some(Detail::Rank(2)), Some(Detail::Rank(5))]);
    }

    #[test]
    fn an_ayatan_says_how_many_stars_are_set_into_it() {
        let reading = read_dump();

        let oro = holding(&reading, "/Lotus/Types/Items/FusionTreasures/OroFusexD");
        assert_eq!(
            oro.and_then(|held| held.detail.clone()),
            Some(Detail::Stars(7))
        );
    }

    #[test]
    fn a_revealed_riven_says_which_weapon_it_rolled_for() {
        let reading = read_dump();

        let riven = holding(
            &reading,
            "/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare",
        );
        assert_eq!(
            riven.and_then(|held| held.detail.clone()),
            Some(Detail::Riven {
                rank: 0,
                revealed: Some(Riven {
                    weapon: "/Lotus/Weapons/Tenno/Rifle/Rifle".into(),
                    mastery: 0,
                    rerolls: 3,
                    polarity: None,
                    buffs: vec![RivenStat {
                        tag: "WeaponCritChanceMod".into(),
                        roll: 858_993_459,
                    }],
                    curses: Vec::new(),
                }),
            })
        );
    }

    #[test]
    fn a_veiled_riven_says_it_is_a_riven_and_nothing_more() {
        let veiled = Variant(
            r#"{"challenge":{"Type":"/Lotus/Types/Challenges/PlainsTimedVariety","Progress":0,"Required":1}}"#
                .into(),
        );

        assert_eq!(
            detail(&veiled),
            Some(Detail::Riven {
                rank: 0,
                revealed: None,
            })
        );
    }

    #[test]
    fn a_riven_that_has_been_ranked_up_says_how_far() {
        let rolled = Variant(
            r#"{"compat":"/Lotus/Weapons/Tenno/Rifle/Rifle","lim":0,"lvl":7,"rerolls":3,"buffs":[]}"#
                .into(),
        );

        assert!(matches!(
            detail(&rolled),
            Some(Detail::Riven { rank: 7, .. })
        ));
    }

    #[test]
    fn a_rolled_riven_says_every_stat_it_rolled() {
        let rolled = Variant(
            r#"{"compat":"/Lotus/Weapons/Infested/Pistols/InfVomitGun/InfVomitGunWep","lim":67612231,"lvlReq":14,"rerolls":114,"pol":"AP_DEFENSE","buffs":[{"Tag":"WeaponDamageAmountMod","Value":469376964},{"Tag":"WeaponStunChanceMod","Value":880276416},{"Tag":"WeaponCritDamageMod","Value":141367479}],"curses":[{"Tag":"WeaponRecoilReductionMod","Value":141767491}],"lvl":8}"#
                .into(),
        );
        let stat = |tag: &str, roll| RivenStat {
            tag: tag.into(),
            roll,
        };

        assert_eq!(
            detail(&rolled),
            Some(Detail::Riven {
                rank: 8,
                revealed: Some(Riven {
                    weapon: "/Lotus/Weapons/Infested/Pistols/InfVomitGun/InfVomitGunWep".into(),
                    mastery: 14,
                    rerolls: 114,
                    polarity: Some(Polarity::Vazarin),
                    buffs: vec![
                        stat("WeaponDamageAmountMod", 469_376_964),
                        stat("WeaponStunChanceMod", 880_276_416),
                        stat("WeaponCritDamageMod", 141_367_479),
                    ],
                    curses: vec![stat("WeaponRecoilReductionMod", 141_767_491)],
                }),
            })
        );
    }

    #[test]
    fn a_variant_this_build_cannot_read_says_nothing() {
        assert_eq!(detail(&Variant::default()), None);
        assert_eq!(detail(&Variant(r#"{"lim":0}"#.into())), None);
        assert_eq!(detail(&Variant("not a fingerprint".into())), None);
    }

    #[test]
    fn the_purse_comes_out_of_the_dump() {
        let purse = read_dump().inventory.purse;

        assert_eq!(purse.platinum, Some(3));
        assert_eq!(purse.credits, Some(1337));
        assert_eq!(purse.endo, Some(68015));
        assert_eq!(purse.trades_left, Some(11));
    }
}
