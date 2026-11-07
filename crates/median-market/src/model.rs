use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::moment;

/// Which variant of an item to read out of its order book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// The item has no variants.
    Whole,
    /// Mods and arcanes.
    Rank(u8),
    /// Relics, rivens and others.
    Subtype(String),
    /// Items distinguished by rank and subtype together, such as a mod the market also lists as
    /// a print.
    Both { rank: u8, subtype: String },
}

/// Which side of the market a row is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Sell,
    Buy,
}

impl Side {
    /// The market's name for this side: `sell` or `buy`.
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Sell => "sell",
            Side::Buy => "buy",
        }
    }

    /// The side the market names `word`; `None` for anything but `sell` or `buy`.
    pub fn from_name(word: &str) -> Option<Self> {
        match word {
            "sell" => Some(Side::Sell),
            "buy" => Some(Side::Buy),
            _ => None,
        }
    }
}

/// A player's status on warframe.market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Presence {
    Ingame,
    Online,
    Offline,
    /// A status this client does not know, such as `invisible`.
    #[serde(other)]
    Unknown,
}

impl Presence {
    /// Whether a trade with this player is possible right now: only while the player is in game.
    pub fn reachable(self) -> bool {
        matches!(self, Self::Ingame)
    }
}

/// The player who owns an order.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trader {
    #[serde(rename = "ingameName")]
    pub name: String,
    pub status: Presence,
    #[serde(default)]
    pub reputation: i32,
    #[serde(default)]
    pub last_seen: Option<String>,
    /// The player's language as the market reports it; `None` when not given.
    #[serde(default)]
    pub locale: Option<String>,
}

/// One open order.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: String,
    #[serde(rename = "type")]
    pub side: Side,
    pub platinum: u32,
    pub quantity: u32,
    #[serde(default)]
    pub per_trade: Option<u32>,
    #[serde(default)]
    pub visible: Option<bool>,
    #[serde(default)]
    pub rank: Option<u8>,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub amber_stars: Option<u8>,
    #[serde(default)]
    pub cyan_stars: Option<u8>,
    /// RFC 3339 timestamp, unparsed.
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub item_id: Option<String>,
    pub user: Trader,
    /// Fields the market sent that this client does not model.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One entry of the market's full item list: the item's market id, slug and game key.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    pub id: String,
    pub slug: String,
    /// Game key of the item; `None` for a few market-only entries.
    #[serde(default)]
    pub game_ref: Option<String>,
}

/// Market metadata of an item, including the variants it declares (ranks, subtypes, stars).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemMeta {
    pub id: String,
    pub slug: String,
    #[serde(default)]
    pub game_ref: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub tradable: bool,
    /// Whether an order on this item may set a lot size; `None` means it may not.
    #[serde(default)]
    pub bulk_tradable: Option<bool>,
    #[serde(default)]
    pub trading_tax: Option<u32>,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub req_mastery_rank: Option<u8>,
    #[serde(default)]
    pub ducats: Option<u32>,
    #[serde(default)]
    pub vaulted: Option<bool>,
    #[serde(default)]
    pub set_parts: Option<Vec<String>>,
    #[serde(default)]
    pub quantity_in_set: Option<u32>,
    // The variants the item can have.
    #[serde(default)]
    pub max_rank: Option<u8>,
    #[serde(default)]
    pub subtypes: Option<Vec<String>>,
    #[serde(default)]
    pub max_amber_stars: Option<u8>,
    #[serde(default)]
    pub max_cyan_stars: Option<u8>,
}

/// One bucket of closed-trade statistics, with the fields as the market sent them.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ClosedBucket {
    pub datetime: String,
    pub volume: u64,
    pub min_price: f64,
    pub max_price: f64,
    #[serde(default)]
    pub open_price: Option<f64>,
    #[serde(default)]
    pub closed_price: Option<f64>,
    pub avg_price: f64,
    #[serde(default)]
    pub wa_price: Option<f64>,
    pub median: f64,
    #[serde(default)]
    pub moving_avg: Option<f64>,
    #[serde(default)]
    pub donch_top: Option<f64>,
    #[serde(default)]
    pub donch_bot: Option<f64>,
    /// The market's own id for the bucket.
    #[serde(default)]
    pub id: Option<String>,
    // Which variant field appears depends on the item.
    #[serde(default)]
    pub mod_rank: Option<u8>,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub amber_stars: Option<u8>,
    #[serde(default)]
    pub cyan_stars: Option<u8>,
    /// Fields the market sent that this client does not model.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ClosedBucket {
    /// Start of the bucket in Unix seconds; `None` when `datetime` does not parse.
    pub fn starts_at(&self) -> Option<i64> {
        moment::seconds(&self.datetime)
    }
}

/// One bucket of open-order statistics, one row per side; `volume` counts orders, not trades.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LiveBucket {
    pub datetime: String,
    pub volume: u64,
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
    #[serde(default)]
    pub wa_price: Option<f64>,
    pub median: f64,
    #[serde(default)]
    pub moving_avg: Option<f64>,
    pub order_type: Side,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub mod_rank: Option<u8>,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub amber_stars: Option<u8>,
    #[serde(default)]
    pub cyan_stars: Option<u8>,
    /// Fields the market sent that this client does not model.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl LiveBucket {
    /// Start of the bucket in Unix seconds; `None` when `datetime` does not parse.
    pub fn starts_at(&self) -> Option<i64> {
        moment::seconds(&self.datetime)
    }
}

/// The four series the statistics endpoint returns.
#[derive(Debug, Clone, PartialEq)]
pub struct Statistics {
    /// Rolling 48 hours.
    pub closed_hourly: Vec<ClosedBucket>,
    /// Ninety days.
    pub closed_daily: Vec<ClosedBucket>,
    /// Rolling 48 hours.
    pub live_hourly: Vec<LiveBucket>,
    /// Ninety days.
    pub live_daily: Vec<LiveBucket>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(json: &str) -> ItemMeta {
        serde_json::from_str(json).expect("item meta parses")
    }

    #[test]
    fn stars_are_read_even_though_they_are_not_asked_for() {
        let ayatan = meta(
            r#"{"id":"d","slug":"ayatan_anasa_sculpture","tradable":true,
                "maxAmberStars":2,"maxCyanStars":2,"baseEndo":2000}"#,
        );
        assert_eq!(ayatan.max_amber_stars, Some(2));
        assert_eq!(ayatan.max_cyan_stars, Some(2));
    }

    #[test]
    fn an_unknown_presence_does_not_break_the_parse() {
        let trader: Trader =
            serde_json::from_str(r#"{"ingameName":"x","status":"invisible"}"#).expect("parses");
        assert_eq!(trader.status, Presence::Unknown);
        assert!(!trader.status.reachable());
    }

    #[test]
    fn being_on_the_site_is_not_being_reachable() {
        assert!(Presence::Ingame.reachable());
        assert!(!Presence::Online.reachable());
        assert!(!Presence::Offline.reachable());
    }
}
