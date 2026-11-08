use serde::{Deserialize, Serialize};
use specta::Type;

/// Unix time in seconds, sent as a `u32`.
pub type Instant = u32;

/// A world cycle that repeats through fixed phases, such as Cetus day and night, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CycleKey {
    Cetus,
    Earth,
    Vallis,
    Cambion,
    Duviri,
}

/// One phase of a cycle, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Phase {
    pub key: String,
    pub seconds: u32,
}

/// A cycle and the phase it is in, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Cycle {
    pub key: CycleKey,
    /// Phases in the order they run.
    pub phases: Vec<Phase>,
    /// Index into `phases` of the one running now.
    pub phase: u32,
    pub ends: Instant,
}

/// Relic era of a void fissure, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Era {
    Lith,
    Meso,
    Neo,
    Axi,
    Requiem,
    Omnia,
}

/// One open void fissure, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Fissure {
    pub node: String,
    pub era: Era,
    pub hard: bool,
    pub storm: bool,
    /// Mission type code from the world-state document (`MT_DEFENSE`); `None` for void storms.
    pub mission: Option<String>,
    pub expires: Instant,
}

/// A game-wide reset timer, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Reset {
    Daily,
    Sortie,
    Weekly,
}

/// A reset and the time it next happens, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Deadline {
    pub reset: Reset,
    pub at: Instant,
}

/// A trader who visits periodically, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Vendor {
    Baro,
    Varzia,
}

/// A trader's visit to a relay: where, and when it starts and ends, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Visit {
    pub vendor: Vendor,
    pub node: String,
    pub arrives: Instant,
    pub departs: Instant,
}

/// Market price of an item, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Quote {
    /// Game key of the item (a `/Lotus/…` path).
    pub item: String,
    /// Median closed-trade price in platinum; for an item traded only as a set, the set's.
    pub platinum: Option<u32>,
    /// The market pair the price comes from; pass it to ask the market cache to keep it fresh.
    pub pair: Option<Pair>,
}

/// Fissure type a fissure watch matches, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FissureKind {
    Plain,
    Hard,
    Storm,
}

/// A world event the user asked to be notified about, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Watch {
    /// A cycle entering a phase.
    Phase { cycle: CycleKey, phase: String },
    /// Baro arriving, or Varzia's stock rotating.
    Arrival { vendor: Vendor },
    /// A fissure opening that matches every condition set: era, mission type, fissure type.
    Fissure {
        era: Option<Era>,
        mission: Option<String>,
        of: Option<FissureKind>,
    },
}

/// A watch that fired, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Alarm {
    pub watch: Watch,
    /// Node where it fired; set only for a fissure or a trader watch.
    pub node: Option<String>,
}

/// World state, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Worldstate {
    pub at: Instant,
    pub cycles: Vec<Cycle>,
    pub fissures: Vec<Fissure>,
    pub deadlines: Vec<Deadline>,
    pub visits: Vec<Visit>,
}

/// Currency of a price, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Currency {
    Ducats,
    Credits,
    Aya,
    RegalAya,
}

/// One amount in one currency, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Price {
    pub currency: Currency,
    pub amount: u32,
}

/// One item a trader sells, with its prices, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Offer {
    pub item: String,
    pub prices: Vec<Price>,
}

/// A visiting trader's current stock, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Shelf {
    pub vendor: Vendor,
    pub offers: Vec<Offer>,
}

/// One item with its name, image and attributes from the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Good {
    pub item: String,
    pub name: String,
    /// Catalog kind slug, such as `primary` or `ship-deco`.
    pub kind: String,
    /// Image id inside the image pack; `None` when the item has no image.
    pub image: Option<String>,
    /// Whether the catalog has a row for the key at all.
    pub known: bool,
    pub slug: Option<String>,
    pub tradable: Option<bool>,
    /// Whether the item is vaulted.
    pub vaulted: Option<bool>,
    /// Whether the game counts the item as prime.
    pub prime: bool,
    pub ducats: Option<u32>,
    /// Kind of the item this blueprint builds; `None` when the item is not a blueprint.
    pub makes: Option<String>,
    /// Whether the account has levelled the item this one builds to max rank; `None` when the
    /// item builds nothing or no inventory dump reports it.
    pub mastered: Option<bool>,
    /// The vendor-sold arcane collection that can roll this item.
    pub collection: Option<String>,
}

/// Mastery from one source: points earned, and entries done out of the total.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub struct MasteryShare {
    pub earned: u32,
    pub done: u32,
    pub of: u32,
}

/// Whether the test for the next mastery rank can be taken, wire form.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum RankTest {
    /// The account has not earned enough for the next rank.
    Unearned,
    Open,
    /// Earned, but the test is locked until `until` after the last attempt.
    Shut {
        until: Instant,
    },
}

/// A mastery source that could not be counted, wire form.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Uncounted {
    Items,
    Nodes,
}

/// The account's mastery rank and progress, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MasteryRank {
    /// `None` until an inventory dump reports the account's rank.
    pub rank: Option<u32>,
    pub total: u32,
    /// Mastery points still available from items the account holds or has finished in the
    /// foundry.
    pub in_hand: u32,
    /// Mastery points the current rank required.
    pub floor: Option<u32>,
    /// Whether every source was counted; otherwise `total` is a lower bound.
    pub exact: bool,
    /// Mastery points the next rank requires; `remaining` is how many are still to earn.
    pub next: Option<u32>,
    pub remaining: Option<u32>,
    pub test: Option<RankTest>,
    /// Every source was counted, yet `total` is below what the current rank required.
    pub undercounted: bool,
    pub items: MasteryShare,
    pub nodes: MasteryShare,
    pub steel_path: MasteryShare,
    pub junctions: MasteryShare,
    pub steel_junctions: MasteryShare,
    pub railjack: MasteryShare,
    pub drifter: MasteryShare,
    pub uncounted: Vec<Uncounted>,
    /// Every item that gives mastery, mastered or not.
    pub masterable: Vec<Masterable>,
}

/// An item that gives mastery, with how far the account has levelled it, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Masterable {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    pub class: String,
    pub kind: String,
    pub rank: u32,
    pub cap: u32,
    /// Mastery points the item still gives; zero once it is mastered.
    pub remaining: u32,
    /// Forma still needed to reach its rank cap: above rank 30, each Forma raises the cap by two.
    pub forma: u32,
    /// Level-ups still needed; every Forma resets the item to rank 0.
    pub passes: u32,
    /// Mastery rank its blueprint requires.
    pub required: u32,
    pub owned: bool,
    pub prime: bool,
    pub vaulted: bool,
    /// When the account got its first copy; `None` when no inventory dump gives a date.
    pub acquired: Option<Instant>,
    /// Recipe inputs the account already holds enough of.
    pub gathered: u32,
    /// Number of recipe inputs; zero when no recipe builds it.
    pub inputs: u32,
    /// Recipe inputs the inventory lacks; filled only while its blueprint is held.
    pub lacking: Vec<Lack>,
    pub stage: Stage,
    /// How to obtain the hardest of its keys (its blueprint and the blueprints of its components,
    /// or the item itself when nothing builds it); `None` while any key has no known source.
    pub way: Option<Way>,
}

/// A recipe input the inventory lacks, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Lack {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// Copies still missing.
    pub short: u32,
}

/// How close an item is to giving its mastery; variants are ordered closest first, wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    /// Held or finished in the foundry; only levelling is left.
    Level,
    /// Its blueprint and every input are held.
    Build,
    /// Its blueprint is held, some inputs are not.
    Short,
    /// Its blueprint, or the item itself, is still to obtain.
    Obtain,
    Mastered,
}

/// How one key of an item (its blueprint, a component blueprint, or the item itself) is
/// obtained, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Way {
    /// A permanent vendor sells it for credits only.
    Credits { vendor: String, credits: u32 },
    /// A permanent vendor sells it for syndicate standing or another currency.
    Vendor {
        vendor: String,
        /// Currency key as the catalog spells it.
        currency: String,
        /// Localized name of the currency, when the currency is an item.
        coin: Option<String>,
        cost: Option<u32>,
        /// Syndicate rank the offer requires.
        rank: Option<u32>,
        area: Option<String>,
    },
    /// The clan lab researches it.
    Lab { lab: String },
    /// It drops from relics; `vaulted` when none of those relics drops any more.
    Relics { vaulted: bool },
    /// A location or an enemy drops it; `chance` is in tenths of a percent.
    Drop {
        place: String,
        area: Option<String>,
        enemy: bool,
        chance: u32,
        /// Planet of the location; `None` when the star chart has none.
        planet: Option<String>,
        /// Mission type there; for a bounty, the bounty activity (such as Cetus Bounties).
        mission: Option<String>,
        #[serde(rename = "levelFrom")]
        level_from: Option<u32>,
        #[serde(rename = "levelTo")]
        level_to: Option<u32>,
        /// Planets where the enemy appears, most frequent first.
        haunts: Vec<String>,
    },
    /// A visiting trader sells it.
    Visit { vendor: String },
}

/// One item of the collection view, with what the account holds of it, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Collectible {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    pub class: String,
    pub kind: String,
    pub prime: bool,
    pub vaulted: bool,
    pub tradable: bool,
    /// Copies the account holds.
    pub owned: u32,
    /// Whether the account has levelled it to max rank; `None` when it gives no mastery.
    pub mastered: Option<bool>,
    /// Whether every recipe input is held; `None` when no recipe builds it.
    pub ready: Option<bool>,
    /// Mastery rank its blueprint requires.
    pub required: u32,
    /// Whether the account's rank is high enough to build it; `None` while the rank is unknown.
    pub reachable: Option<bool>,
    /// Whether relics the account holds can drop one of its recipe inputs.
    pub relics: bool,
}

/// How far the inventory covers one step of a build tree, wire form.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Readiness {
    Held,
    /// Not held, but every input it is built from is held.
    Buildable,
    Short,
}

/// An item's build tree, with the total cost of the whole build, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Tree {
    pub root: Branch,
    /// Credits for the whole build, including every component's own build.
    pub credits: u32,
    /// Credits the account holds; `None` until an inventory dump reports them.
    pub purse: Option<u32>,
    /// Foundry time along the longest build chain, in seconds.
    pub time: u32,
}

/// One step of an item's build tree, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Branch {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// Copies this step requires.
    pub count: u32,
    /// Inventory copies not yet allocated to earlier steps when the traversal reached this one.
    pub have: u32,
    pub readiness: Readiness,
    pub tradable: bool,
    /// Lowest price of one copy among sellers in game, in platinum.
    pub price: Option<u32>,
    /// How to build the copies the inventory does not cover.
    pub craft: Option<Forge>,
}

/// Recipe for the copies of a step the inventory does not cover, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Forge {
    pub blueprint: String,
    pub name: String,
    pub image: Option<String>,
    /// Blueprints the build requires.
    pub count: u32,
    /// Blueprints the account holds.
    pub have: u32,
    /// Lowest price of the blueprint among sellers in game, in platinum.
    pub price: Option<u32>,
    /// Credits for this build alone, excluding its components' own builds.
    pub credits: u32,
    /// Foundry time in seconds.
    pub time: u32,
    pub parts: Vec<Branch>,
}

/// One line of the roadmap, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// Catalog kind slug; groups the line under its class.
    pub kind: String,
    /// Copies required.
    pub count: u32,
    /// Copies the account holds.
    pub have: u32,
    /// Copies the foundry is building.
    pub forging: u32,
    /// Whether the copies the inventory lacks are built from the lines below it.
    pub built: bool,
    pub tradable: bool,
    /// warframe.market slug; `None` when the market does not trade it.
    pub slug: Option<String>,
    /// Lowest price of one copy among sellers in game; `None` when the market cache has no
    /// such order.
    pub price: Option<u32>,
    /// Whether it is to be bought rather than farmed.
    pub buy: bool,
    /// Relics the account holds that can drop it.
    pub owned_relics: u32,
    /// Chance, in tenths of a percent, that opening all of those relics drops it at least once.
    pub owned_chance: u32,
    /// Relics still in the drop tables that contain it.
    pub relics: u32,
    /// Names of the relics that contain it, relics the account holds first.
    pub relic_names: Vec<String>,
    /// Whether it is vaulted, so no relic that contains it still drops.
    pub vaulted: bool,
    /// The easiest way to obtain it; `None` when the catalog lists none.
    pub way: Option<Way>,
    /// Every way to obtain it, easiest first; each location appears once, at its best chance.
    pub ways: Vec<Way>,
    /// Runs of the best-yielding location needed to cover the shortfall, counting only drops
    /// the location gives every run; `None` when nothing is short, when no location drops it
    /// every run, or when enemies drop it (it is then farmed by killing).
    pub runs: Option<u32>,
    /// Keys of the areas where it is obtained: it drops there, or a vendor there sells it.
    pub areas: Vec<String>,
}

/// Everything needed to obtain one or more items, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    /// Lines still to obtain first, then lines the inventory covers.
    pub lines: Vec<Requirement>,
    pub credits: u32,
    /// Credits the account holds; `None` until an inventory dump reports them.
    pub purse: Option<u32>,
    /// Foundry time along the longest build chain, in seconds.
    pub time: u32,
    /// Total platinum for the lines to buy, at the lowest price among sellers in game.
    pub to_buy: u32,
    /// Lines to buy the market cache has no price for yet.
    pub unpriced: u32,
}

/// One item in the roadmap still to master, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Planned {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// Whether the account already holds it; it then only needs levelling.
    pub owned: bool,
    /// Mastery points it still gives.
    pub remaining: u32,
}

/// An item the foundry can start building now, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Ready {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// Copies the plan needs built.
    pub count: u32,
}

/// The roadmap: requirements of each chosen item and of all of them together, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Roadmap {
    pub planned: Vec<Planned>,
    pub total: Requirements,
    /// Mastery points the planned items still give.
    pub gain: u32,
    /// Mastery rank the account reaches with that gain; `None` while its rank is unknown.
    pub reaches: Option<u32>,
    /// Items the foundry can start building now.
    pub ready: Vec<Ready>,
    /// Builds the inventory allows (sub-builds first where needed) and finished foundry items
    /// the plan uses; items that give mastery are excluded.
    pub forge: Vec<Requirement>,
    /// Areas where missing items are obtained, the area that covers the most first.
    pub areas: Vec<Area>,
}

/// A currency's name and image from the catalog; some currencies are ordinary items, some have
/// no item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Coin {
    pub currency: Currency,
    pub name: String,
    pub image: Option<String>,
}

/// A reward a relic contains, or a relic that contains an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Reward {
    /// Item key, used to open that item's card.
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// `common`, `uncommon` or `rare`, as the drop tables grade it.
    pub rarity: String,
    /// Refinement of the relic, when the row is a relic: the game stores each refinement as a
    /// separate item, so one relic yields four rows.
    pub refinement: Option<String>,
    /// Drop chance among the relic's rewards, in tenths of a percent.
    pub chance: u32,
    /// Quantity the reward gives; more than one for Forma and Kuva.
    pub count: u32,
}

/// A relic the account holds, with the expected yield of each refinement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Relic {
    /// Key shared by every refinement of the relic.
    pub base: String,
    pub name: String,
    pub image: Option<String>,
    /// First word of the relic's English name, lowercased: `lith`, `axi`, `requiem`.
    pub era: String,
    pub vaulted: bool,
    /// The refinements, intact first.
    pub steps: Vec<Step>,
    /// Rewards the relic can drop, in game order: most common first.
    pub holds: Vec<Slot>,
}

/// A relic at one refinement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Item key of this refinement, as the inventory names it.
    pub item: String,
    /// `intact`, `exceptional`, `flawless` or `radiant`.
    pub refinement: String,
    /// How many the account holds; zero for a refinement it holds none of.
    pub count: u32,
    /// Expected platinum per run of the relic, in tenths of a platinum.
    pub platinum: u32,
    /// Expected ducats per run, in tenths of a ducat.
    pub ducats: u32,
    /// Echoes it costs to bring an intact relic up to this refinement.
    pub echoes: u32,
}

/// One item a relic can drop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Slot {
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// `common`, `uncommon` or `rare`, as the drop tables grade it.
    pub rarity: String,
    /// Quantity the row gives; more than one for Forma.
    pub count: u32,
    /// How many of the item the account holds.
    pub own: u32,
    /// Market price of one copy in platinum; `None` while the market cache has no price.
    pub platinum: Option<u32>,
    pub ducats: Option<u32>,
    /// Odds of the row at each refinement, in the order of `steps`.
    pub odds: Vec<Odds>,
}

/// Drop chance of one relic reward row, and how often it is the squad's best draw. All three
/// fields are in tenths of a percent.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub struct Odds {
    /// Drop chance of this row among the relic's rewards.
    pub chance: u32,
    /// Share of runs in which this row is the squad's best draw by platinum.
    pub platinum: u32,
    /// Share of runs in which this row is the squad's best draw by ducats.
    pub ducats: u32,
}

/// A vendor that sells the item, and its price.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Sale {
    pub vendor: String,
    pub cost: Option<u32>,
    /// Currency key as the catalog spells it, mostly English; empty when the price is credits
    /// only.
    pub currency: String,
    /// Localized name of the currency, when the currency is an item.
    pub coin: Option<String>,
    pub credits: Option<u32>,
    /// Syndicate rank the offer requires; `None` when the vendor has no ranks.
    pub rank: Option<u32>,
    /// Key of the vendor's area; `None` when the catalog assigns none.
    pub area: Option<String>,
    /// Name of that area.
    pub place: Option<String>,
    /// The collection the vendor sells, when the item comes only as a random roll from it.
    pub pack: Option<String>,
    /// Whether the vendor visits periodically instead of being permanent.
    pub rotates: bool,
}

/// A location the item drops from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Drop {
    /// Catalog category of the location: `node`, `bounty`, `key`, `transient`, `sortie`.
    pub kind: String,
    pub place: String,
    /// Whether the location exists only while an event runs.
    pub event: bool,
    /// Planet of the location; `None` when the star chart has none.
    pub planet: Option<String>,
    /// Mission type there, when the star chart has one; for a bounty, the bounty activity
    /// (such as Cetus Bounties).
    pub mission: Option<String>,
    /// Who gives out the mission; `None` when nobody does.
    pub giver: Option<String>,
    pub level_from: Option<u32>,
    pub level_to: Option<u32>,
    /// Reward rotation of the drop; `None` when the table has no rotations.
    pub rotation: Option<String>,
    /// Mission stage of the drop; `None` when the table has no stages.
    pub stage: Option<String>,
    /// Drop-table rarity, lowercased and hyphenated: `rare`, `ultra-rare`, `very-common`.
    /// These are the catalog's grades, not a scale computed here.
    pub rarity: String,
    /// Chance within its drop table, in tenths of a percent. For an enemy this is the share of
    /// its mod table, not the chance per kill.
    pub chance: u32,
    /// Quantity per drop; `None` when the table does not state it.
    pub count: Option<u32>,
    /// Refinement the relic drops at here, when the item is a relic.
    pub refinement: Option<String>,
    /// Key of the area containing the location; `None` when the catalog assigns none.
    pub area: Option<String>,
    /// Planets where the enemy appears, most frequent first; empty for a location.
    pub haunts: Vec<String>,
}

/// A game region the player farms across several nodes, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Area {
    pub key: String,
    pub name: String,
    /// Star-chart nodes in the area, by their game names.
    pub nodes: Vec<String>,
}

/// One item a recipe consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Part {
    /// Item key, used to open that item's card.
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    pub count: u32,
    /// Catalog class slug; distinguishes a component from a resource.
    pub class: String,
}

/// A recipe that builds the item, one level deep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Recipe {
    /// Blueprint key, used to open the blueprint's card.
    pub blueprint: String,
    pub parts: Vec<Part>,
    pub credits: Option<u32>,
    /// Foundry time in seconds.
    pub time: Option<u32>,
    /// Platinum cost to rush the build.
    pub rush: Option<u32>,
}

/// A recipe that uses the item, and what that recipe builds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Need {
    /// Item key of the build result, used to open its card.
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// How many of this card's item the recipe uses.
    pub count: u32,
}

/// A clan lab the item's blueprint is researched in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Research {
    pub lab: String,
    pub credits: u32,
    /// Seconds the research takes.
    pub time: u32,
    /// Clan affinity the research costs.
    pub affinity: u32,
    /// Research that must be finished first, by its catalog name.
    pub prereq: Option<String>,
}

/// All catalog data on one item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub good: Good,
    pub sold: Vec<Sale>,
    /// Rewards of the relic, when the item is a relic.
    pub inside: Vec<Reward>,
    /// Relics that contain the item.
    pub found: Vec<Reward>,
    pub drops: Vec<Drop>,
    /// Clan labs the item is researched in; empty for everything the clan cannot build.
    pub research: Vec<Research>,
    /// Recipes that build the item; a few items have more than one.
    pub recipes: Vec<Recipe>,
    /// Recipes that use the item.
    pub needed_by: Vec<Need>,
    /// The easiest way to obtain the item; `None` when the catalog lists none.
    pub way: Option<Way>,
}

/// A top-level item group of the catalog, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Class {
    pub slug: String,
    pub name: String,
    /// Position of the class in the catalog's order.
    pub ord: u32,
}

/// A kind of item, and the class it belongs to, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Kind {
    pub slug: String,
    pub class: String,
    pub name: String,
    pub ord: u32,
}

/// The catalog's item classes and kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Taxonomy {
    pub classes: Vec<Class>,
    pub kinds: Vec<Kind>,
}

/// A set of parts and what they build into, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SetRow {
    pub slug: String,
    pub name: String,
    /// Ducat value of the whole set, from the catalog.
    pub ducats: Option<u32>,
    /// Whether the set is vaulted.
    pub vaulted: Option<bool>,
    /// Item key the parts build into; `None` when the catalog names none.
    pub built: Option<String>,
    /// Item keys of the parts, in the catalog's own order.
    pub members: Vec<String>,
    /// How many of each part one set takes, in the order of `members`.
    pub per_set: Vec<u32>,
}

/// A star-chart node referenced by the world state, with its catalog names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub node: String,
    pub name: String,
    pub planet: String,
    pub mission: String,
    /// Mission type in the catalog's English spelling, without spaces.
    pub mission_key: String,
}

/// Which part of a game data release is being downloaded, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CatalogPart {
    Database,
    Pictures,
}

/// Current game data activity, wire form. Byte counts are `u32`: specta does not export
/// 64-bit integers, and JavaScript would receive them as `bigint`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CatalogActivity {
    Idle,
    Checking,
    Fetching {
        part: CatalogPart,
        done: u32,
        total: u32,
    },
    Opening,
    Unreachable {
        reason: String,
    },
    Unsupported {
        schema: u32,
    },
    Stale {
        schema: u32,
    },
}

/// Installation state of the game data (catalog and pictures), wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Catalog {
    /// Whether both the catalog and its pictures are installed, so the interface can open.
    pub ready: bool,
    /// Version of the open catalog; `None` when none is open.
    pub installed: Option<String>,
    /// Version whose pictures are complete on disk.
    pub pack: Option<String>,
    /// Version being installed.
    pub pending: Option<String>,
    pub activity: CatalogActivity,
}

/// The open game data, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    /// Version of the data, such as `19.4.1`.
    pub version: String,
    /// When Median-Data fetched the sources the data was made from, Unix seconds; `None` when
    /// the data does not say.
    pub fetched: Option<Instant>,
    /// Game build the data was made from, such as `2026.10.28`; `None` when the data does not
    /// say.
    pub game_build: Option<String>,
}

/// Connection state of the Overwolf collector, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Collector {
    /// No envelope received within the server's timeout.
    Offline,
    /// Connected; `heard` is when the last envelope arrived, `feeds` what the collector last
    /// reported about Overwolf's game-event feeds.
    Live { heard: Instant, feeds: Feeds },
    /// The connected collector uses another protocol version, given in `speaks`.
    Outdated { speaks: u32 },
}

/// Overwolf's game-event feeds (GEP) for Warframe, as the collector reports them, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Feeds {
    /// Not reported yet, or requested and not up yet.
    Waiting,
    /// Up: game data flows.
    Up,
    /// Overwolf refused them with `reason`; the collector keeps asking.
    Refused { reason: String },
}

/// This build: revision, commit, build time and channel, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    /// Number of commits in the branch the build was made from; 0 outside a git repository.
    pub revision: u32,
    /// Short hash of the build's commit, 9 characters; empty outside a git repository.
    pub hash: String,
    /// Compile time, in Unix seconds.
    pub built: Instant,
    /// Release channel the build was made for.
    pub channel: ReleaseChannel,
    /// Whether the build takes updates.
    pub updates: bool,
    /// What changed since the previous release, one change per entry; empty for a local build.
    pub changes: Vec<String>,
}

/// A release the updater found to install, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    /// Revision of the release; `None` when its version does not read as one.
    pub revision: Option<u32>,
    /// Version as the release announces it.
    pub version: String,
    /// Channel the release came from.
    pub channel: ReleaseChannel,
    /// What changed in the release, one change per entry.
    pub changes: Vec<String>,
    /// Publication time, Unix seconds; `None` when the announcement gives none.
    pub published: Option<Instant>,
    /// Whether installed builds must take the release before going on.
    pub required: bool,
}

/// Why median did not start, wire form; the launcher shows it and still offers updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StartFailure {
    /// The error as the log has it.
    pub reason: String,
    /// Whether the account data comes from a later build.
    pub newer_data: bool,
    /// Interface language from the saved settings.
    pub locale: String,
}

/// Release channel of a build, wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseChannel {
    /// Released from `main`.
    Stable,
    /// Released from `preview`, ahead of the stable channel.
    Preview,
    /// Built locally, not released.
    Dev,
}

/// Status of the market cache and of the warframe.market connection, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    /// How many times the account's own orders have been read from the market.
    pub own: u32,
    /// When warframe.market stopped responding to every request; `None` while it responds.
    pub failing_since: Option<Instant>,
}

/// One open order on an item, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    /// The player's in-game name.
    pub name: String,
    /// Whether the player can be traded with right now.
    pub reachable: bool,
    /// The player's status as warframe.market reports it.
    pub status: String,
    pub selling: bool,
    /// Price of one copy, in platinum.
    pub platinum: u32,
    /// Quantity on the order.
    pub quantity: u32,
    /// Copies per trade; some items are sold only in lots.
    pub per_trade: u32,
    pub rank: Option<u8>,
    pub subtype: Option<String>,
    /// In-game whisper text that starts the trade, in the other player's language.
    pub whisper: String,
    /// The player's reputation on the market.
    pub reputation: i32,
    /// When the order last changed; zero when the market's timestamp cannot be parsed.
    pub updated: Instant,
}

/// Open orders on one item, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Orders {
    pub rows: Vec<Listing>,
    /// Maximum rank of the item; `None` when it has no ranks.
    pub max_rank: Option<u8>,
    /// Whether the market allows the item to be traded in lots.
    pub bulk: bool,
}

/// A warframe.market item slug and one of its variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Pair {
    pub slug: String,
    /// warframe.market variant key; empty when the item has a single price.
    pub key: String,
}

/// Market prices of one inventory entry, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Priced {
    /// Game key of the item (a `/Lotus/…` path).
    pub item: String,
    /// Variant of this copy in the game's notation; empty when the item has no variants.
    pub variant: String,
    /// Lowest price among sellers currently in game, in platinum.
    pub ingame: Option<u32>,
    /// Seconds since the order book was last fetched.
    pub looked: Option<u32>,
    /// Seconds since closed trades were last fetched.
    pub asked: Option<u32>,
    /// Copies traded over the last 48 hours.
    pub traded: Option<u32>,
    /// Maximum rank of the item; `None` when it has no ranks.
    pub max_rank: Option<u8>,
    /// Price of a copy at max rank, when the item has ranks. An arcane at rank 0 and one at
    /// rank 5 differ widely in price, and the max-rank price is the copy's real value; a copy
    /// already at max rank gets its own price.
    pub topped: Option<u32>,
    /// warframe.market slug of this item.
    pub slug: String,
    /// warframe.market variant key; `variant` holds the game's notation, and the two use
    /// different names.
    pub key: String,
    /// Value of one copy: the median closed-trade price, else the lowest price among sellers
    /// in game.
    pub worth: Option<u32>,
}

/// One point of a price curve, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Spot {
    pub at: Instant,
    pub platinum: f32,
    /// Closed trades in the bucket, or order-book snapshots taken that day.
    pub weight: u32,
}

/// Daily summary of the open orders on one side of the market, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct StandingDay {
    pub at: Instant,
    /// Lowest sell price, or highest buy price.
    pub best: f32,
    pub median: f32,
    pub orders: u32,
}

/// Price history of one item over a period, as the series the chart draws, wire form.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Curve {
    /// Closed trades.
    pub closed: Vec<Spot>,
    pub selling: Vec<StandingDay>,
    pub buying: Vec<StandingDay>,
    /// Maximum rank; `None` when the item has no ranks.
    pub max_rank: Option<u8>,
}

/// Status of the account the collector reports on, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Account {
    /// Account name; `None` until an inventory dump identifies the account.
    pub profile: Option<String>,
    /// Inventory entries in the last readable dump.
    pub positions: u32,
    /// Inventory dumps read since the application started.
    pub read: u32,
    /// Of those, dumps whose inventory differed from the previous dump.
    pub distinct: u32,
    /// Dumps received that could not be parsed.
    pub unreadable: u32,
    /// Dumps that parsed but did not identify the account.
    pub unidentified: u32,
    /// Trades left for the account today, as of the last dump.
    pub exchanges: Option<u32>,
    /// Platinum the account holds.
    pub platinum: Option<u32>,
    /// Credits the account holds.
    pub credits: Option<u32>,
    /// Seconds since a dump last changed anything; the counts above are that old.
    pub moved: Option<u32>,
}

/// Variant details of a copy, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Detail {
    /// A mod at this rank.
    Rank { rank: u32 },
    /// An ayatan sculpture with this many stars set into it.
    Stars { stars: u32 },
    /// A riven at this rank, with its rolled stats once it is revealed.
    Riven { rank: u32, revealed: Option<Riven> },
}

/// Polarity of a riven, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Polarity {
    Madurai,
    Vazarin,
    Naramon,
}

/// One stat a riven rolled, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RivenStat {
    /// Game key of the stat.
    pub tag: String,
    /// Position of the roll within the stat's range, as a fraction of 2³⁰.
    pub roll: u32,
}

/// A revealed riven, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Riven {
    /// Game key of the weapon the riven is for.
    pub weapon: String,
    /// Mastery rank required to equip it.
    pub mastery: u32,
    /// How many times it has been rerolled.
    pub rerolls: u32,
    pub polarity: Option<Polarity>,
    /// Positive stats.
    pub buffs: Vec<RivenStat>,
    /// Negative stats.
    pub curses: Vec<RivenStat>,
}

/// How useful a riven stat is on its weapon, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StatVerdict {
    /// A buff that at least one good roll requires.
    Decisive,
    /// A buff that at least one good roll allows.
    Wanted,
    /// A buff no good roll includes, or a curse on a stat no good roll uses.
    Useless,
    /// A curse the good rolls list as harmless.
    Harmless,
    /// A curse on a stat that at least one good roll uses.
    Harmful,
}

/// How the game formats a stat value, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum StatForm {
    /// `+274.4%`.
    Percent,
    /// `x1.58`.
    Multiplier,
    /// `+2.7`.
    Plain,
}

/// One stat of an appraised riven, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppraisedStat {
    /// Game key of the stat.
    pub tag: String,
    pub is_curse: bool,
    /// Stat label the game shows after the number.
    pub label: String,
    pub form: StatForm,
    /// Stat value as a fraction: `2.744` means +274.4%.
    pub amount: f64,
    /// Position of the roll within its range, from 0 (minimum) to 1 (maximum).
    pub roll_share: f64,
    /// `None` when the weapon has no known good rolls.
    pub verdict: Option<StatVerdict>,
}

/// Chance that a reroll gives a better grade with one lock choice, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RerollOdds {
    /// Tag of the stat kept through the reroll; `None` when nothing is locked.
    pub locked: Option<String>,
    pub chance: f64,
    /// Kuva expected to be spent until a reroll gives a better grade; `None` when no reroll
    /// ever does.
    pub expected_kuva: Option<f64>,
    /// Average price of the rerolls that give a better grade; `None` when none does or none is
    /// priced.
    pub better_mean: Option<f64>,
}

/// Source of a riven's price band, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BandBasis {
    /// Enough similar auctions: their quartiles, outliers removed.
    Lots,
    /// A few similar auctions: their median, kept within the range of last week's trades.
    FewLots,
    /// Too few similar auctions: last week's trades for the weapon.
    Trades,
}

/// Price range of rivens similar to a given one, in platinum, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RivenBand {
    pub low: f64,
    pub middle: f64,
    pub high: f64,
    /// Similar auctions the band is computed from.
    pub lots: u32,
    pub basis: BandBasis,
    /// Whether last week's trades capped the band.
    pub is_capped: bool,
}

/// One stat of a listed riven as its seller entered it, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ListedStat {
    /// Game key of the stat.
    pub tag: String,
    /// Stat label the game shows after the number.
    pub label: String,
    pub form: StatForm,
    /// The value entered, as a fraction: `2.744` means +274.4%.
    pub amount: f64,
    pub is_curse: bool,
}

/// A riven auction similar to a given riven, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RivenAnalog {
    /// warframe.market auction id.
    pub id: String,
    pub price: u32,
    pub rank: u32,
    pub rerolls: u32,
    pub stats: Vec<ListedStat>,
    /// Seller's in-game name.
    pub seller: Option<String>,
    /// Seller status: `ingame`, `online` or `offline`.
    pub status: Option<String>,
    /// Share of stats it has in common with the riven, counting a stat only when it is a buff
    /// on both or a curse on both.
    pub likeness: f64,
}

/// One of the account's riven auctions, parsed back into a riven, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OwnAuction {
    /// warframe.market auction id.
    pub id: String,
    /// Game key of the riven's weapon.
    pub weapon: Option<String>,
    /// The parsed riven; `None` when the auction cannot be parsed into one.
    pub riven: Option<Riven>,
    /// Riven name as warframe.market shows it.
    pub name: String,
    pub rank: u32,
    pub starting_price: u32,
    /// Buyout price; `None` when only bids can end the auction.
    pub buyout_price: Option<u32>,
    pub top_bid: Option<u32>,
    /// Whether it sells at a fixed price rather than to the highest bid.
    pub is_direct_sell: bool,
    pub visible: bool,
    /// Creation time, as the market's timestamp string.
    pub created: String,
}

/// What a riven is worth if sold or if used in a transmutation, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct RivenChoices {
    /// Prices sellers ask for similar rivens.
    pub sell: Option<RivenBand>,
    /// A quarter of last week's trade price of a veiled riven of its class, since a
    /// transmutation consumes four rivens.
    pub transmute: Option<f64>,
    /// Similar auctions: most similar first, cheapest first among equals.
    pub analogs: Vec<RivenAnalog>,
}

/// How actively rivens for a weapon trade, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RivenDemand {
    Lively,
    Slow,
    Dead,
    /// Not enough data to judge.
    Unclear,
}

/// Grade of a riven's stats for its weapon, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RivenGrade {
    Bad,
    Potential,
    Good,
    Perfect,
}

/// Grade and per-stat appraisal of a riven for its weapon, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct RivenAppraisal {
    /// `None` when the weapon has no known good rolls.
    pub grade: Option<RivenGrade>,
    /// Buffs first, then curses.
    pub stats: Vec<AppraisedStat>,
}

/// One inventory entry, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Piece {
    /// Game key of the item (a `/Lotus/…` path).
    pub item: String,
    /// Variant that distinguishes this copy from others of the same item; empty when there is
    /// none.
    pub variant: String,
    /// Quantity held; zero means the entry was removed.
    pub count: u32,
    pub detail: Option<Detail>,
    /// Whether a copy of this entry is installed on something in the game.
    pub equipped: bool,
}

/// The account's whole inventory, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Holdings {
    /// Number of inventory changes since the application started.
    pub step: u32,
    /// Digest of the inventory this list represents.
    pub digest: String,
    pub pieces: Vec<Piece>,
}

/// State of a background task, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TaskState {
    /// The task is running.
    Running,
    /// The task ended and will be started again.
    Restarting { reason: String },
    /// The task ended and will not be started again.
    Stopped { reason: String },
}

/// Last reported state of a background task, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Task {
    pub name: String,
    pub state: TaskState,
}

/// An item's localized name and image id, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Icon {
    /// Item name in the requested locale.
    pub name: String,
    /// Image id inside the image pack.
    pub image: String,
}

/// warframe.market sign-in state and the status the account shows there, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Membership {
    /// `out`, `in` or `expired`.
    pub standing: String,
    /// warframe.market username; `None` unless signed in.
    pub name: Option<String>,
    /// `online`, `ingame` or `invisible`.
    pub status: String,
    pub auto: bool,
}

/// An item bought for resale, and its current market state, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    /// Game key of the item; empty when the slug maps to no game item.
    pub item: String,
    pub slug: String,
    pub name: String,
    pub image: Option<String>,
    /// Variant in warframe.market notation; empty when the item has none.
    pub variant: String,
    /// Maximum rank of the item; `None` when it has no ranks.
    pub max_rank: Option<u8>,
    pub quantity: u32,
    /// Purchase price of one copy, in platinum.
    pub basis: u32,
    /// Lowest price among sellers currently in game.
    pub ingame: Option<u32>,
    /// Seconds since the order book was last fetched.
    pub looked: Option<u32>,
    /// Whole days the position has been open.
    pub days: u32,
    /// Price of the account's own sell order; `None` when it has no open one.
    pub asking: Option<u32>,
}

/// One of the account's own orders on warframe.market, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OwnOrder {
    pub id: String,
    /// Game key of the item; empty when the slug maps to no game item.
    pub item: String,
    /// warframe.market slug; empty when no slug maps to the item.
    pub slug: String,
    pub name: String,
    pub image: Option<String>,
    /// Variant in warframe.market notation; empty when the item has none.
    pub variant: String,
    /// Maximum rank of the item; `None` when it has no ranks.
    pub max_rank: Option<u8>,
    pub selling: bool,
    pub platinum: u32,
    pub quantity: u32,
    /// Copies the account actually holds; `None` when no inventory dump reports it.
    pub owned: Option<u32>,
    pub per_trade: u32,
    pub visible: bool,
    /// Whether the agent placed it.
    pub by_agent: bool,
    /// Price this order is compared with: the lowest reachable seller for a sell order, the
    /// highest reachable buyer for a buy order.
    pub reference: Option<u32>,
    /// Purchase price of one copy, when the order sells a position the account holds.
    pub basis: Option<u32>,
    /// Median closed-trade price.
    pub ours: Option<u32>,
    /// Highest price the account's bid may reach: the resale price less the profit the policy
    /// requires. `None` on a sell order, or while the market cache has no resale price.
    pub most: Option<u32>,
    /// Expected resale price of a copy bought through this bid. `None` on a sell order, or
    /// while the market cache has no price.
    pub resale: Option<u32>,
}

/// One order from the order stream and the agent's decision on it, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Seen {
    pub at: Instant,
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    /// Catalog category of the item; empty when the catalog gives it no kind.
    pub category: String,
    /// Price one copy would cost: the seller's ask, or one platinum above the highest bid when
    /// the agent would outbid the buyers.
    pub entry: u32,
    /// Whether the opportunity came from a buy order rather than a sell order.
    pub outbid: bool,
    pub worth: Option<u32>,
    /// Key of the reason the agent rejected the order; `None` when the agent accepted it.
    pub passed: Option<String>,
}

/// Agent state: whether it runs, its counters since start, and the orders it judged last, wire
/// form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    /// Whether the agent is running. A running agent places and changes orders; there is no
    /// third state.
    pub running: bool,
    /// Orders from the order stream that were judged far enough to get a verdict.
    pub seen: u32,
    /// Of `seen`, the ones the policy accepted.
    pub taken: u32,
    /// Orders received from the order stream since the application started.
    pub heard: u32,
    /// Of `heard`, the ones whose counterparty cannot be traded with right now.
    pub unreachable: u32,
    /// Of `taken`, the ones placed as an order.
    pub placed: u32,
    /// Times one of the account's open orders was repriced or withdrawn.
    pub moved: u32,
    /// Watched items whose order book is fresher than the refresh limit for watched pairs.
    pub warm: u32,
    /// Items the heat map watches; zero until it has been counted.
    pub hot: u32,
    /// The latest judged orders, newest first; at most 40.
    pub flow: Vec<Seen>,
    /// Riven auctions picked to be bought, newest first.
    pub picks: Vec<RivenPick>,
}

/// A riven auction the agent picked to be bought, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RivenPick {
    /// warframe.market auction id.
    pub id: String,
    pub at: u32,
    /// Riven name as warframe.market shows it.
    pub name: String,
    pub riven: Riven,
    pub rank: u32,
    pub seller: String,
    /// `ingame`, `online` or `offline`.
    pub status: String,
    /// `cheap` or `unrolled`.
    pub reason: String,
    pub buyout: u32,
    /// Estimated resale value, in platinum.
    pub worth: u32,
    /// Opening offer to the seller, in platinum.
    pub offer: u32,
    /// Message to the seller, ready to paste.
    pub whisper: String,
}

/// A riven bought for resale, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RivenBuy {
    /// warframe.market id of the auction it was bought from.
    pub auction: String,
    pub name: String,
    pub riven: Riven,
    pub rank: u32,
    pub price: u32,
    pub bought: u32,
    /// When it sold; `None` while still held.
    pub sold: Option<u32>,
    /// Sale price, in platinum.
    pub sale: Option<u32>,
}

/// Changes to one open order; an omitted field stays unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Amend {
    pub platinum: Option<u32>,
    pub quantity: Option<u32>,
    pub visible: Option<bool>,
}

/// Sound alert settings for in-game events, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    /// Whether an in-game whisper plays a sound.
    pub whisper: bool,
    /// Volume in percent, 0 to 100.
    pub volume: u8,
}

/// An in-game whisper the account received, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Whisper {
    pub player: String,
    pub at: Instant,
}

/// One line of an in-game trade, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SwapPart {
    /// Catalog name, or the game's own name when the catalog has no match.
    pub name: String,
    pub count: u32,
    /// `item`, `platinum` or `credits`.
    pub kind: String,
    /// Rank of the copy; `None` when it has no rank.
    pub rank: Option<u8>,
}

/// One in-game trade confirmed in the game log, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Swap {
    pub at: Instant,
    pub partner: String,
    /// What the account gave.
    pub given: Vec<SwapPart>,
    /// What the account received.
    pub taken: Vec<SwapPart>,
    /// `done`, `failed` or `cancelled`.
    pub outcome: String,
    /// Platinum transferred in the trade, in either direction.
    pub platinum: u32,
    /// Whether the account paid platinum and received something else.
    pub buying: bool,
}

/// Game state read from the game log and from screen captures, wire form.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    /// Whether the game log is being read.
    pub listening: bool,
    /// Whether another application also has the game log open.
    pub shared: bool,
    /// Received whispers, newest first.
    pub whispers: Vec<Whisper>,
    /// Trades the game log confirmed, newest first.
    pub swaps: Vec<Swap>,
    /// The relic reward screen as the overlay shows it.
    pub rewards: Rewards,
    /// The riven cycling screen as the overlay shows it.
    pub cycling: Cycling,
    /// Star-chart node the game last loaded; `None` on the ship, at a camp, or before any.
    pub node: Option<String>,
    /// Unix seconds the game window was first seen in this run of the game; `None` while the game
    /// is not running.
    pub running_since: Option<Instant>,
    /// Whether reward cards can be read.
    pub recognition: Recognition,
}

/// State of the model that reads reward card names off the screen, wire form.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Recognition {
    /// Not loaded yet.
    #[default]
    Unchecked,
    /// Loaded; reward cards are read.
    Ready,
    /// The model embedded in the build did not load; reward cards are not read.
    Failed,
}

/// The relic reward screen, wire form.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Rewards {
    #[default]
    Gone,
    /// The screen is open and its cards are still being read.
    Reading,
    /// The cards read; `fresh` once their prices were fetched from the market again.
    Read { cards: Vec<Offered>, fresh: bool },
}

/// The riven cycling screen, wire form.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Cycling {
    #[default]
    Gone,
    /// The screen is open: the cards read from it and the cycles made since it opened.
    Open {
        cards: Vec<CycleCard>,
        rolls: Vec<CycleRoll>,
        /// Whether new cards are being read.
        reading: bool,
        /// Why the last cards could not be parsed into rivens: `noReader`, `notHeld`,
        /// `unsupported` or `stats`.
        unread: Option<String>,
    },
}

/// One card of the cycling screen, parsed into a riven, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CycleCard {
    /// `chosen`, `left` or `right`.
    pub side: String,
    pub riven: Riven,
    pub rank: u32,
    pub name: String,
    /// Whether it is the riven the last cycle rolled.
    pub fresh: bool,
}

/// One riven cycle performed on the screen, wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CycleRoll {
    pub kuva: u32,
    /// Name of the riven it rolled; `None` until it is read.
    pub name: Option<String>,
}

/// One part of the set a reward belongs to, and how many of it the account holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Owned {
    pub image: Option<String>,
    pub count: u32,
    /// Copies one set requires.
    pub need: u32,
    /// Whether this is the part shown on the reward screen.
    pub this: bool,
}

/// The account's progress on the item a reward's set builds into, wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Reached {
    /// Held or finished in the foundry, not yet at max rank.
    Built,
    /// Levelled to max rank.
    Mastered,
}

/// One reward card shown on the relic reward screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Offered {
    /// Whether the reward, or the item its set builds into, is marked a favourite.
    pub favourite: bool,
    /// Catalog key; empty when the card was not recognised.
    pub item: String,
    /// Display name: the catalog's, or the text read from the screen when nothing matched.
    pub name: String,
    pub image: Option<String>,
    /// Price estimated from closed trades; `None` when the market cache has too few of them.
    pub platinum: Option<u32>,
    /// Lowest price among reachable sellers, from the last order book in the market cache.
    pub ingame: Option<u32>,
    /// Copies traded per day, in tenths; `None` while the market cache has no data.
    pub per_day_tenths: Option<u32>,
    pub ducats: Option<u32>,
    /// Whether this is the reward from the account's own relic, which the game marks
    /// explicitly.
    pub own: bool,
    /// Parts of the reward's set, in catalog order; empty when it belongs to no set.
    pub kit: Vec<Owned>,
    /// Value of the whole set; `None` when the reward belongs to no set or the market cache
    /// has no price.
    pub whole: Option<u32>,
    /// The account's progress on the item the set builds into; `None` when it has neither
    /// built nor mastered it, or the reward belongs to no set.
    pub reached: Option<Reached>,
}

/// One order the user asks to place, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Posting {
    /// Game key of the item (a `/Lotus/…` path).
    pub item: String,
    pub selling: bool,
    pub platinum: u32,
    pub quantity: u32,
    pub rank: Option<u8>,
    pub subtype: Option<String>,
    /// Copies per trade for an item sold in lots; `None` for an item sold one at a time.
    pub per_trade: Option<u32>,
}

/// One trade as recorded in the account store, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Dealt {
    pub at: Instant,
    /// Game key of the item; empty when the slug maps to no game item.
    pub item: String,
    pub name: String,
    pub image: Option<String>,
    pub selling: bool,
    /// Price per copy, in platinum.
    pub platinum: u32,
    pub quantity: u32,
    /// Whether it filled an order the agent placed.
    pub agent: bool,
    /// Trading category: `riven`, `relic`, `arcane`, `mod`, `set`, `prime`, or empty.
    pub group: String,
    /// Profit of a sale over the average cost of its copies, in platinum; `None` for a purchase.
    pub margin: Option<i32>,
}

/// The account's standing with one syndicate, and what its vendors sell for standing, wire
/// form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Syndicate {
    /// Game tag of the syndicate.
    pub tag: String,
    /// Whether the account has met the syndicate at all.
    pub met: bool,
    /// Spendable standing, not counting the standing the reached ranks used up.
    pub standing: i32,
    /// Rank with the syndicate; below zero while it is hostile.
    pub title: i32,
    /// Standing still earnable today; `None` until an inventory dump of this session reports it.
    pub left_today: Option<u32>,
    /// What its vendors sell for standing.
    pub offers: Vec<StandingOffer>,
}

/// One item a syndicate vendor sells for standing, with its market data, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StandingOffer {
    /// Game key of the item (a `/Lotus/…` path).
    pub item: String,
    /// warframe.market slug; `None` when the market does not trade it.
    pub slug: Option<String>,
    pub name: String,
    pub image: Option<String>,
    /// Vendor name in the requested language.
    pub vendor: String,
    /// Vendor location; `None` for a vendor found on every relay.
    pub place: Option<String>,
    /// Standing cost of one copy.
    pub cost: u32,
    /// Syndicate rank the vendor requires; `None` when it requires none.
    pub rank: Option<i32>,
    /// Whether players can trade it.
    pub tradable: bool,
    /// Expected platinum per copy when sold within the allotted sale period.
    pub platinum: Option<u32>,
    /// Lowest price among sellers reachable right now.
    pub ask: Option<u32>,
    /// Copies traded per day.
    pub per_day: Option<f32>,
    /// Expected days to sell a copy listed at `ask`.
    pub days: Option<f32>,
    /// Copies the account already holds.
    pub owned: u32,
}

/// Items the account obtained since the haul started, wire form.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Haul {
    /// When the haul started; `None` until an inventory reading sets the baseline it counts
    /// from.
    pub started: Option<Instant>,
    pub lines: Vec<HaulLine>,
}

/// Copies of one item the account obtained during the haul, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HaulLine {
    /// Game key of the item (a `/Lotus/…` path).
    pub item: String,
    /// Variant of the copies in the game's notation; empty when there is none.
    pub variant: String,
    pub count: u32,
    pub name: String,
    pub image: Option<String>,
    /// Whether the catalog has the item; an item it lacks has no price.
    pub known: bool,
    /// Ducat value of one copy.
    pub ducats: Option<u32>,
    /// Market price of one copy; `None` while the market cache has no price.
    pub worth: Option<u32>,
}

/// Trades of one period, wire form.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Figures {
    /// Every trade of the period, newest first.
    pub deals: Vec<Dealt>,
}

/// Results of the agent's trading, wire form.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Scorecard {
    pub fills: Vec<DepthFills>,
    pub per_exchange: Option<f64>,
    pub burned: Vec<BurnedDay>,
}

/// Fill statistics of the agent's bids at one depth below the median trade price, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DepthFills {
    /// Depth below the median, in tenths of the median; 5 covers half and deeper.
    pub tenths: u8,
    /// Intervals in which a bid stayed at one price at this depth.
    pub stretches: u32,
    pub filled: u32,
    /// Chance that a bid at this depth is filled within a day.
    pub within_day: f64,
}

/// Trades left unused at the end of one finished day, wire form.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BurnedDay {
    pub day: Instant,
    pub exchanges: u32,
}

/// One whitelist entry, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Capped {
    pub item: String,
    pub most: u32,
}

/// Limits the agents trade within, wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub least_profit: u32,
    pub least_per_day: u32,
    pub most_per_position: u32,
    pub most_tied: u32,
    /// Whether `most_tied` is replaced by a share of the platinum actually held.
    pub tied_from_purse: bool,
    /// Percent of held platinum the agent may tie up, when `tied_from_purse` is set.
    pub purse_share: u32,
    /// Trades left in the day at which the agent stops; the remaining trades are the user's.
    pub least_exchanges: u32,
    pub most_per_trade: u32,
    /// Maximum quantity of one bid; the available platinum may lower it.
    pub most_per_order: u32,
    /// Days a copy is listed at the ask before it is sold to the highest bid.
    pub sale_days: u32,
    pub blacklist: Vec<String>,
    pub whitelist: Vec<Capped>,
    /// Whether the agent also picks riven auctions to buy.
    pub rivens: bool,
    /// Whether the agent buys mods at rank zero to sell them ranked up.
    pub rank_up: bool,
    /// Maximum total platinum in rivens.
    pub riven_capital: u32,
    /// Percent of held platinum for rivens, when the limit is derived from the purse.
    pub riven_share: u32,
    /// Minimum discount of a riven auction below its estimated value, in percent.
    pub riven_margin: u32,
    /// Minimum profit of a riven auction over its buyout, in platinum.
    pub riven_least_profit: u32,
}

/// UI-facing snapshot of application state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    /// Active UI locale, as a language tag.
    pub locale: String,
    /// Name of the interface accent colour.
    pub accent: String,
    /// Latest world state, once fetched.
    pub worldstate: Option<Worldstate>,
    /// Current stock of the visiting traders.
    pub shelves: Vec<Shelf>,
    /// Count of inputs rejected at the boundary.
    pub quarantined: u32,
    /// Installed game data and the current install activity.
    pub catalog: Catalog,
    /// Collector connection state.
    pub collector: Collector,
    /// Last reported state of each background task.
    pub tasks: Vec<Task>,
    /// Status of the account the collector reports on.
    pub account: Account,
    /// Market cache status.
    pub market: Market,
    /// Limits the agents trade within.
    pub policy: Policy,
    /// Agent state and counters.
    pub agent: Agent,
    pub membership: Membership,
    /// Game state read from the game log and from screen captures.
    pub game: Game,
    /// Sound alert settings.
    pub alert: Alert,
    /// World events the user is notified about.
    pub watches: Vec<Watch>,
    /// Release channel updates come from, as the user chose it; `None` for the build's own.
    pub update_channel: Option<ReleaseChannel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_posting_reads_the_names_the_window_writes() {
        let json = r#"{"item":"/Lotus/Weapons/Gram","selling":true,"platinum":140,
            "quantity":1,"rank":null,"subtype":"radiant","perTrade":6}"#;

        let posting: Posting = serde_json::from_str(json).expect("a posting parses");

        assert_eq!(posting.item, "/Lotus/Weapons/Gram");
        assert!(posting.selling);
        assert_eq!(posting.platinum, 140);
        assert_eq!(posting.subtype.as_deref(), Some("radiant"));
        assert_eq!(posting.rank, None);
        assert_eq!(posting.per_trade, Some(6));
    }
}
