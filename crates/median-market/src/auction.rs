use serde::{Deserialize, Serialize};

use crate::error::MarketError;
use crate::gate::Pass;
use crate::http::Client;
use crate::session::Session;

/// v1 wraps every response in `{ payload }`.
#[derive(Debug, Deserialize)]
struct Envelope<T> {
    payload: T,
}

#[derive(Debug, Deserialize)]
struct Auctions {
    auctions: Vec<Raw>,
}

#[derive(Debug, Deserialize)]
struct One {
    auction: Raw,
}

/// One auction in the market's format, whatever item it sells.
#[derive(Debug, Deserialize)]
struct Raw {
    id: String,
    starting_price: u32,
    buyout_price: Option<u32>,
    top_bid: Option<u32>,
    is_direct_sell: bool,
    visible: bool,
    closed: bool,
    created: String,
    updated: String,
    #[serde(default)]
    note: String,
    owner: Option<Owner>,
    item: RawItem,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    #[serde(rename = "type")]
    kind: String,
    weapon_url_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    mastery_level: Option<u32>,
    #[serde(default)]
    re_rolls: Option<u32>,
    #[serde(default)]
    mod_rank: Option<u32>,
    #[serde(default)]
    polarity: Option<String>,
    #[serde(default)]
    attributes: Vec<AuctionStat>,
}

/// The player who created an auction.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Owner {
    pub ingame_name: String,
    pub reputation: i32,
    /// `ingame`, `online` or `offline`.
    pub status: String,
}

/// One stat of an auctioned riven, as the seller entered it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuctionStat {
    /// warframe.market slug of the stat.
    #[serde(rename = "url_name", alias = "slug")]
    pub slug: String,
    pub value: f64,
    pub positive: bool,
}

/// A riven auction.
#[derive(Debug, Clone, PartialEq)]
pub struct Auction {
    pub id: String,
    pub starting_price: u32,
    /// Buyout price; `None` when only bids can end the auction.
    pub buyout_price: Option<u32>,
    pub top_bid: Option<u32>,
    /// Whether it sells at a fixed price rather than to the highest bid.
    pub is_direct_sell: bool,
    pub visible: bool,
    pub closed: bool,
    /// Creation time, as the market's timestamp string.
    pub created: String,
    pub updated: String,
    pub note: String,
    /// `None` on the account's own auctions.
    pub owner: Option<Owner>,
    /// warframe.market slug of the weapon.
    pub weapon: String,
    /// The riven's name, as the seller entered it.
    pub name: String,
    pub mastery: u32,
    pub rerolls: u32,
    pub rank: u32,
    pub polarity: Option<String>,
    pub stats: Vec<AuctionStat>,
}

impl Raw {
    /// The auction as a riven auction; `None` when it sells something other than a riven.
    fn riven(self) -> Option<Auction> {
        if self.item.kind != "riven" {
            return None;
        }
        Some(Auction {
            id: self.id,
            starting_price: self.starting_price,
            buyout_price: self.buyout_price,
            top_bid: self.top_bid,
            is_direct_sell: self.is_direct_sell,
            visible: self.visible,
            closed: self.closed,
            created: self.created,
            updated: self.updated,
            note: self.note,
            owner: self.owner,
            weapon: self.item.weapon_url_name,
            name: self.item.name.unwrap_or_default(),
            mastery: self.item.mastery_level.unwrap_or(0),
            rerolls: self.item.re_rolls.unwrap_or(0),
            rank: self.item.mod_rank.unwrap_or(0),
            polarity: self.item.polarity,
            stats: self.item.attributes,
        })
    }
}

fn rivens(raw: Vec<Raw>) -> Vec<Auction> {
    raw.into_iter().filter_map(Raw::riven).collect()
}

/// Which end of the price range a search starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceOrder {
    Cheapest,
    Dearest,
}

/// Filters for a search among riven auctions of one weapon; an empty field filters nothing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RivenSearch {
    /// warframe.market slug of the weapon.
    pub weapon: String,
    /// Stat slugs every riven must have as buffs.
    pub buffs: Vec<String>,
    /// Stat slug the riven must have as its curse.
    pub curse: Option<String>,
    /// Whether the riven must have some curse.
    pub has_curse: bool,
    pub rerolls_max: Option<u32>,
    /// Only auctions with a buyout price.
    pub buyout_only: bool,
    pub order: Option<PriceOrder>,
}

impl RivenSearch {
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = vec![
            ("type", "riven".to_owned()),
            ("weapon_url_name", self.weapon.clone()),
        ];
        if !self.buffs.is_empty() {
            query.push(("positive_stats", self.buffs.join(",")));
        }
        match (&self.curse, self.has_curse) {
            (Some(curse), _) => query.push(("negative_stats", curse.clone())),
            (None, true) => query.push(("negative_stats", "has".to_owned())),
            (None, false) => {}
        }
        if let Some(most) = self.rerolls_max {
            query.push(("re_rolls_max", most.to_string()));
        }
        if self.buyout_only {
            query.push(("buyout_policy", "direct".to_owned()));
        }
        match self.order {
            Some(PriceOrder::Cheapest) => query.push(("sort_by", "price_asc".to_owned())),
            Some(PriceOrder::Dearest) => query.push(("sort_by", "price_desc".to_owned())),
            None => {}
        }
        query
    }
}

/// A riven auction to create.
#[derive(Debug, Clone, PartialEq)]
pub struct AuctionDraft {
    pub weapon: String,
    pub name: String,
    pub mastery: u32,
    pub rerolls: u32,
    pub rank: u32,
    /// The market's polarity name: `madurai`, `vazarin` or `naramon`.
    pub polarity: String,
    pub stats: Vec<AuctionStat>,
    pub starting_price: u32,
    /// `None` sells to the highest bid only.
    pub buyout_price: Option<u32>,
    pub minimal_reputation: u32,
    pub visible: bool,
    pub note: String,
}

impl AuctionDraft {
    fn body(&self) -> serde_json::Value {
        let stats: Vec<serde_json::Value> = self
            .stats
            .iter()
            .map(|stat| {
                serde_json::json!({
                    "url_name": stat.slug,
                    "value": stat.value,
                    "positive": stat.positive,
                })
            })
            .collect();
        serde_json::json!({
            "starting_price": self.starting_price,
            "buyout_price": self.buyout_price,
            "minimal_reputation": self.minimal_reputation,
            "visible": self.visible,
            "note": self.note,
            "item": {
                "type": "riven",
                "weapon_url_name": self.weapon,
                "name": self.name,
                "attributes": stats,
                "re_rolls": self.rerolls,
                "mastery_level": self.mastery,
                "mod_rank": self.rank,
                "polarity": self.polarity,
            },
        })
    }
}

/// Changes to one of the account's auctions; a `None` field stays unchanged.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AuctionChange {
    pub starting_price: Option<u32>,
    pub buyout_price: Option<u32>,
    pub minimal_reputation: Option<u32>,
    pub visible: Option<bool>,
    pub note: Option<String>,
}

impl AuctionChange {
    fn body(&self) -> serde_json::Value {
        let mut body = serde_json::Map::new();
        if let Some(price) = self.starting_price {
            body.insert("starting_price".into(), price.into());
        }
        if let Some(price) = self.buyout_price {
            body.insert("buyout_price".into(), price.into());
        }
        if let Some(reputation) = self.minimal_reputation {
            body.insert("minimal_reputation".into(), reputation.into());
        }
        if let Some(visible) = self.visible {
            body.insert("visible".into(), visible.into());
        }
        if let Some(note) = &self.note {
            body.insert("note".into(), note.clone().into());
        }
        serde_json::Value::Object(body)
    }
}

impl Client {
    /// Riven auctions of one weapon that match `search`: up to 500, about 550 KB.
    pub async fn riven_auctions(
        &self,
        pass: Pass,
        search: &RivenSearch,
    ) -> Result<Vec<Auction>, MarketError> {
        let url = reqwest::Url::parse_with_params(&self.v1("/auctions/search"), search.query())
            .map_err(|e| MarketError::Unreachable(e.to_string()))?;
        let answer: Envelope<Auctions> = self.get(pass, url.into(), &search.weapon).await?;
        Ok(rivens(answer.payload.auctions))
    }

    /// The riven auctions among the 100 most recently created auctions.
    pub async fn recent_auctions(&self, pass: Pass) -> Result<Vec<Auction>, MarketError> {
        let url = self.v1("/auctions");
        let answer: Envelope<Auctions> = self.get(pass, url, "recent auctions").await?;
        Ok(rivens(answer.payload.auctions))
    }

    /// One auction by its id; `None` when it does not sell a riven.
    pub async fn auction(&self, pass: Pass, id: &str) -> Result<Option<Auction>, MarketError> {
        let url = self.v1(&format!("/auctions/entry/{id}"));
        let answer: Envelope<One> = self.get(pass, url, id).await?;
        Ok(answer.payload.auction.riven())
    }
}

impl Session {
    /// Creates one riven auction; `Err(Unreachable)` when the created auction is not a riven.
    pub async fn put_up(&self, pass: Pass, draft: &AuctionDraft) -> Result<Auction, MarketError> {
        let url = self.client().v1("/auctions/create");
        let answer: Envelope<One> = self
            .client()
            .json(
                pass,
                reqwest::Method::POST,
                url,
                Some(self.authorization()),
                Some(draft.body()),
                "create auction",
            )
            .await?;
        answer
            .payload
            .auction
            .riven()
            .ok_or_else(|| MarketError::Unreachable("created auction sells no riven".into()))
    }

    /// Changes one of this account's auctions.
    pub async fn change_auction(
        &self,
        pass: Pass,
        id: &str,
        change: &AuctionChange,
    ) -> Result<Auction, MarketError> {
        let url = self.client().v1(&format!("/auctions/entry/{id}"));
        let answer: Envelope<One> = self
            .client()
            .json(
                pass,
                reqwest::Method::PUT,
                url,
                Some(self.authorization()),
                Some(change.body()),
                "change auction",
            )
            .await?;
        answer
            .payload
            .auction
            .riven()
            .ok_or_else(|| MarketError::Unreachable("changed auction sells no riven".into()))
    }

    /// Closes one of this account's auctions.
    pub async fn close_auction(&self, pass: Pass, id: &str) -> Result<(), MarketError> {
        let url = self.client().v1(&format!("/auctions/entry/{id}/close"));
        let _: Envelope<serde_json::Value> = self
            .client()
            .json(
                pass,
                reqwest::Method::PUT,
                url,
                Some(self.authorization()),
                None,
                "close auction",
            )
            .await?;
        Ok(())
    }

    /// Every riven auction of this account; `Err(Unauthorized)` when the session has no account
    /// name.
    pub async fn my_auctions(&self, pass: Pass) -> Result<Vec<Auction>, MarketError> {
        let name = self.name().ok_or(MarketError::Unauthorized)?;
        let url = self.client().v1(&format!("/profile/{name}/auctions"));
        let answer: Envelope<Auctions> = self
            .client()
            .json(
                pass,
                reqwest::Method::GET,
                url,
                Some(self.authorization()),
                None,
                "my auctions",
            )
            .await?;
        Ok(rivens(answer.payload.auctions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trimmed response of `GET /v1/auctions` in the real format: one riven, one lich.
    const RECENT: &str = r#"{"payload":{"auctions":[
      {"note":"","starting_price":400,"buyout_price":400,"minimal_reputation":0,"visible":true,
       "platform":"pc","crossplay":true,"closed":false,"top_bid":null,"is_marked_for":null,
       "marked_operation_at":null,"created":"2026-10-11T23:51:25.000+00:00",
       "updated":"2026-10-11T23:51:25.000+00:00","note_raw":"","is_direct_sell":true,
       "id":"6acc20fddd18e501350a5d56",
       "owner":{"reputation":1,"platform":"pc","locale":"en","avatar":null,
         "last_seen":"2026-10-11T23:40:14.570+00:00","ingame_name":"poxxii","slug":"poxxii",
         "crossplay":true,"status":"ingame","id":"6442944f0b076608e0b2d4a0","region":"en"},
       "winner":null,
       "item":{"mastery_level":8,"weapon_url_name":"phantasma","name":"hexa-scican",
         "type":"riven","re_rolls":3,
         "attributes":[{"value":111.0,"positive":true,"url_name":"multishot"},
                       {"value":-108.0,"positive":false,"url_name":"puncture_damage"}],
         "polarity":"madurai","mod_rank":8},
       "private":false},
      {"note":"","starting_price":50,"buyout_price":null,"minimal_reputation":0,"visible":true,
       "closed":false,"top_bid":60,"created":"2026-10-11T23:50:00.000+00:00",
       "updated":"2026-10-11T23:50:00.000+00:00","is_direct_sell":false,
       "id":"6acc20fddd18e501350a5d57","owner":null,
       "item":{"type":"lich","weapon_url_name":"kuva_bramma","element":"heat","damage":60}}
    ]}}"#;

    #[test]
    fn only_rivens_are_kept_and_their_stats_read_as_entered() {
        let answer: Envelope<Auctions> = serde_json::from_str(RECENT).expect("auctions parse");
        let found = rivens(answer.payload.auctions);

        assert_eq!(found.len(), 1);
        let riven = &found[0];
        assert_eq!(riven.weapon, "phantasma");
        assert_eq!(riven.buyout_price, Some(400));
        assert_eq!(riven.rerolls, 3);
        assert_eq!(
            riven.owner.as_ref().map(|owner| owner.status.as_str()),
            Some("ingame")
        );
        assert_eq!(
            riven.stats[1],
            AuctionStat {
                slug: "puncture_damage".into(),
                value: -108.0,
                positive: false,
            }
        );
    }

    #[test]
    fn a_search_names_only_what_narrows_it() {
        let search = RivenSearch {
            weapon: "torid".into(),
            buffs: vec!["critical_damage".into(), "multishot".into()],
            has_curse: true,
            buyout_only: true,
            order: Some(PriceOrder::Cheapest),
            ..RivenSearch::default()
        };

        let url = reqwest::Url::parse_with_params("https://market.test/search", search.query())
            .expect("a url");

        assert_eq!(
            url.query(),
            Some(
                "type=riven&weapon_url_name=torid&positive_stats=critical_damage%2Cmultishot\
                 &negative_stats=has&buyout_policy=direct&sort_by=price_asc"
            )
        );
    }

    #[test]
    fn a_change_sends_only_what_it_changes() {
        let change = AuctionChange {
            buyout_price: Some(350),
            ..AuctionChange::default()
        };

        assert_eq!(change.body(), serde_json::json!({ "buyout_price": 350 }));
    }

    #[test]
    fn a_draft_sends_the_riven_as_the_market_lists_it() {
        let draft = AuctionDraft {
            weapon: "dual_toxocyst".into(),
            name: "hexa-visitis".into(),
            mastery: 14,
            rerolls: 114,
            rank: 8,
            polarity: "vazarin".into(),
            stats: vec![AuctionStat {
                slug: "base_damage_/_melee_damage".into(),
                value: 274.4,
                positive: true,
            }],
            starting_price: 500,
            buyout_price: Some(500),
            minimal_reputation: 0,
            visible: true,
            note: String::new(),
        };

        let body = draft.body();
        assert_eq!(body["item"]["type"], "riven");
        assert_eq!(
            body["item"]["attributes"][0]["url_name"],
            "base_damage_/_melee_damage"
        );
        assert_eq!(body["item"]["re_rolls"], 114);
        assert_eq!(body["buyout_price"], 500);
    }
}
