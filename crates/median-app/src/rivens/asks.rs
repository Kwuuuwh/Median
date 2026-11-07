use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tracing::{info, warn};

use median_core::seconds;
use median_core::{
    Asks, Band, Basis, Listed, ListedStat, PoolStat, RivenClass, StatForm as CoreStatForm,
    TradeBounds,
};
use median_ipc::{BandBasis, Riven, RivenBand, RivenStat};
use median_market::{Auction, MarketError, PriceOrder, RivenSearch};

use super::appraisal::to_core;
use super::trades::Weekly;
use crate::catalog::{self, Armed, Catalog, Pooled};
use crate::market::{Gateway, Priority, Store, Sweep};

/// How long one search's auctions are reused before the search is sent again.
const KEPT: Duration = Duration::from_secs(60 * 60);

/// Minimum number of similar auctions; below it the search widens to each pair of buffs.
const ENOUGH_ALIKE: usize = 8;

/// Age in seconds after which a saved price band is recounted in the background.
const BAND_KEPT: i64 = 6 * 60 * 60;

/// Auctions found by one search, and when they were read.
struct Slice {
    read: Instant,
    auctions: Arc<Vec<Auction>>,
}

/// Recently read riven auctions keyed by search, saved to the market cache as they are read,
/// and the weapons whose rivens the user viewed, with the time of viewing.
pub struct Slices {
    kept: Mutex<HashMap<String, Slice>>,
    /// Per-search lock held while the search runs, so the same search is not sent twice at once.
    reading: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    looked: Mutex<HashMap<String, Instant>>,
    market_cache: Arc<Store>,
}

impl Slices {
    pub fn new(market_cache: Arc<Store>) -> Self {
        Self {
            kept: Mutex::default(),
            reading: Mutex::default(),
            looked: Mutex::default(),
            market_cache,
        }
    }

    /// Weapons whose rivens the user viewed within `within`.
    pub fn looked_within(&self, within: Duration) -> Vec<String> {
        self.looked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|(_, at)| at.elapsed() < within)
            .map(|(weapon, _)| weapon.clone())
            .collect()
    }

    /// Re-reads the buyout auctions of `weapon` at background priority; returns how many were
    /// found.
    pub async fn sweep(&self, gateway: &Gateway, weapon: &str) -> Result<usize, MarketError> {
        let search = RivenSearch {
            weapon: weapon.to_owned(),
            buyout_only: true,
            order: Some(PriceOrder::Cheapest),
            ..RivenSearch::default()
        };
        Ok(self
            .find(gateway, search, Priority::Background)
            .await?
            .len())
    }

    /// Auctions `search` finds; the saved result is reused until it is older than `KEPT`.
    async fn find(
        &self,
        gateway: &Gateway,
        search: RivenSearch,
        priority: Priority,
    ) -> Result<Arc<Vec<Auction>>, MarketError> {
        let key = format!("{search:?}");
        let turn = Arc::clone(
            self.reading
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(key.clone())
                .or_default(),
        );
        let _reading = turn.lock().await;
        if let Some(found) = self.fresh(&key) {
            return Ok(found);
        }
        let client = Arc::clone(gateway.client());
        let asked = search.clone();
        let found = gateway
            .call_auction(priority, |pass| async move {
                client.riven_auctions(pass, &asked).await
            })
            .await?;
        self.write_down(&search, &found).await;
        let found = Arc::new(found);
        self.kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                key,
                Slice {
                    read: Instant::now(),
                    auctions: Arc::clone(&found),
                },
            );
        Ok(found)
    }

    /// Saves the found auctions to the market cache and marks as closed the cached auctions this
    /// search covers but did not return.
    async fn write_down(&self, search: &RivenSearch, found: &[Auction]) {
        let at = seconds();
        let sweep = Sweep {
            weapon: search.weapon.clone(),
            buffs: search.buffs.clone(),
            found: found.iter().map(|auction| auction.id.clone()).collect(),
            ceiling: found
                .iter()
                .filter_map(|auction| auction.buyout_price)
                .max(),
        };
        if let Err(why) = self.market_cache.keep_auctions(found.to_vec(), at).await {
            warn!(error = %why, weapon = search.weapon, "riven auctions not saved");
            return;
        }
        match self.market_cache.sweep_auctions(sweep, at).await {
            Ok(0) => {}
            Ok(gone) => info!(
                auctions = gone,
                weapon = search.weapon,
                "riven auctions closed"
            ),
            Err(why) => {
                warn!(error = %why, weapon = search.weapon, "closed riven auctions not marked")
            }
        }
    }

    /// Number of riven auctions of weapon `slug` seen listed within `within`; `None` when zero.
    pub(super) async fn listed(&self, slug: &str, within: Duration) -> Result<Option<u32>, String> {
        let since = seconds() - i64::try_from(within.as_secs()).unwrap_or(i64::MAX);
        let listed = self
            .market_cache
            .listed_rivens(slug, since)
            .await
            .map_err(|e| e.to_string())?;
        Ok((listed > 0).then_some(listed))
    }

    /// Price band last saved for rivens similar to `riven`, and when it was saved; `None` when
    /// none is saved or the read fails.
    async fn kept_band(&self, riven: &Riven) -> Option<(Band, median_core::Instant)> {
        match self.market_cache.riven_band(band_key(riven)).await {
            Ok(kept) => kept,
            Err(why) => {
                warn!(error = %why, weapon = riven.weapon, "saved riven price band read failed");
                None
            }
        }
    }

    pub(super) async fn keep_band(&self, riven: &Riven, band: &Band) {
        if let Err(why) = self
            .market_cache
            .keep_riven_band(band_key(riven), *band, seconds())
            .await
        {
            warn!(error = %why, weapon = riven.weapon, "riven price band not saved");
        }
    }

    fn fresh(&self, key: &str) -> Option<Arc<Vec<Auction>>> {
        let kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        kept.get(key)
            .filter(|slice| slice.read.elapsed() < KEPT)
            .map(|slice| Arc::clone(&slice.auctions))
    }

    /// Buyout auctions of one weapon similar to a riven with `buffs`: those with every buff;
    /// with fewer than `ENOUGH_ALIKE`, also those with each pair of buffs (only for more than
    /// two buffs); with still too few, the weapon's cheapest. Deduplicated by id.
    async fn alike(
        &self,
        gateway: &Gateway,
        weapon: &str,
        buffs: &[String],
    ) -> Result<Vec<Auction>, MarketError> {
        let search = |buffs: Vec<String>| RivenSearch {
            weapon: weapon.to_owned(),
            buffs,
            buyout_only: true,
            order: Some(PriceOrder::Cheapest),
            ..RivenSearch::default()
        };
        self.looked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(weapon.to_owned(), Instant::now());
        let person = Priority::Person;
        let mut found: Vec<Auction> = self
            .find(gateway, search(buffs.to_vec()), person)
            .await?
            .to_vec();
        if found.len() < ENOUGH_ALIKE && buffs.len() > 2 {
            for (at, first) in buffs.iter().enumerate() {
                for second in &buffs[at + 1..] {
                    let pair = vec![first.clone(), second.clone()];
                    found.extend(
                        self.find(gateway, search(pair), person)
                            .await?
                            .iter()
                            .cloned(),
                    );
                }
            }
        }
        if found.len() < ENOUGH_ALIKE {
            found.extend(
                self.find(gateway, search(Vec::new()), person)
                    .await?
                    .iter()
                    .cloned(),
            );
        }
        found.sort_by(|one, other| one.id.cmp(&other.id));
        found.dedup_by(|one, other| one.id == other.id);
        Ok(found)
    }
}

/// Catalog data needed to grade and price the rivens of one weapon.
pub(super) struct Armory {
    pub armed: Armed,
    pub class: RivenClass,
    pub pool: Vec<Pooled>,
}

impl Armory {
    /// Loads the weapon `weapon` with stat labels in `lang`; `None` when it is not in the catalog
    /// or has no riven class.
    pub async fn of(catalog: &Catalog, lang: &str, weapon: String) -> Result<Option<Self>, String> {
        let lang = lang.to_owned();
        catalog
            .read(move |connection| {
                let Some(armed) = catalog::armed(connection, &weapon)? else {
                    return Ok(None);
                };
                let Some(class) = RivenClass::from_name(&armed.class) else {
                    return Ok(None);
                };
                let pool = catalog::pool(connection, &lang, &armed.class)?;
                Ok(Some(Self { armed, class, pool }))
            })
            .await
            .map_err(|e| e.to_string())
    }

    /// Buyout listings similar to `riven` and the auctions they were read from; `None` when
    /// warframe.market has no riven slug for the weapon.
    pub async fn asks(
        &self,
        gateway: &Gateway,
        slices: &Slices,
        riven: &Riven,
    ) -> Result<Option<(Asks, Vec<Auction>)>, String> {
        let Some(slug) = self.armed.slug.as_deref() else {
            return Ok(None);
        };
        let buffs: Vec<String> = riven
            .buffs
            .iter()
            .filter_map(|stat| market_slug(&self.pool, &stat.tag))
            .collect();
        let auctions = slices
            .alike(gateway, slug, &buffs)
            .await
            .map_err(|e| e.to_string())?;
        let listed: Vec<Listed> = auctions
            .iter()
            .filter_map(|auction| listed(auction, &self.pool))
            .collect();
        Ok(Some((Asks::new(listed), auctions)))
    }

    /// Price bounds from last week's trades of this weapon's rivens, rerolled or not; `None`
    /// without a riven family or trade data.
    pub fn bounds(&self, weekly: &Weekly, rerolled: bool) -> Option<TradeBounds> {
        weekly.bounds(self.armed.family.as_deref()?, rerolled)
    }
}

/// Highest price of a rerolled riven of `weapon`'s family in last week's trades; `None` when
/// the weapon, its family or the trade data is missing.
pub async fn ceiling(
    catalog: &Catalog,
    weekly: &Weekly,
    weapon: String,
) -> Result<Option<f64>, String> {
    let armed = catalog
        .read(move |connection| catalog::armed(connection, &weapon))
        .await
        .map_err(|e| e.to_string())?;
    Ok(armed
        .and_then(|armed| weekly.bounds(armed.family.as_deref()?, true))
        .map(|bounds| bounds.ceiling))
}

/// Price band of rivens similar to `riven`: the saved band (recounted in the background once
/// older than `BAND_KEPT`), or one counted now when none is saved. `None` when the weapon is not
/// in the catalog or warframe.market does not list its rivens.
pub async fn band(
    catalog: &Arc<Catalog>,
    gateway: &Gateway,
    slices: &Arc<Slices>,
    weekly: &Arc<Weekly>,
    riven: Riven,
) -> Result<Option<RivenBand>, String> {
    let Some((kept, at)) = slices.kept_band(&riven).await else {
        return Ok(count_band(catalog, gateway, slices, weekly, &riven)
            .await?
            .map(|band| to_band(&band)));
    };
    if seconds() - at >= BAND_KEPT {
        let (catalog, gateway) = (Arc::clone(catalog), gateway.clone());
        let (slices, weekly) = (Arc::clone(slices), Arc::clone(weekly));
        tauri::async_runtime::spawn(async move {
            if let Err(why) = count_band(&catalog, &gateway, &slices, &weekly, &riven).await {
                warn!(error = %why, weapon = riven.weapon, "riven price band recount failed");
            }
        });
    }
    Ok(Some(to_band(&kept)))
}

/// Counts the price band from current buyout prices of rivens similar to `riven` and saves it.
async fn count_band(
    catalog: &Catalog,
    gateway: &Gateway,
    slices: &Slices,
    weekly: &Weekly,
    riven: &Riven,
) -> Result<Option<Band>, String> {
    let Some(armory) = Armory::of(catalog, "en", riven.weapon.clone()).await? else {
        return Ok(None);
    };
    let Some((asks, _)) = armory.asks(gateway, slices, riven).await? else {
        return Ok(None);
    };
    let bounds = armory.bounds(weekly, riven.rerolls > 0);
    let counted = asks.band(&to_core(riven), bounds);
    if let Some(band) = &counted {
        slices.keep_band(riven, band).await;
    }
    Ok(counted)
}

/// Cache key of a band: weapon, sorted buff tags, sorted curse tags, and whether rerolled.
fn band_key(riven: &Riven) -> String {
    let tags = |stats: &[RivenStat]| {
        let mut tags: Vec<&str> = stats.iter().map(|stat| stat.tag.as_str()).collect();
        tags.sort_unstable();
        tags.join(",")
    };
    format!(
        "{}|{}|{}|{}",
        riven.weapon,
        tags(&riven.buffs),
        tags(&riven.curses),
        riven.rerolls > 0
    )
}

/// Converts a band to its IPC form.
pub(super) fn to_band(band: &Band) -> RivenBand {
    RivenBand {
        low: band.low,
        middle: band.middle,
        high: band.high,
        lots: count(band.lots),
        basis: match band.basis {
            Basis::Lots => BandBasis::Lots,
            Basis::FewLots => BandBasis::FewLots,
            Basis::Trades => BandBasis::Trades,
        },
        is_capped: band.is_capped,
    }
}

/// Converts the stats of `pool` to the core type; an unknown form counts as percent.
pub(super) fn pool_stats(pool: &[Pooled]) -> Vec<PoolStat<'_>> {
    pool.iter()
        .map(|stat| PoolStat {
            tag: &stat.tag,
            base: stat.base,
            can_buff: stat.can_buff,
            can_curse: stat.can_curse,
            form: CoreStatForm::from_name(&stat.form).unwrap_or(CoreStatForm::Percent),
        })
        .collect()
}

fn market_slug(pool: &[Pooled], tag: &str) -> Option<String> {
    pool.iter()
        .find(|stat| stat.tag == tag)
        .and_then(|stat| stat.slug.clone())
}

/// Converts an auction to a core listing; `None` without a buyout price. A stat not in the
/// catalog keeps its market slug as its tag.
fn listed(auction: &Auction, pool: &[Pooled]) -> Option<Listed> {
    let price = auction.buyout_price?;
    let stats = auction
        .stats
        .iter()
        .map(|stat| ListedStat {
            tag: pool
                .iter()
                .find(|pooled| pooled.slug.as_deref() == Some(stat.slug.as_str()))
                .map_or_else(|| stat.slug.clone(), |pooled| pooled.tag.clone()),
            shown: stat.value,
            positive: stat.positive,
        })
        .collect();
    Some(Listed {
        id: auction.id.clone(),
        price,
        rank: u8::try_from(auction.rank).unwrap_or(u8::MAX),
        stats,
    })
}

fn count(lots: usize) -> u32 {
    u32::try_from(lots).unwrap_or(u32::MAX)
}
