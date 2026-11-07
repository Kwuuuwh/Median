use median_core::{
    Currency, Deadline, Era, Fissure, Instant, Offer, Price, Reset, Shelf, Vendor, Visit,
    Worldstate, cycles,
};
use serde::Deserialize;

/// Tag of the Ostron bounties; their rotation matches the Cetus day-night cycle, so their expiry
/// anchors the cycle clocks.
const OSTRON: &str = "CetusSyndicate";

/// Path segment that store item keys contain and catalog keys do not.
const STORE: &str = "/StoreItems/";

/// The fields of the raw world-state document that median reads.
#[derive(Deserialize)]
struct RawWorldstate {
    #[serde(rename = "Time")]
    time: Instant,
    #[serde(rename = "SyndicateMissions")]
    syndicates: Vec<RawSyndicate>,
    #[serde(rename = "ActiveMissions")]
    fissures: Vec<RawFissure>,
    #[serde(rename = "VoidStorms", default)]
    storms: Vec<RawFissure>,
    #[serde(rename = "Sorties", default)]
    sorties: Vec<RawWindow>,
    #[serde(rename = "LiteSorties", default)]
    archons: Vec<RawWindow>,
    #[serde(rename = "VoidTraders", default)]
    void_traders: Vec<RawTrader>,
    #[serde(rename = "PrimeVaultTraders", default)]
    vault_traders: Vec<RawTrader>,
}

#[derive(Deserialize)]
struct RawSyndicate {
    #[serde(rename = "Tag")]
    tag: String,
    #[serde(rename = "Expiry")]
    expiry: RawDate,
}

#[derive(Deserialize)]
struct RawWindow {
    #[serde(rename = "Expiry")]
    expiry: RawDate,
}

#[derive(Deserialize)]
struct RawFissure {
    #[serde(rename = "Node")]
    node: String,
    /// `Modifier` on ground missions, `ActiveMissionTier` on railjack storms.
    #[serde(rename = "Modifier", alias = "ActiveMissionTier")]
    tier: String,
    #[serde(rename = "Expiry")]
    expiry: RawDate,
    #[serde(rename = "Hard", default)]
    hard: bool,
    #[serde(rename = "MissionType")]
    mission: Option<String>,
}

#[derive(Deserialize)]
struct RawTrader {
    #[serde(rename = "Node")]
    node: String,
    #[serde(rename = "Activation")]
    activation: RawDate,
    #[serde(rename = "Expiry")]
    expiry: RawDate,
    #[serde(rename = "Manifest", default)]
    manifest: Vec<RawOffer>,
    /// Varzia's permanent stock, listed apart from the seasonal `Manifest`.
    #[serde(rename = "EvergreenManifest", default)]
    evergreen: Vec<RawOffer>,
}

/// One item in a trader's stock. `PrimePrice` is ducats for Baro and regal aya for Varzia;
/// `RegularPrice` is credits for Baro and aya for Varzia.
#[derive(Deserialize)]
struct RawOffer {
    #[serde(rename = "ItemType")]
    item: String,
    #[serde(rename = "PrimePrice")]
    prime: Option<u32>,
    #[serde(rename = "RegularPrice")]
    regular: Option<u32>,
}

/// A timestamp as the document encodes it: Unix milliseconds in a string.
#[derive(Deserialize)]
struct RawDate {
    #[serde(rename = "$date")]
    date: RawMillis,
}

#[derive(Deserialize)]
struct RawMillis {
    #[serde(rename = "$numberLong")]
    value: String,
}

impl RawDate {
    fn at(&self) -> Result<Instant, String> {
        self.date
            .value
            .parse::<i64>()
            .map(|millis| millis / 1000)
            .map_err(|_| format!("unreadable date {}", self.date.value))
    }
}

/// Parses a world-state body into the world state (cycles, fissures, resets, trader visits) and
/// the traders' stock; `Err` when the body does not parse or has no Ostron bounty entry.
pub fn parse(body: &str) -> Result<(Worldstate, Vec<Shelf>), String> {
    let raw: RawWorldstate = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let now = raw.time;

    let ostron = raw
        .syndicates
        .iter()
        .find(|syndicate| syndicate.tag == OSTRON)
        .ok_or_else(|| format!("no {OSTRON} window"))?
        .expiry
        .at()?;

    let worldstate = Worldstate {
        at: now,
        cycles: cycles(now, ostron),
        fissures: fissures(&raw, now)?,
        deadlines: deadlines(&raw, now)?,
        visits: visits(&raw)?,
    };

    Ok((worldstate, shelves(&raw)))
}

fn fissures(raw: &RawWorldstate, now: Instant) -> Result<Vec<Fissure>, String> {
    let missions = raw.fissures.iter().map(|mission| (mission, false));
    let storms = raw.storms.iter().map(|storm| (storm, true));

    let mut open = Vec::with_capacity(raw.fissures.len() + raw.storms.len());
    for (mission, storm) in missions.chain(storms) {
        let expires = mission.expiry.at()?;
        if let Some(era) = era(&mission.tier)
            && expires > now
        {
            open.push(Fissure {
                node: mission.node.clone(),
                era,
                hard: mission.hard,
                storm,
                mission: mission.mission.clone(),
                expires,
            });
        }
    }

    open.sort_by_key(|fissure| fissure.expires);
    Ok(open)
}

fn deadlines(raw: &RawWorldstate, now: Instant) -> Result<Vec<Deadline>, String> {
    let mut clocks = vec![Deadline {
        reset: Reset::Daily,
        at: next_boundary(now, 86_400),
    }];

    if let Some(sortie) = raw.sorties.first() {
        clocks.push(Deadline {
            reset: Reset::Sortie,
            at: sortie.expiry.at()?,
        });
    }
    if let Some(archon) = raw.archons.first() {
        clocks.push(Deadline {
            reset: Reset::Weekly,
            at: archon.expiry.at()?,
        });
    }

    clocks.sort_by_key(|deadline| deadline.at);
    Ok(clocks)
}

fn visits(raw: &RawWorldstate) -> Result<Vec<Visit>, String> {
    let mut standing = Vec::new();

    for (vendor, trader) in [
        (Vendor::Baro, raw.void_traders.first()),
        (Vendor::Varzia, raw.vault_traders.first()),
    ] {
        let Some(trader) = trader else { continue };
        standing.push(Visit {
            vendor,
            node: trader.node.clone(),
            arrives: trader.activation.at()?,
            departs: trader.expiry.at()?,
        });
    }

    Ok(standing)
}

fn shelves(raw: &RawWorldstate) -> Vec<Shelf> {
    let mut standing = Vec::new();

    if let Some(baro) = raw.void_traders.first() {
        standing.push(Shelf {
            vendor: Vendor::Baro,
            offers: offers(&baro.manifest, Currency::Ducats, Currency::Credits),
        });
    }
    if let Some(varzia) = raw.vault_traders.first() {
        let mut goods = offers(&varzia.manifest, Currency::RegalAya, Currency::Aya);
        goods.extend(offers(&varzia.evergreen, Currency::RegalAya, Currency::Aya));
        standing.push(Shelf {
            vendor: Vendor::Varzia,
            offers: goods,
        });
    }

    standing
}

/// Trader stock entries, with store item keys rewritten to catalog keys.
fn offers(raw: &[RawOffer], prime: Currency, regular: Currency) -> Vec<Offer> {
    raw.iter()
        .map(|entry| {
            let mut prices = Vec::new();
            if let Some(amount) = entry.prime {
                prices.push(Price {
                    currency: prime,
                    amount,
                });
            }
            if let Some(amount) = entry.regular {
                prices.push(Price {
                    currency: regular,
                    amount,
                });
            }
            Offer {
                item: entry.item.replacen(STORE, "/", 1),
                prices,
            }
        })
        .collect()
}

/// Maps the document's fissure tier, `VoidT1` through `VoidT6`, to a relic era; `None` for any
/// other tier.
fn era(tier: &str) -> Option<Era> {
    match tier {
        "VoidT1" => Some(Era::Lith),
        "VoidT2" => Some(Era::Meso),
        "VoidT3" => Some(Era::Neo),
        "VoidT4" => Some(Era::Axi),
        "VoidT5" => Some(Era::Requiem),
        "VoidT6" => Some(Era::Omnia),
        _ => None,
    }
}

/// First instant after `now` divisible by `period`.
fn next_boundary(now: Instant, period: i64) -> Instant {
    now + period - now.rem_euclid(period)
}

#[cfg(test)]
mod tests {
    use super::{next_boundary, parse};
    use median_core::{Currency, Era, Reset, Shelf, Vendor, Worldstate};

    /// A document trimmed to the fields median reads, taken on 2026-07-28T12:07:21Z.
    const DOCUMENT: &str = r#"{
      "Time": 1785240441,
      "SyndicateMissions": [
        { "Tag": "SolarisSyndicate", "Expiry": { "$date": { "$numberLong": "1785243478000" } } },
        { "Tag": "CetusSyndicate", "Expiry": { "$date": { "$numberLong": "1785243478000" } } }
      ],
      "ActiveMissions": [
        { "Node": "SolNode131", "Modifier": "VoidT2", "Hard": true,
          "Expiry": { "$date": { "$numberLong": "1785240536000" } } },
        { "Node": "SolNode744", "Modifier": "VoidT6",
          "Expiry": { "$date": { "$numberLong": "1785244000000" } } },
        { "Node": "SolNode1", "Modifier": "VoidT1",
          "Expiry": { "$date": { "$numberLong": "1785200000000" } } }
      ],
      "VoidStorms": [
        { "Node": "CrewBattleNode509", "ActiveMissionTier": "VoidT1",
          "Expiry": { "$date": { "$numberLong": "1785243003000" } } }
      ],
      "Sorties": [ { "Expiry": { "$date": { "$numberLong": "1785254400000" } } } ],
      "LiteSorties": [ { "Expiry": { "$date": { "$numberLong": "1785715200000" } } } ],
      "VoidTraders": [
        { "Node": "SaturnHUB",
          "Activation": { "$date": { "$numberLong": "1786107600000" } },
          "Expiry": { "$date": { "$numberLong": "1786280400000" } },
          "Manifest": [] }
      ],
      "PrimeVaultTraders": [
        { "Node": "TradeHUB1",
          "Activation": { "$date": { "$numberLong": "1783620000000" } },
          "Expiry": { "$date": { "$numberLong": "1786039200000" } },
          "Manifest": [
            { "ItemType": "/Lotus/Types/StoreItems/Packages/MegaPrimeVault/MPVGaraPrimeSinglePack",
              "PrimePrice": 6 }
          ],
          "EvergreenManifest": [
            { "ItemType": "/Lotus/StoreItems/Weapons/Tenno/Rifle/BratonPrime", "PrimePrice": 1 },
            { "ItemType": "/Lotus/StoreItems/Upgrades/Skins/Scarves/InfMembraneCape",
              "RegularPrice": 10 }
          ] }
      ]
    }"#;

    fn world() -> Worldstate {
        parse(DOCUMENT).expect("a readable document").0
    }

    fn shelf(vendor: Vendor) -> Shelf {
        parse(DOCUMENT)
            .expect("a readable document")
            .1
            .into_iter()
            .find(|shelf| shelf.vendor == vendor)
            .expect("a shelf")
    }

    #[test]
    fn cycles_are_pinned_to_the_ostron_window() {
        let world = world();

        assert_eq!(world.cycles.len(), 5);
        assert_eq!(world.cycles[0].phase, 0);
        assert_eq!(world.cycles[0].ends, 1_785_240_478);
    }

    #[test]
    fn expired_fissures_are_dropped_and_storms_are_kept() {
        let world = world();

        assert_eq!(world.fissures.len(), 3);
        assert_eq!(world.fissures[0].era, Era::Meso);
        assert!(world.fissures[0].hard);
        assert!(world.fissures.iter().any(|fissure| fissure.storm));
    }

    #[test]
    fn the_daily_reset_is_computed_and_the_others_are_read() {
        let world = world();
        let at = |reset| {
            world
                .deadlines
                .iter()
                .find(|deadline| deadline.reset == reset)
                .map(|deadline| deadline.at)
        };

        assert_eq!(at(Reset::Daily), Some(1_785_283_200));
        assert_eq!(at(Reset::Sortie), Some(1_785_254_400));
        assert_eq!(at(Reset::Weekly), Some(1_785_715_200));
    }

    #[test]
    fn both_traders_get_a_visit() {
        let world = world();

        assert_eq!(world.visits[0].vendor, Vendor::Baro);
        assert_eq!(world.visits[0].departs - world.visits[0].arrives, 172_800);
        assert_eq!(world.visits[1].vendor, Vendor::Varzia);
        assert_eq!(world.visits[1].node, "TradeHUB1");
    }

    #[test]
    fn store_keys_become_catalog_keys() {
        let varzia = shelf(Vendor::Varzia);

        assert_eq!(varzia.offers.len(), 3);
        assert_eq!(
            varzia.offers[1].item,
            "/Lotus/Weapons/Tenno/Rifle/BratonPrime"
        );
        assert_eq!(varzia.offers[1].prices[0].currency, Currency::RegalAya);
        assert_eq!(varzia.offers[2].prices[0].currency, Currency::Aya);
    }

    #[test]
    fn a_trader_on_the_road_brings_an_empty_shelf() {
        assert!(shelf(Vendor::Baro).offers.is_empty());
    }

    #[test]
    fn a_document_without_the_ostron_window_is_rejected() {
        assert!(parse(r#"{"Time":1,"SyndicateMissions":[],"ActiveMissions":[]}"#).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        assert!(parse(r#"{"Time":1}"#).is_err());
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(parse("<!doctype html>").is_err());
    }

    #[test]
    fn boundaries_land_after_now() {
        assert_eq!(next_boundary(1_785_240_441, 86_400), 1_785_283_200);
        assert_eq!(next_boundary(1_785_283_200, 86_400), 1_785_369_600);
    }
}
