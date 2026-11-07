use std::collections::{BTreeMap, HashMap};

use median_core::{
    DAY, Detail, Fair, Instant, Inventory, LEAST, NEAR, Policy, Prospect, Resale, Sight, THIN,
    Traded, credible_ask, fair, falling, prospect, seconds,
};
use median_ipc::{Listing, Orders, OwnOrder, Priced};
use median_market::{Order, Presence};

use crate::account;
use crate::catalog::{self, Catalog};
use crate::market::schedule::Freshener;
use crate::market::store::{Bucket, Look, Recent, STATISTICS, Store, StoreError};
use crate::market::variant::Variant;

/// In-game whisper that opens a trade, in English and Russian.
const WHISPER_EN: &str = "/w {name} Hi! I want to {deal}: \"{item}\" for {platinum} platinum";
const WHISPER_RU: &str = "/w {name} Привет! Хочу {deal}: \"{item}\" за {platinum} платины";

/// Whisper suffix for an order traded in lots instead of one copy at a time.
const LOT_EN: &str = " each, {lot} pieces ({total} platinum)";
const LOT_RU: &str = " за штуку, {lot} шт ({total} платины)";

/// Subtype of an ordinary copy on items where the market also lists a collectible print.
const REGULAR: &str = "regular";

/// One holding mapped to its market slug and variant.
pub(crate) struct Listed {
    pub(crate) item: String,
    /// The copy's variant as the game writes it.
    pub(crate) variant: String,
    pub(crate) slug: String,
    pub(crate) of: Variant,
}

/// Every open position, with names from the catalog and prices from the market cache.
pub async fn positions(
    catalog: &Catalog,
    market_cache: &Store,
    history: &account::Store,
    lang: &str,
) -> Result<Vec<median_ipc::Position>, String> {
    let held = history.open_positions().await.map_err(|e| e.to_string())?;
    let standing = history.standing_orders().await.map_err(|e| e.to_string())?;
    let now = seconds();
    let mut rows = Vec::with_capacity(held.len());

    for position in held {
        let known = named(catalog, lang, &position.slug).await?;
        let (name, item, image) = match known {
            Some(known) => (known.name, known.item, known.image),
            None => (position.slug.clone(), position.item.clone(), None),
        };

        let of = Variant::parse(&position.variant);
        let look = market_cache
            .look(&position.slug, &of)
            .await
            .map_err(|e| e.to_string())?;
        let asking = standing
            .iter()
            .find(|order| {
                order.selling
                    && order.slug.as_deref() == Some(&position.slug)
                    && order.variant == position.variant
            })
            .map(|order| order.platinum);

        rows.push(median_ipc::Position {
            item,
            slug: position.slug.clone(),
            name,
            image,
            variant: position.variant.clone(),
            max_rank: market_cache
                .max_rank(&position.slug)
                .await
                .map_err(|e| e.to_string())?,
            quantity: position.quantity,
            basis: position.basis(),
            ingame: look.as_ref().and_then(Look::held_ask),
            looked: look.as_ref().map(|look| age(now, look.taken)),
            days: u32::try_from((now - position.opened).max(0) / DAY).unwrap_or(0),
            asking,
        });
    }
    Ok(rows)
}

pub async fn own_orders(
    catalog: &Catalog,
    market_cache: &Store,
    history: &account::Store,
    held: &Inventory,
    lang: &str,
    policy: &Policy,
) -> Result<Vec<OwnOrder>, String> {
    let standing = history.standing_orders().await.map_err(|e| e.to_string())?;
    let positions = history.open_positions().await.map_err(|e| e.to_string())?;
    let now = seconds();

    // The market cache and the catalog are read once for the whole list; one read of each per
    // order made the orders page slow.
    let by_id = match standing.iter().any(|order| order.slug.is_none()) {
        true => market_cache
            .slugs_by_id()
            .await
            .map_err(|e| e.to_string())?,
        false => BTreeMap::new(),
    };
    let mut slugs: Vec<String> = standing
        .iter()
        .filter_map(|order| slug_of(order, &by_id))
        .collect();
    slugs.sort();
    slugs.dedup();
    let mut pairs: Vec<(String, String)> = standing
        .iter()
        .filter_map(|order| {
            slug_of(order, &by_id).map(|slug| (slug, Variant::parse(&order.variant).key()))
        })
        .collect();
    pairs.sort();
    pairs.dedup();

    let recent = market_cache
        .recent(pairs)
        .await
        .map_err(|e| e.to_string())?;
    let ranks = market_cache
        .max_ranks(slugs.clone())
        .await
        .map_err(|e| e.to_string())?;
    let named = names_of(catalog, lang, slugs).await?;
    let told = Told {
        held,
        recent: &recent,
        ranks: &ranks,
        named: &named,
        now,
        policy,
    };

    let mut rows = Vec::with_capacity(standing.len());
    for order in standing {
        let basis = cost(&positions, &order);
        let slug = slug_of(&order, &by_id);
        let mut row = row(&told, order, slug);
        row.basis = basis;
        rows.push(row);
    }
    Ok(rows)
}

/// Slug of an order: its own, else the market cache's slug for its item id.
fn slug_of(order: &account::Standing, by_id: &BTreeMap<String, String>) -> Option<String> {
    order
        .slug
        .clone()
        .or_else(|| by_id.get(&order.item_id).cloned())
}

/// Catalog names, game item keys and images of these slugs, in one read.
async fn names_of(
    catalog: &Catalog,
    lang: &str,
    slugs: Vec<String>,
) -> Result<BTreeMap<String, catalog::Named>, String> {
    let lang = lang.to_owned();
    catalog
        .read(move |connection| {
            let mut out = BTreeMap::new();
            for slug in slugs {
                if let Some(named) = catalog::named(connection, &lang, &slug)? {
                    out.insert(slug, named);
                }
            }
            Ok(out)
        })
        .await
        .map_err(|e| e.to_string())
}

/// Market cache, catalog and holdings data for building order rows.
struct Told<'a> {
    held: &'a Inventory,
    recent: &'a BTreeMap<(String, String), Recent>,
    ranks: &'a BTreeMap<String, u8>,
    named: &'a BTreeMap<String, catalog::Named>,
    now: Instant,
    policy: &'a Policy,
}

/// IPC row of one of the account's orders, built from `told`.
fn row(told: &Told<'_>, order: account::Standing, slug: Option<String>) -> OwnOrder {
    let known = slug.as_ref().and_then(|slug| told.named.get(slug));
    let (name, item, image) = match known {
        Some(known) => (known.name.clone(), known.item.clone(), known.image.clone()),
        None => (
            slug.clone().unwrap_or_else(|| order.item_id.clone()),
            String::new(),
            None,
        ),
    };

    let of = Variant::parse(&order.variant);
    let held = slug
        .as_ref()
        .and_then(|slug| told.recent.get(&(slug.clone(), of.key())));
    let reading = reading_of(held, told.now);
    let max_rank = slug.as_ref().and_then(|slug| told.ranks.get(slug).copied());

    // Counted before the struct literal, which moves `variant`.
    let owned = owned(told.held, &item, &order.variant);

    // The same calculation the agent bids by: a copy is worth its expected sale price while a
    // reachable seller exists, and a bid is at most that price less the profit floor per copy.
    let resale = match order.selling {
        true => None,
        false => reading.ingame.and(reading.worth(told.policy)),
    };
    let most =
        resale.map(|worth| worth.saturating_sub(told.policy.least_per_copy(order.per_trade)));

    OwnOrder {
        id: order.id,
        slug: slug.unwrap_or_default(),
        item,
        name,
        image,
        variant: order.variant,
        max_rank,
        selling: order.selling,
        platinum: order.platinum,
        quantity: order.quantity,
        owned,
        per_trade: order.per_trade,
        visible: order.visible,
        by_agent: order.by_agent,
        reference: match order.selling {
            true => reading.ingame,
            false => reading.bid,
        },
        basis: None,
        ours: reading.ours.map(|fair| fair.platinum.round() as u32),
        most,
        resale,
    }
}

/// Cost per copy of the account's position on this order's pair; `None` without one.
fn cost(positions: &[account::Position], order: &account::Standing) -> Option<u32> {
    let slug = order.slug.as_deref()?;
    positions
        .iter()
        .find(|held| held.slug == slug && held.variant == order.variant)
        .map(account::Position::basis)
}

/// IPC row of one of the account's orders, with catalog names, market cache prices and held
/// copies.
pub async fn one_order(
    catalog: &Catalog,
    market_cache: &Store,
    held: &Inventory,
    lang: &str,
    order: account::Standing,
    now: Instant,
    policy: &Policy,
) -> Result<OwnOrder, String> {
    let slug = match order.slug.clone() {
        Some(slug) => Some(slug),
        None => market_cache
            .slug_of(order.item_id.clone())
            .await
            .map_err(|e| e.to_string())?,
    };
    let slugs: Vec<String> = slug.clone().into_iter().collect();
    let pairs: Vec<(String, String)> = slugs
        .iter()
        .map(|slug| (slug.clone(), Variant::parse(&order.variant).key()))
        .collect();

    let recent = market_cache
        .recent(pairs)
        .await
        .map_err(|e| e.to_string())?;
    let ranks = market_cache
        .max_ranks(slugs.clone())
        .await
        .map_err(|e| e.to_string())?;
    let named = names_of(catalog, lang, slugs).await?;
    let told = Told {
        held,
        recent: &recent,
        ranks: &ranks,
        named: &named,
        now,
        policy,
    };

    Ok(row(&told, order, slug))
}

/// The holdings of `item` in a variant that `want` covers.
fn copies<'a>(
    held: &'a Inventory,
    item: &'a str,
    want: &'a Variant,
) -> impl Iterator<Item = &'a median_core::Holding> + 'a {
    held.holdings.iter().filter(move |holding| {
        holding.item == item
            && Variant::of_detail(holding.detail.as_ref()).is_some_and(|of| want.covers(&of))
    })
}

/// Copies of this item and variant the account holds; `None` for an unknown item.
fn owned(held: &Inventory, item: &str, variant: &str) -> Option<u32> {
    if item.is_empty() {
        return None;
    }
    let want = Variant::parse(variant);
    Some(copies(held, item, &want).map(|holding| holding.count).sum())
}

/// Copies of this item and variant not equipped in any build; `None` for an unknown item.
pub fn free(held: &Inventory, item: &str, variant: &str) -> Option<u32> {
    if item.is_empty() {
        return None;
    }
    let want = Variant::parse(variant);
    Some(
        copies(held, item, &want)
            .map(|holding| {
                // The equipped list names a copy once however many loadouts use it, so
                // equipping takes one copy.
                let taken = u32::from(
                    held.equipped
                        .iter()
                        .any(|copy| copy.item == holding.item && copy.variant == holding.variant),
                );
                holding.count.saturating_sub(taken)
            })
            .sum(),
    )
}

/// Market cache prices and volume of one variant.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Reading {
    /// Copies traded over the last 48 hours.
    pub traded: Option<u32>,
    /// Volume-weighted median of closed trades.
    pub ours: Option<Fair>,
    /// Lowest ask among sellers reachable now.
    pub ingame: Option<u32>,
    /// Highest bid among buyers reachable now.
    pub bid: Option<u32>,
    /// Time of the last order book read.
    pub looked: Option<Instant>,
    /// Sale outlook: sell at once to the highest bid, or list at the ask and wait.
    pub prospect: Prospect,
    /// Whether the trade price is falling.
    pub falling: bool,
}

impl Reading {
    /// Expected sale price of a copy within the policy's `sale_days`.
    pub fn worth(&self, policy: &Policy) -> Option<u32> {
        self.prospect.worth(policy.sale_days)
    }
}

/// Closed trades and the last book read of one pair, as used to price a bid.
#[derive(Debug, Clone, Default)]
pub(crate) struct Sighted {
    pub hours: Vec<Traded>,
    pub days: Vec<Traded>,
    pub look: Option<Look>,
    /// Copies per trade offered by in-game sellers; `None` when no seller listed a lot.
    pub lot: Option<u32>,
}

impl Sighted {
    /// Pricing input for the agent at `now`, for the game item key `item`.
    pub fn sight<'a>(&'a self, item: &'a str, now: Instant) -> Sight<'a> {
        Sight {
            item,
            hours: &self.hours,
            days: &self.days,
            top_bid: self.look.and_then(|look| look.highest_bid_ingame),
            lead: self
                .look
                .and_then(|look| look.highest_bid_ingame)
                .unwrap_or(LEAST),
            held_ask: self.look.and_then(|look| look.held_ask()),
            sellers: self.look.and_then(|look| look.sellers_ingame),
            lot: self.lot.unwrap_or(1),
            resale: None,
            now,
        }
    }

    /// Resale pricing input of this pair, for a copy bought as another variant and sold as this
    /// one.
    pub fn resale(&self) -> Resale<'_> {
        Resale {
            hours: &self.hours,
            days: &self.days,
            top_bid: self.look.and_then(|look| look.highest_bid_ingame),
            held_ask: self.look.and_then(|look| look.held_ask()),
            sellers: self.look.and_then(|look| look.sellers_ingame),
        }
    }
}

/// How far back the sellers' lot sizes are read.
const LOTS_SEEN: Instant = 7 * DAY;

/// Market cache data of one variant for pricing a bid at `now`; empty when the cache has none.
pub(crate) async fn sighted(
    store: &Store,
    slug: &str,
    variant: &Variant,
    now: Instant,
) -> Result<Sighted, StoreError> {
    let pair = (slug.to_owned(), variant.key());
    let held = store.recent(vec![pair.clone()]).await?;
    let Some(recent) = held.get(&pair) else {
        return Ok(Sighted::default());
    };
    let series = |buckets: &[Bucket]| buckets.iter().map(Bucket::traded).collect();
    Ok(Sighted {
        hours: series(&recent.hour),
        days: series(&recent.day),
        look: recent.look,
        lot: store.sold_lot(slug, &pair.1, now - LOTS_SEEN).await?,
    })
}

/// Price reading of one variant from the market cache alone, without a market request.
pub(crate) async fn priced(
    store: &Store,
    slug: &str,
    variant: &Variant,
    now: Instant,
) -> Result<Reading, StoreError> {
    let pair = (slug.to_owned(), variant.key());
    let held = store.recent(vec![pair.clone()]).await?;
    Ok(reading_of(held.get(&pair), now))
}

/// Price reading of one pair from its cached closed trades and last book read; the default
/// reading when `held` is `None`.
pub(crate) fn reading_of(held: Option<&Recent>, now: Instant) -> Reading {
    let Some(held) = held else {
        return Reading::default();
    };
    let mut traded: Vec<Traded> = Vec::new();
    let mut by_hour: Option<u64> = None;
    let mut by_day: Option<u64> = None;
    for (buckets, counted) in [(&held.hour, &mut by_hour), (&held.day, &mut by_day)] {
        for bucket in buckets {
            if bucket.starts >= now - NEAR {
                *counted = Some(counted.unwrap_or(0) + bucket.volume);
            }
            traded.push(bucket.traded());
        }
    }
    let lately = by_hour.or(by_day);
    let series =
        |buckets: &[Bucket]| -> Vec<Traded> { buckets.iter().map(Bucket::traded).collect() };
    let (hours, days) = (series(&held.hour), series(&held.day));

    let ours = fair(&traded, now, THIN);
    let ingame = credible_ask(held.look.and_then(|look| look.held_ask()), ours.as_ref());
    let bid = held.look.and_then(|look| look.highest_bid_ingame);
    let sellers = held.look.and_then(|look| look.sellers_ingame);
    Reading {
        traded: lately.map(|volume| volume as u32),
        ours,
        ingame,
        bid,
        looked: held.look.map(|look| look.taken),
        prospect: prospect(&hours, &days, now, bid, ingame, sellers),
        falling: falling(&hours, &days, now),
    }
}

/// Catalog name, game item key and image of this slug.
pub(crate) async fn named(
    catalog: &Catalog,
    lang: &str,
    slug: &str,
) -> Result<Option<catalog::Named>, String> {
    let (slug, lang) = (slug.to_owned(), lang.to_owned());
    catalog
        .read(move |connection| catalog::named(connection, &lang, &slug))
        .await
        .map_err(|e| e.to_string())
}

/// Market cache prices of every holding with a market listing.
pub async fn prices(
    catalog: &Catalog,
    store: &Store,
    held: &Inventory,
) -> Result<Vec<Priced>, String> {
    let listed = listed(catalog, store, held).await?;
    let now = seconds();
    let mut prices = Vec::with_capacity(listed.len());
    let mut caps: HashMap<String, Option<u8>> = HashMap::new();

    for holding in &listed {
        let cap = match caps.get(&holding.slug) {
            Some(cap) => *cap,
            None => {
                let cap = store
                    .max_rank(&holding.slug)
                    .await
                    .map_err(|e| e.to_string())?;
                caps.insert(holding.slug.clone(), cap);
                cap
            }
        };
        prices.push(priced_row(store, holding, cap, now).await?);
    }
    Ok(prices)
}

/// Current market cache prices of the holdings listed under one slug.
pub(crate) async fn prices_of(
    store: &Store,
    listed: &[Listed],
    slug: &str,
    now: Instant,
) -> Result<Vec<Priced>, String> {
    let cap = store.max_rank(slug).await.map_err(|e| e.to_string())?;
    let mut prices = Vec::new();
    for holding in listed.iter().filter(|holding| holding.slug == slug) {
        prices.push(priced_row(store, holding, cap, now).await?);
    }
    Ok(prices)
}

/// Value of a held copy: the closed-trade median, else the lowest in-game ask.
fn held_worth(reading: &Reading) -> Option<u32> {
    reading
        .ours
        .map(|fair| fair.platinum.round().clamp(0.0, f64::from(u32::MAX)) as u32)
        .or(reading.ingame)
}

/// Price row of one holding, with its value at max rank `cap`.
async fn priced_row(
    store: &Store,
    holding: &Listed,
    cap: Option<u8>,
    now: Instant,
) -> Result<Priced, String> {
    let reading = priced(store, &holding.slug, &holding.of, now)
        .await
        .map_err(|e| e.to_string())?;
    let asked = store
        .stamped(&holding.slug, STATISTICS)
        .await
        .map_err(|e| e.to_string())?;

    let topped = match cap {
        // A copy already at max rank uses the reading above; reading the cache again would
        // return the same.
        Some(rank) if holding.of.rank != Some(rank) => {
            let topped = Variant {
                rank: Some(rank),
                ..holding.of.clone()
            };
            held_worth(
                &priced(store, &holding.slug, &topped, now)
                    .await
                    .map_err(|e| e.to_string())?,
            )
        }
        Some(_) => held_worth(&reading),
        None => None,
    };

    Ok(Priced {
        key: holding.of.key(),
        slug: holding.slug.clone(),
        item: holding.item.clone(),
        variant: holding.variant.clone(),
        ingame: reading.ingame,
        looked: reading.looked.map(|at| age(now, at)),
        asked: asked.map(|at| age(now, at)),
        traded: reading.traded,
        max_rank: cap,
        topped,
        worth: held_worth(&reading),
    })
}

/// Market cache quotes of items by game key, each as an unranked copy; items without a slug
/// are skipped.
pub async fn quoted(
    catalog: &Catalog,
    store: &Store,
    items: &[String],
) -> Result<Vec<Quote>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let asking = items.to_vec();
    let slugs: HashMap<String, String> = catalog
        .read(move |connection| catalog::slugs(connection, &asking))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    let now = seconds();
    let mut caps: HashMap<String, Option<u8>> = HashMap::new();
    let mut quotes = Vec::with_capacity(items.len());

    for item in items {
        let Some(slug) = slugs.get(item) else {
            continue;
        };

        // An item with ranks is traded by rank, and the market lists an unranked copy as rank 0,
        // not as no rank.
        let cap = match caps.get(slug) {
            Some(cap) => *cap,
            None => {
                let cap = store.max_rank(slug).await.map_err(|e| e.to_string())?;
                caps.insert(slug.clone(), cap);
                cap
            }
        };
        let unranked = Variant {
            rank: cap.map(|_| 0),
            ..Variant::default()
        };

        let reading = priced(store, slug, &unranked, now)
            .await
            .map_err(|e| e.to_string())?;

        quotes.push(Quote {
            item: item.clone(),
            slug: slug.clone(),
            variant: unranked.key(),
            platinum: reading.ours.map(|fair| fair.platinum.round() as u32),
            ingame: reading.ingame,
            looked: reading.looked,
            prospect: reading.prospect,
        });
    }
    Ok(quotes)
}

/// Closed-trade median of full sets by set slug; sets without one are omitted.
pub async fn quoted_sets(store: &Store, slugs: &[String]) -> Result<HashMap<String, u32>, String> {
    let now = seconds();
    let plain = Variant::default();
    let mut whole = HashMap::new();

    for slug in slugs {
        let reading = priced(store, slug, &plain, now)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(fair) = reading.ours {
            whole.insert(slug.clone(), fair.platinum.round() as u32);
        }
    }
    Ok(whole)
}

/// Price quote of one item from the market cache.
pub struct Quote {
    pub item: String,
    /// The item's market slug, for later requests.
    pub slug: String,
    /// Variant key of the quote.
    pub variant: String,
    /// Closed-trade median; `None` when the market cache has too few trades.
    pub platinum: Option<u32>,
    /// Lowest in-game ask; `None` when the market cache has none.
    pub ingame: Option<u32>,
    /// Time of the last order book read.
    pub looked: Option<Instant>,
    /// Sale outlook: sell at once to the highest bid, or list at the ask and wait.
    pub prospect: Prospect,
}

/// Market cache prices of every set the account holds a part of.
pub async fn set_prices(
    catalog: &Catalog,
    store: &Store,
    held: &Inventory,
) -> Result<Vec<Priced>, String> {
    let keys = held.items();
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let kits = catalog
        .read(move |connection| catalog::set_slugs(connection, &keys))
        .await
        .map_err(|e| e.to_string())?;

    let now = seconds();
    let mut prices = Vec::with_capacity(kits.len());
    for slug in kits {
        prices.push(set_price(store, &slug, now).await?);
    }
    Ok(prices)
}

/// Price row of one set, which the market trades only whole.
pub(crate) async fn set_price(store: &Store, slug: &str, now: Instant) -> Result<Priced, String> {
    let reading = priced(store, slug, &Variant::default(), now)
        .await
        .map_err(|e| e.to_string())?;
    let asked = store
        .stamped(slug, STATISTICS)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Priced {
        slug: slug.to_owned(),
        key: String::new(),
        item: slug.to_owned(),
        variant: String::new(),
        ingame: reading.ingame,
        looked: reading.looked.map(|at| age(now, at)),
        asked: asked.map(|at| age(now, at)),
        traded: reading.traded,
        max_rank: None,
        // A set has no rank of its own.
        topped: None,
        worth: held_worth(&reading),
    })
}

/// All open orders of one item, every variant and user, fetched from the market now; sell
/// orders first, then reachable users first, then cheapest first.
pub async fn listings(
    freshener: &Freshener,
    catalog: &Catalog,
    store: &Store,
    held: &Inventory,
    item: &str,
    variant: &str,
) -> Result<Orders, String> {
    let holding = match listed(catalog, store, held)
        .await?
        .into_iter()
        .find(|listed| listed.item == item && listed.variant == variant)
    {
        Some(holding) => holding,
        None => unheld(catalog, store, item)
            .await?
            .ok_or_else(|| format!("the market does not list {item}"))?,
    };

    let book = freshener.book_now(&holding.slug, &holding.of).await?;

    let name = names(catalog, item).await?;
    let mut rows: Vec<Listing> = book
        .rows()
        .iter()
        .map(|order| listed_row(order, &name))
        .collect();
    rows.sort_by_key(|row| (!row.selling, !row.reachable, row.platinum));

    let bulk = rows.iter().any(|row| row.per_trade > 1)
        || store
            .lots_allowed(&holding.slug)
            .await
            .map_err(|e| e.to_string())?
            .unwrap_or(false);
    Ok(Orders {
        rows,
        max_rank: store
            .max_rank(&holding.slug)
            .await
            .map_err(|e| e.to_string())?,
        bulk,
    })
}

/// Every holding with a market listing, with its slug and variant; others are skipped.
pub(crate) async fn listed(
    catalog: &Catalog,
    store: &Store,
    held: &Inventory,
) -> Result<Vec<Listed>, String> {
    let keys = held.items();
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let asking = keys.clone();
    let slugs: HashMap<String, String> = catalog
        .read(move |connection| catalog::slugs(connection, &asking))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    // A relic is one item on the market and four in the game, one per refinement.
    let refined: HashMap<String, (String, String)> = catalog
        .read(move |connection| catalog::relics(connection, &keys))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|relic| (relic.item, (relic.slug, relic.refinement)))
        .collect();

    let mut listed = Vec::with_capacity(held.holdings.len());
    for holding in &held.holdings {
        let (slug, of) = match refined.get(&holding.item) {
            Some((slug, refinement)) => (
                slug.clone(),
                Variant {
                    subtype: Some(refinement.clone()),
                    ..Variant::default()
                },
            ),
            None => {
                let Some(slug) = slugs.get(&holding.item) else {
                    continue;
                };
                let Some(of) = ranked(store, slug, holding.detail.as_ref()).await? else {
                    continue;
                };
                (slug.clone(), of)
            }
        };

        listed.push(Listed {
            item: holding.item.clone(),
            variant: holding.variant.0.clone(),
            slug,
            of,
        });
    }
    Ok(listed)
}

/// Market listing of an item the account does not hold: a relic by refinement, else the item
/// unranked, else a built prime as its set; `None` when none applies.
pub(crate) async fn unheld(
    catalog: &Catalog,
    store: &Store,
    item: &str,
) -> Result<Option<Listed>, String> {
    let asking = vec![item.to_owned()];
    let relic = catalog
        .read({
            let asking = asking.clone();
            move |connection| catalog::relics(connection, &asking)
        })
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .next();
    if let Some(relic) = relic {
        return Ok(Some(Listed {
            item: item.to_owned(),
            variant: String::new(),
            slug: relic.slug,
            of: Variant {
                subtype: Some(relic.refinement),
                ..Variant::default()
            },
        }));
    }

    let slug = catalog
        .read(move |connection| catalog::slugs(connection, &asking))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
        .map(|(_, slug)| slug);
    if let Some(slug) = slug {
        return Ok(ranked(store, &slug, None).await?.map(|of| Listed {
            item: item.to_owned(),
            variant: String::new(),
            slug,
            of,
        }));
    }

    let set = catalog
        .read(move |connection| catalog::sets(connection, "en"))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|set| set.built.as_deref() == Some(item));
    Ok(set.map(|set| Listed {
        item: item.to_owned(),
        variant: String::new(),
        slug: set.slug,
        of: Variant::default(),
    }))
}

/// Market variant of a copy among the item's known variants: a missing rank becomes 0 on a
/// ranked item, and a missing subtype becomes the only subtype or `regular`; `None` when the
/// game's detail has no market variant.
async fn ranked(
    store: &Store,
    slug: &str,
    detail: Option<&Detail>,
) -> Result<Option<Variant>, String> {
    let mut of = match Variant::of_detail(detail) {
        Some(of) => of,
        None => return Ok(None),
    };

    let asked = store
        .variants_asked(slug)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    if of.rank.is_none() && asked.iter().any(|one| one.rank.is_some()) {
        of.rank = Some(0);
    }
    if of.subtype.is_none() {
        let mut named: Vec<&String> = asked
            .iter()
            .filter(|one| one.rank == of.rank)
            .filter_map(|one| one.subtype.as_ref())
            .collect();
        named.dedup();
        of.subtype = match named.as_slice() {
            [only] => Some((*only).clone()),
            many if many.iter().any(|name| name.as_str() == REGULAR) => Some(REGULAR.to_owned()),
            _ => None,
        };
    }
    Ok(Some(of))
}

/// The item's name in English and Russian, the two whisper languages.
async fn names(catalog: &Catalog, item: &str) -> Result<(String, String), String> {
    let mut named = Vec::new();
    for lang in ["en", "ru"] {
        let asking = vec![item.to_owned()];
        let good = catalog
            .read(move |connection| catalog::goods(connection, lang, &asking))
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .next()
            .ok_or_else(|| format!("the catalog does not name {item}"))?;
        named.push(good.name);
    }
    Ok((named[0].clone(), named[1].clone()))
}

fn listed_row(order: &Order, name: &(String, String)) -> Listing {
    let russian = order.user.locale.as_deref() == Some("ru");
    let (template, called, deal) = match (russian, order.side == median_market::Side::Sell) {
        (true, true) => (WHISPER_RU, &name.1, "купить"),
        (true, false) => (WHISPER_RU, &name.1, "продать"),
        (false, true) => (WHISPER_EN, &name.0, "buy"),
        (false, false) => (WHISPER_EN, &name.0, "sell"),
    };

    let lot = order.per_trade.unwrap_or(1).max(1);
    let mut whisper = template
        .replace("{name}", &order.user.name)
        .replace("{deal}", deal)
        .replace("{item}", called)
        .replace("{platinum}", &order.platinum.to_string());
    if lot > 1 {
        let tail = if russian { LOT_RU } else { LOT_EN };
        whisper.push_str(
            &tail
                .replace("{lot}", &lot.to_string())
                .replace("{total}", &(order.platinum * lot).to_string()),
        );
    }

    Listing {
        name: order.user.name.clone(),
        reachable: order.user.status.reachable(),
        status: match order.user.status {
            Presence::Ingame => "ingame",
            Presence::Online => "online",
            Presence::Offline => "offline",
            Presence::Unknown => "unknown",
        }
        .to_owned(),
        selling: order.side == median_market::Side::Sell,
        platinum: order.platinum,
        quantity: order.quantity,
        per_trade: lot,
        rank: order.rank,
        subtype: order.subtype.clone(),
        whisper,
        reputation: order.user.reputation,
        updated: median_market::moment(&order.updated_at)
            .and_then(|at| u32::try_from(at).ok())
            .unwrap_or(0),
    }
}

fn age(now: Instant, then: Instant) -> u32 {
    now.saturating_sub(then).max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(side: &str, platinum: u32, locale: Option<&str>) -> Order {
        let locale = match locale {
            Some(code) => format!(r#","locale":"{code}""#),
            None => String::new(),
        };
        let json = format!(
            r#"{{"id":"x","type":"{side}","platinum":{platinum},"quantity":1,
                "createdAt":"2026-08-18T00:00:00Z","updatedAt":"2026-08-18T00:00:00Z",
                "user":{{"ingameName":"Tenno","status":"ingame"{locale}}}}}"#
        );
        serde_json::from_str(&json).expect("order parses")
    }

    fn named() -> (String, String) {
        (
            "Gram Prime Blueprint".to_owned(),
            "Чертёж Грам Прайм".to_owned(),
        )
    }

    const MOD: &str = "/Lotus/Upgrades/Mods/Pistol/Event/AmbulasEvent/Expert/SecondaryExplosion";

    fn stocked(copies: &[(&str, Option<u8>, u32)], built: &[&str]) -> Inventory {
        Inventory {
            holdings: copies
                .iter()
                .map(|(variant, rank, count)| median_core::Holding {
                    item: MOD.to_owned(),
                    variant: median_core::Variant((*variant).to_owned()),
                    detail: rank.map(Detail::Rank),
                    count: *count,
                })
                .collect(),
            equipped: built
                .iter()
                .map(|variant| median_core::Equipped {
                    item: MOD.to_owned(),
                    variant: median_core::Variant((*variant).to_owned()),
                })
                .collect(),
            ..Inventory::default()
        }
    }

    #[test]
    fn a_copy_standing_in_a_build_is_not_free() {
        let held = stocked(&[(r#"{"lvl":10}"#, Some(10), 1)], &[r#"{"lvl":10}"#]);

        assert_eq!(free(&held, MOD, "mod_rank=10"), Some(0));
    }

    #[test]
    fn a_second_copy_beside_the_built_one_is_free() {
        let held = stocked(&[(r#"{"lvl":10}"#, Some(10), 2)], &[r#"{"lvl":10}"#]);

        assert_eq!(free(&held, MOD, "mod_rank=10"), Some(1));
    }

    #[test]
    fn a_copy_ranked_up_leaves_the_rank_it_was_bought_at() {
        let held = stocked(&[(r#"{"lvl":10}"#, Some(10), 1)], &[]);

        assert_eq!(free(&held, MOD, "mod_rank=0"), Some(0));
        assert_eq!(free(&held, MOD, "mod_rank=10"), Some(1));
    }

    #[test]
    fn a_seller_is_whispered_a_wish_to_buy() {
        let row = listed_row(&order("sell", 20, Some("en")), &named());
        assert_eq!(
            row.whisper,
            "/w Tenno Hi! I want to buy: \"Gram Prime Blueprint\" for 20 platinum"
        );
    }

    #[test]
    fn a_buyer_is_whispered_a_wish_to_sell() {
        let row = listed_row(&order("buy", 15, Some("en")), &named());
        assert!(row.whisper.contains("I want to sell"));
        assert!(row.whisper.ends_with("for 15 platinum"));
    }

    #[test]
    fn the_whisper_is_written_in_the_language_the_other_side_reads() {
        let row = listed_row(&order("sell", 20, Some("ru")), &named());
        assert!(row.whisper.contains("Хочу купить"));
        assert!(row.whisper.contains("Чертёж Грам Прайм"));
    }

    #[test]
    fn a_lot_is_named_with_what_the_whole_lot_costs() {
        let json = r#"{"id":"x","type":"sell","platinum":36,"quantity":24,"perTrade":6,
            "createdAt":"2026-08-18T00:00:00Z","updatedAt":"2026-08-18T00:00:00Z",
            "user":{"ingameName":"Tenno","status":"ingame","locale":"en"}}"#;
        let order: Order = serde_json::from_str(json).expect("order parses");
        let row = listed_row(&order, &named());

        assert_eq!(row.per_trade, 6);
        assert!(
            row.whisper
                .contains("for 36 platinum each, 6 pieces (216 platinum)")
        );
    }

    #[test]
    fn one_at_a_time_says_nothing_about_lots() {
        let row = listed_row(&order("sell", 20, Some("en")), &named());
        assert_eq!(row.per_trade, 1);
        assert!(!row.whisper.contains("pieces"));
    }

    #[test]
    fn a_side_that_names_no_language_is_whispered_in_english() {
        let row = listed_row(&order("sell", 20, None), &named());
        assert!(row.whisper.contains("I want to buy"));
        assert!(row.whisper.contains("Gram Prime Blueprint"));
    }
}
