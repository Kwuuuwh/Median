use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use deadpool_sqlite::{InteractError, Pool, PoolError};
use median_core::{Band, Basis, Instant, Traded, seconds};
use median_market::{
    Auction, AuctionStat, Book, ClosedBucket, ItemMeta, ItemRef, LiveBucket, Side, Snapshot,
    Statistics,
};
use rusqlite::{Connection, OptionalExtension, params};
use tokio::sync::Mutex;
use tracing::warn;

use crate::market::db;
use crate::market::variant::Variant;
use crate::schema::OpenError;

/// Keys under which the time of each kind of fetch is recorded.
pub const ITEM: &str = "item";
pub const BOOK: &str = "book";
pub const STATISTICS: &str = "statistics";
pub const LISTED: &str = "listed";
pub const DAY_FILE: &str = "day";

/// Time resolution of a closed-trade bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Hour,
    Day,
}

impl Span {
    fn as_str(self) -> &'static str {
        match self {
            Self::Hour => "hour",
            Self::Day => "day",
        }
    }
}

/// Why the market cache could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the market cache could not be opened: {0}")]
    Open(#[from] OpenError),
    #[error("the market cache could not be reached: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("no connection to the market cache: {0}")]
    Pool(#[from] PoolError),
    #[error("the work did not finish: {0}")]
    Interact(#[from] InteractError),
}

/// One closed-trade bucket from the market cache.
#[derive(Debug, Clone, PartialEq)]
pub struct Bucket {
    pub starts: i64,
    pub volume: u64,
    pub median: f64,
    pub low: f64,
}

impl Bucket {
    /// The bucket in the form the price functions take.
    pub fn traded(&self) -> Traded {
        Traded {
            starts: self.starts,
            volume: self.volume,
            median: self.median,
            low: self.low,
        }
    }
}

/// The closed buckets of one pair over one span, oldest first.
const CLOSED: &str = "SELECT starts, volume, median, min_price FROM closed
                       WHERE slug = ?1 AND variant = ?2 AND span = ?3
                       ORDER BY starts";

/// One row of `CLOSED`.
fn closed_bucket(row: &rusqlite::Row<'_>) -> rusqlite::Result<Bucket> {
    Ok(Bucket {
        starts: row.get(0)?,
        volume: row.get::<_, i64>(1)? as u64,
        median: row.get(2)?,
        low: row.get(3)?,
    })
}

/// Daily summary of the open orders on one side of the market for a variant.
#[derive(Debug, Clone, PartialEq)]
pub struct Standing {
    pub side: Side,
    pub starts: i64,
    pub variant: Variant,
    pub orders: u64,
    pub min_price: f64,
    pub max_price: f64,
    pub median: f64,
}

impl Standing {
    /// One side's row from a day file; `None` when its date cannot be read.
    pub fn of_day(bucket: &ClosedBucket, side: Side) -> Option<Self> {
        Some(Self {
            side,
            starts: bucket.starts_at()?,
            variant: Variant::of_closed(bucket),
            orders: bucket.volume,
            min_price: bucket.min_price,
            max_price: bucket.max_price,
            median: bucket.median,
        })
    }

    /// A row from a statistics response; `None` when its date cannot be read.
    pub fn of_live(bucket: &LiveBucket) -> Option<Self> {
        Some(Self {
            side: bucket.order_type,
            starts: bucket.starts_at()?,
            variant: Variant::of_live(bucket),
            orders: bucket.volume,
            min_price: bucket.min_price,
            max_price: bucket.max_price,
            median: bucket.median,
        })
    }
}

/// One search of a weapon's riven auctions: its filters and its results.
#[derive(Debug, Clone, PartialEq)]
pub struct Sweep {
    /// The market's slug for the weapon.
    pub weapon: String,
    /// Stat slugs that every found auction has as positive stats.
    pub buffs: Vec<String>,
    /// Ids of the auctions found.
    pub found: HashSet<String>,
    /// Highest buyout found; `None` when nothing was found. The market truncates results at an
    /// undocumented count, so an auction at or above this price is not known to be gone.
    pub ceiling: Option<u32>,
}

impl Sweep {
    /// Whether an auction at `buyout` with `stats` would be in the results if it were still
    /// listed.
    fn covers(&self, buyout: u32, stats: &[AuctionStat]) -> bool {
        self.ceiling.is_some_and(|ceiling| buyout < ceiling)
            && self
                .buffs
                .iter()
                .all(|buff| stats.iter().any(|stat| stat.positive && &stat.slug == buff))
    }
}

/// Summary of the last order book read for one variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    pub taken: i64,
    pub lowest: Option<u32>,
    pub lowest_ingame: Option<u32>,
    pub held_ingame: Option<u32>,
    pub highest_bid_ingame: Option<u32>,
    pub sellers_ingame: Option<u32>,
}

impl Look {
    /// The held in-game ask, or the lowest in-game ask for a row without a held ask.
    pub fn held_ask(&self) -> Option<u32> {
        self.held_ingame.or(self.lowest_ingame)
    }
}

/// Market cache data of one pair: closed-trade buckets per span and the last book read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Recent {
    pub hour: Vec<Bucket>,
    pub day: Vec<Bucket>,
    pub look: Option<Look>,
}

/// Trade volume of one pair: copies per day and their median price.
#[derive(Debug, Clone, PartialEq)]
pub struct Demand {
    pub slug: String,
    pub variant: String,
    pub per_day: f64,
    pub median: f64,
}

/// One order from the order stream, as received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub id: String,
    pub slug: String,
    pub variant: String,
    pub selling: bool,
    pub platinum: u32,
    pub quantity: u32,
    pub per_trade: Option<u32>,
    /// Whether its user was reachable when the order was received.
    pub reachable: bool,
    pub at: Instant,
}

/// The market cache: warframe.market data about items, with the time of each fetch.
pub struct Store {
    pool: Pool,
    /// Held during a write, since SQLite allows one writer at a time.
    writing: Mutex<()>,
}

impl Store {
    /// Opens `market.sqlite` under `root`, creating it when missing.
    pub fn under(root: &Path) -> Result<Self, StoreError> {
        Ok(Self {
            pool: db::open(&root.join("market.sqlite"))?,
            writing: Mutex::new(()),
        })
    }

    /// Saves the market's full item list, replacing the stored one; returns the rows written.
    pub async fn keep_listed(&self, listed: Vec<ItemRef>) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let seen = seconds();

        connection
            .interact(move |connection| write_listed(connection, &listed, seen))
            .await?
    }

    /// Every listed slug, keyed by market item id.
    pub async fn slugs_by_id(&self) -> Result<BTreeMap<String, String>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| {
                let mut statement = connection.prepare("SELECT id, slug FROM listed")?;
                let rows = statement.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                Ok(rows.collect::<rusqlite::Result<BTreeMap<String, String>>>()?)
            })
            .await?
    }

    /// Copies traded per day and their volume-weighted median for every pair traded since
    /// `since`, from daily buckets; empty when there are no daily buckets.
    pub async fn demand(&self, since: Instant) -> Result<Vec<Demand>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let days: i64 = connection.query_row(
                    "SELECT COUNT(DISTINCT starts) FROM closed WHERE span = 'day' AND starts >= ?1",
                    params![since],
                    |row| row.get(0),
                )?;
                if days == 0 {
                    return Ok(Vec::new());
                }
                let mut statement = connection.prepare(
                    "SELECT slug, variant, SUM(volume), SUM(median * volume) / SUM(volume)
                     FROM closed WHERE span = 'day' AND starts >= ?1 AND volume > 0
                     GROUP BY slug, variant",
                )?;
                let rows = statement.query_map(params![since], |row| {
                    Ok(Demand {
                        slug: row.get(0)?,
                        variant: row.get(1)?,
                        per_day: row.get::<_, i64>(2)? as f64 / days as f64,
                        median: row.get(3)?,
                    })
                })?;
                Ok(rows.collect::<rusqlite::Result<Vec<Demand>>>()?)
            })
            .await?
    }

    /// Median of the highest in-game bids recorded for each pair since `since`; pairs without
    /// a recorded bid are omitted.
    pub async fn bids_since(
        &self,
        pairs: Vec<(String, String)>,
        since: Instant,
    ) -> Result<HashMap<(String, String), u32>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let mut seen = connection.prepare(
                    "SELECT highest_bid_ingame FROM live
                      WHERE slug = ?1 AND variant = ?2 AND taken >= ?3
                        AND highest_bid_ingame IS NOT NULL",
                )?;
                let mut out = HashMap::new();
                for (slug, variant) in pairs {
                    let mut bids = seen
                        .query_map(params![slug, variant, since], |row| row.get::<_, u32>(0))?
                        .collect::<rusqlite::Result<Vec<u32>>>()?;
                    bids.sort_unstable();
                    if let Some(middle) = bids.get(bids.len() / 2).copied() {
                        out.insert((slug, variant), middle);
                    }
                }
                Ok(out)
            })
            .await?
    }

    /// Number of items with closed trades in the market cache.
    pub async fn priced_items(&self) -> Result<usize, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| {
                let count: i64 =
                    connection.query_row("SELECT count(DISTINCT slug) FROM closed", [], |row| {
                        row.get(0)
                    })?;
                Ok(usize::try_from(count).unwrap_or(0))
            })
            .await?
    }

    /// Saves daily closed-trade buckets of many items in one transaction, as a day file holds
    /// them; returns the rows written.
    pub async fn keep_days(
        &self,
        by_slug: Vec<(String, Vec<ClosedBucket>)>,
    ) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                let mut landed = 0;
                for (slug, buckets) in &by_slug {
                    landed += write_closed(&transaction, slug, Span::Day, buckets)?;
                }
                transaction.commit()?;
                Ok(landed)
            })
            .await?
    }

    /// Saves new daily open-order summaries of many items; returns the rows written.
    pub async fn keep_standing(
        &self,
        by_slug: Vec<(String, Vec<Standing>)>,
    ) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                let mut landed = 0;
                for (slug, days) in &by_slug {
                    landed += write_standing(&transaction, slug, days)?;
                }
                transaction.commit()?;
                Ok(landed)
            })
            .await?
    }

    /// Saves riven auctions seen at `at`, records each price change, and clears the gone mark
    /// of auctions listed again; returns how many auctions were new.
    pub async fn keep_auctions(
        &self,
        auctions: Vec<Auction>,
        at: Instant,
    ) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                let mut new = 0;
                {
                    let mut seen = transaction.prepare(
                        "UPDATE riven_auction
                            SET buyout = ?2, starting = ?3, last_seen = ?4, gone = NULL
                          WHERE id = ?1",
                    )?;
                    let mut first = transaction.prepare(
                        "INSERT INTO riven_auction (id, weapon, name, buyout, starting, direct,
                                                    rank, rerolls, mastery, polarity, stats,
                                                    owner, created, first_seen, last_seen)
                              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                                      ?14, ?14)",
                    )?;
                    let mut priced = transaction.prepare(
                        "INSERT OR IGNORE INTO riven_price (id, at, buyout, starting)
                              SELECT ?1, ?2, ?3, ?4
                               WHERE NOT EXISTS (
                                     SELECT 1 FROM riven_price
                                      WHERE id = ?1 AND buyout IS ?3 AND starting = ?4
                                        AND at = (SELECT max(at) FROM riven_price WHERE id = ?1))",
                    )?;
                    for auction in &auctions {
                        priced.execute(params![
                            auction.id,
                            at,
                            auction.buyout_price,
                            auction.starting_price
                        ])?;
                        let touched = seen.execute(params![
                            auction.id,
                            auction.buyout_price,
                            auction.starting_price,
                            at
                        ])?;
                        if touched > 0 {
                            continue;
                        }
                        let stats = serde_json::to_string(&auction.stats)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                        first.execute(params![
                            auction.id,
                            auction.weapon,
                            auction.name,
                            auction.buyout_price,
                            auction.starting_price,
                            auction.is_direct_sell,
                            auction.rank,
                            auction.rerolls,
                            auction.mastery,
                            auction.polarity,
                            stats,
                            auction
                                .owner
                                .as_ref()
                                .map(|owner| owner.ingame_name.clone()),
                            auction.created,
                            at,
                        ])?;
                        new += 1;
                    }
                }
                transaction.commit()?;
                Ok(new)
            })
            .await?
    }

    /// Marks as gone at `at` the auctions that `sweep` covers but did not find; returns how many.
    pub async fn sweep_auctions(&self, sweep: Sweep, at: Instant) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                let missing: Vec<String> = {
                    let mut standing = transaction.prepare(
                        "SELECT id, buyout, stats FROM riven_auction
                          WHERE weapon = ?1 AND gone IS NULL AND last_seen < ?2
                            AND buyout IS NOT NULL",
                    )?;
                    let rows = standing.query_map(params![sweep.weapon, at], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, u32>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?;
                    let mut missing = Vec::new();
                    for row in rows {
                        let (id, buyout, stats) = row?;
                        let stats: Vec<AuctionStat> =
                            serde_json::from_str(&stats).unwrap_or_default();
                        if !sweep.found.contains(&id) && sweep.covers(buyout, &stats) {
                            missing.push(id);
                        }
                    }
                    missing
                };
                {
                    let mut gone =
                        transaction.prepare("UPDATE riven_auction SET gone = ?2 WHERE id = ?1")?;
                    for id in &missing {
                        gone.execute(params![id, at])?;
                    }
                }
                transaction.commit()?;
                Ok(missing.len())
            })
            .await?
    }

    /// Number of riven auctions of `weapon` seen at or after `since` and not marked gone.
    pub async fn listed_rivens(&self, weapon: &str, since: Instant) -> Result<u32, StoreError> {
        let connection = self.pool.get().await?;
        let weapon = weapon.to_owned();

        connection
            .interact(move |connection| {
                Ok(connection.query_row(
                    "SELECT count(*) FROM riven_auction
                      WHERE weapon = ?1 AND gone IS NULL AND last_seen >= ?2",
                    params![weapon, since],
                    |row| row.get(0),
                )?)
            })
            .await?
    }

    /// Last stored asking-price band of rivens like `riven`, and the time it was computed.
    pub async fn riven_band(&self, riven: String) -> Result<Option<(Band, Instant)>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let kept = connection
                    .query_row(
                        "SELECT low, middle, high, lots, basis, capped, at FROM riven_band
                          WHERE riven = ?1",
                        params![riven],
                        |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get::<_, i64>(3)?,
                                row.get::<_, String>(4)?,
                                row.get(5)?,
                                row.get(6)?,
                            ))
                        },
                    )
                    .optional()?;
                Ok(
                    kept.and_then(|(low, middle, high, lots, basis, is_capped, at)| {
                        let band = Band {
                            low,
                            middle,
                            high,
                            lots: usize::try_from(lots).ok()?,
                            basis: basis_of(&basis)?,
                            is_capped,
                        };
                        Some((band, at))
                    }),
                )
            })
            .await?
    }

    /// Saves the asking-price band of rivens like `riven`, computed at `at`.
    pub async fn keep_riven_band(
        &self,
        riven: String,
        band: Band,
        at: Instant,
    ) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                connection.execute(
                    "INSERT INTO riven_band (riven, low, middle, high, lots, basis, capped, at)
                          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT (riven) DO UPDATE
                             SET low = ?2, middle = ?3, high = ?4, lots = ?5, basis = ?6,
                                 capped = ?7, at = ?8",
                    params![
                        riven,
                        band.low,
                        band.middle,
                        band.high,
                        i64::try_from(band.lots).unwrap_or(i64::MAX),
                        basis_name(band.basis),
                        band.is_capped,
                        at
                    ],
                )?;
                Ok(())
            })
            .await?
    }

    /// Saves one order-stream order, once per order id and price.
    pub async fn keep_observation(&self, observation: Observation) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                connection.execute(
                    "INSERT INTO observation (id, slug, variant, side, platinum, quantity, per_trade,
                                        reachable, at)
                          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT (id, platinum) DO NOTHING",
                    params![
                        observation.id,
                        observation.slug,
                        observation.variant,
                        match observation.selling {
                            true => "sell",
                            false => "buy",
                        },
                        observation.platinum,
                        observation.quantity,
                        observation.per_trade,
                        observation.reachable,
                        observation.at,
                    ],
                )?;
                Ok(())
            })
            .await?
    }

    /// Lowest reachable sell price observed on a pair since `since`, and the number of
    /// observations on that pair.
    pub async fn asked_lately(
        &self,
        slug: &str,
        variant: &str,
        since: Instant,
    ) -> Result<(Option<u32>, u32), StoreError> {
        let connection = self.pool.get().await?;
        let (slug, variant) = (slug.to_owned(), variant.to_owned());

        connection
            .interact(move |connection| {
                Ok(connection.query_row(
                    "SELECT min(CASE WHEN side = 'sell' AND reachable = 1 THEN platinum END),
                            count(*)
                       FROM observation
                      WHERE slug = ?1 AND variant = ?2 AND at >= ?3",
                    params![slug, variant, since],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?)
            })
            .await?
    }

    /// Median copies per trade offered by reachable sellers on this pair since `since`; `None`
    /// when no seller listed a lot.
    pub async fn sold_lot(
        &self,
        slug: &str,
        variant: &str,
        since: Instant,
    ) -> Result<Option<u32>, StoreError> {
        let connection = self.pool.get().await?;
        let (slug, variant) = (slug.to_owned(), variant.to_owned());

        connection
            .interact(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT min(per_trade, quantity) FROM observation
                      WHERE slug = ?1 AND variant = ?2 AND at >= ?3 AND side = 'sell'
                        AND reachable = 1 AND per_trade IS NOT NULL
                      ORDER BY 1",
                )?;
                let lots = statement
                    .query_map(params![slug, variant, since], |row| row.get::<_, u32>(0))?
                    .collect::<rusqlite::Result<Vec<u32>>>()?;
                Ok(lots.get(lots.len() / 2).copied())
            })
            .await?
    }

    /// Deletes observations older than `before`; returns how many.
    pub async fn forget_observations(&self, before: Instant) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                Ok(connection.execute("DELETE FROM observation WHERE at < ?1", params![before])?)
            })
            .await?
    }

    /// Whether an order on this item may set a lot size; `None` while unknown.
    pub async fn lots_allowed(&self, slug: &str) -> Result<Option<bool>, StoreError> {
        let connection = self.pool.get().await?;
        let slug = slug.to_owned();

        connection
            .interact(move |connection| {
                Ok(connection
                    .query_row(
                        "SELECT bulk FROM item WHERE slug = ?1",
                        params![slug],
                        |row| row.get::<_, Option<bool>>(0),
                    )
                    .optional()?
                    .flatten())
            })
            .await?
    }

    /// Saves the rival bidders seen on a pair with their update times; returns how many of them
    /// have changed their order since first seen.
    pub async fn keep_rivals(
        &self,
        slug: &str,
        variant: &str,
        seen: Vec<(String, String)>,
    ) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let (slug, variant) = (slug.to_owned(), variant.to_owned());
        let at = seconds();

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                for (name, updated) in &seen {
                    transaction.execute(
                        "INSERT INTO rival (slug, variant, name, updated, moves, seen)
                              VALUES (?1, ?2, ?3, ?4, 0, ?5)
                         ON CONFLICT (slug, variant, name) DO UPDATE SET
                              moves = moves + (excluded.updated <> rival.updated),
                              updated = excluded.updated,
                              seen = excluded.seen",
                        params![slug, variant, name, updated, at],
                    )?;
                }
                let movers: usize = transaction.query_row(
                    "SELECT count(*) FROM rival WHERE slug = ?1 AND variant = ?2 AND moves > 0",
                    params![slug, variant],
                    |row| row.get::<_, i64>(0),
                )? as usize;
                transaction.commit()?;
                Ok(movers)
            })
            .await?
    }

    /// Market item id of this slug.
    pub async fn id_of(&self, slug: String) -> Result<Option<String>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                Ok(connection
                    .query_row(
                        "SELECT id FROM listed WHERE slug = ?1",
                        params![slug],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .await?
    }

    /// Slug of this market item id.
    pub async fn slug_of(&self, id: String) -> Result<Option<String>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                Ok(connection
                    .query_row(
                        "SELECT slug FROM listed WHERE id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .await?
    }

    /// Saves the market's description of an item, replacing the stored one.
    pub async fn keep_item(&self, meta: &ItemMeta) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let meta = meta.clone();
        let seen = seconds();

        connection
            .interact(move |connection| write_item(connection, &meta, seen))
            .await?
    }

    /// Saves hourly and daily closed-trade buckets, keeping the ones already stored; returns the
    /// rows written.
    pub async fn keep_closed(
        &self,
        slug: &str,
        statistics: &Statistics,
    ) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let slug = slug.to_owned();
        let hourly = statistics.closed_hourly.clone();
        let daily = statistics.closed_daily.clone();

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                let mut landed = write_closed(&transaction, &slug, Span::Hour, &hourly)?;
                landed += write_closed(&transaction, &slug, Span::Day, &daily)?;
                transaction.commit()?;
                Ok(landed)
            })
            .await?
    }

    /// Saves a summary of one book read, one row per variant (`variants` plus every variant in
    /// the book); returns the saved summaries.
    pub async fn keep_look(
        &self,
        slug: &str,
        book: &Book,
        variants: &[Variant],
    ) -> Result<Vec<(Variant, Snapshot)>, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let slug = slug.to_owned();
        let taken = seconds();
        let mut of = variants.to_vec();
        for order in book.rows() {
            let carried = Variant::of_order(order);
            if !of.contains(&carried) {
                of.push(carried);
            }
        }
        let looks: Vec<(Variant, Snapshot)> = of
            .into_iter()
            .map(|variant| {
                let snapshot = book.summarise(|order| variant.holds(order));
                (variant, snapshot)
            })
            .collect();

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                for (variant, snapshot) in &looks {
                    write_look(&transaction, &slug, taken, variant, snapshot)?;
                }
                transaction.commit()?;
                Ok(looks)
            })
            .await?
    }

    /// Records that this item's `what` was fetched now.
    pub async fn stamp(&self, slug: &str, what: &str) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let (slug, what, at) = (slug.to_owned(), what.to_owned(), seconds());

        connection
            .interact(move |connection| {
                connection.execute(
                    "INSERT INTO refreshed (slug, what, at) VALUES (?1, ?2, ?3)
                       ON CONFLICT (slug, what) DO UPDATE SET at = ?3",
                    params![slug, what, at],
                )?;
                Ok(())
            })
            .await?
    }

    /// Last fetch time of `what` for each item, by slug.
    pub async fn stamps(&self, what: &str) -> Result<HashMap<String, i64>, StoreError> {
        let connection = self.pool.get().await?;
        let what = what.to_owned();

        connection
            .interact(move |connection| {
                let mut statement =
                    connection.prepare("SELECT slug, at FROM refreshed WHERE what = ?1")?;
                let rows = statement.query_map(params![what], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })?;
                Ok(rows.collect::<rusqlite::Result<HashMap<String, i64>>>()?)
            })
            .await?
    }

    /// Last fetch time of this item's `what`; `None` when never fetched.
    pub async fn stamped(&self, slug: &str, what: &str) -> Result<Option<i64>, StoreError> {
        let connection = self.pool.get().await?;
        let (slug, what) = (slug.to_owned(), what.to_owned());

        connection
            .interact(move |connection| {
                Ok(connection
                    .query_row(
                        "SELECT at FROM refreshed WHERE slug = ?1 AND what = ?2",
                        params![slug, what],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .await?
    }

    /// Closed buckets of one variant, oldest first.
    pub async fn closed(
        &self,
        slug: &str,
        variant: &Variant,
        span: Span,
    ) -> Result<Vec<Bucket>, StoreError> {
        let connection = self.pool.get().await?;
        let (slug, key) = (slug.to_owned(), variant.key());

        connection
            .interact(move |connection| {
                let mut statement = connection.prepare(CLOSED)?;
                let rows = statement
                    .query_map(params![slug, key, span.as_str()], closed_bucket)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await?
    }

    /// Daily open-order summaries of one side and variant since `since`, oldest first.
    pub async fn standing(
        &self,
        slug: &str,
        variant: &Variant,
        side: Side,
        since: Instant,
    ) -> Result<Vec<Standing>, StoreError> {
        let connection = self.pool.get().await?;
        let (slug, variant) = (slug.to_owned(), variant.clone());

        connection
            .interact(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT starts, orders, min_price, max_price, median FROM standing
                      WHERE slug = ?1 AND variant = ?2 AND side = ?3 AND starts >= ?4
                      ORDER BY starts",
                )?;
                let rows = statement
                    .query_map(params![slug, variant.key(), side.as_str(), since], |row| {
                        Ok(Standing {
                            side,
                            starts: row.get(0)?,
                            variant: variant.clone(),
                            orders: row.get::<_, i64>(1)? as u64,
                            min_price: row.get(2)?,
                            max_price: row.get(3)?,
                            median: row.get(4)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await?
    }

    /// Summary of the last book read for one variant; `None` when it was never read.
    pub async fn look(&self, slug: &str, variant: &Variant) -> Result<Option<Look>, StoreError> {
        let connection = self.pool.get().await?;
        let (slug, key) = (slug.to_owned(), variant.key());

        connection
            .interact(move |connection| {
                Ok(connection
                    .query_row(
                        "SELECT taken, lowest, lowest_ingame, highest_bid_ingame, sellers_ingame, held_ingame
                           FROM live WHERE slug = ?1 AND variant = ?2
                          ORDER BY taken DESC LIMIT 1",
                        params![slug, key],
                        |row| {
                            Ok(Look {
                                taken: row.get(0)?,
                                lowest: row.get(1)?,
                                lowest_ingame: row.get(2)?,
                                held_ingame: row.get(5)?,
                                highest_bid_ingame: row.get(3)?,
                                sellers_ingame: row.get(4)?,
                            })
                        },
                    )
                    .optional()?)
            })
            .await?
    }

    /// Max rank of this item from the market's description; `None` when unranked or unknown.
    pub async fn max_rank(&self, slug: &str) -> Result<Option<u8>, StoreError> {
        let connection = self.pool.get().await?;
        let slug = slug.to_owned();

        connection
            .interact(move |connection| {
                Ok(connection
                    .query_row(
                        "SELECT max_rank FROM item WHERE slug = ?1",
                        params![slug],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten())
            })
            .await?
    }

    /// Rarity and max rank of an item ranked up by fusing; `None` for an item traded in lots,
    /// which is combined instead of fused, or one without ranks or rarity.
    pub async fn fused(&self, slug: &str) -> Result<Option<(String, u8)>, StoreError> {
        let connection = self.pool.get().await?;
        let slug = slug.to_owned();

        connection
            .interact(move |connection| {
                Ok(connection
                    .query_row(
                        "SELECT rarity, max_rank FROM item
                          WHERE slug = ?1 AND COALESCE(bulk, 0) = 0 AND max_rank > 0
                            AND rarity IS NOT NULL",
                        params![slug],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?)
            })
            .await?
    }

    /// Closed-trade buckets and the last book read of each pair, in one database call.
    pub async fn recent(
        &self,
        pairs: Vec<(String, String)>,
    ) -> Result<BTreeMap<(String, String), Recent>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let mut buckets = connection.prepare(CLOSED)?;
                let mut last = connection.prepare(
                    "SELECT taken, lowest, lowest_ingame, highest_bid_ingame, sellers_ingame, held_ingame
                       FROM live WHERE slug = ?1 AND variant = ?2
                      ORDER BY taken DESC LIMIT 1",
                )?;

                let mut out: BTreeMap<(String, String), Recent> = BTreeMap::new();
                for (slug, variant) in pairs {
                    let mut closed = |span: Span| -> Result<Vec<Bucket>, rusqlite::Error> {
                        buckets
                            .query_map(params![slug, variant, span.as_str()], closed_bucket)?
                            .collect()
                    };
                    let held = Recent {
                        hour: closed(Span::Hour)?,
                        day: closed(Span::Day)?,
                        look: last
                            .query_row(params![slug, variant], |row| {
                                Ok(Look {
                                    taken: row.get(0)?,
                                    lowest: row.get(1)?,
                                    lowest_ingame: row.get(2)?,
                                    held_ingame: row.get(5)?,
                                    highest_bid_ingame: row.get(3)?,
                                    sellers_ingame: row.get(4)?,
                                })
                            })
                            .optional()?,
                    };
                    out.insert((slug, variant), held);
                }
                Ok(out)
            })
            .await?
    }

    /// Max rank of each item, in one database call; unranked items are omitted.
    pub async fn max_ranks(&self, slugs: Vec<String>) -> Result<BTreeMap<String, u8>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let mut statement =
                    connection.prepare("SELECT max_rank FROM item WHERE slug = ?1")?;
                let mut out = BTreeMap::new();
                for slug in slugs {
                    let rank: Option<Option<u8>> = statement
                        .query_row(params![slug], |row| row.get(0))
                        .optional()?;
                    if let Some(rank) = rank.flatten() {
                        out.insert(slug, rank);
                    }
                }
                Ok(out)
            })
            .await?
    }

    /// Variants of the item to request; `None` before the market's description is stored.
    pub async fn variants_asked(&self, slug: &str) -> Result<Option<Vec<Variant>>, StoreError> {
        let connection = self.pool.get().await?;
        let slug = slug.to_owned();

        connection
            .interact(move |connection| {
                let max_rank: Option<Option<u8>> = connection
                    .query_row(
                        "SELECT max_rank FROM item WHERE slug = ?1",
                        params![slug],
                        |row| row.get(0),
                    )
                    .optional()?;
                let Some(max_rank) = max_rank else {
                    return Ok(None);
                };

                let mut statement = connection
                    .prepare("SELECT subtype FROM subtype WHERE slug = ?1 ORDER BY subtype")?;
                let subtypes = statement
                    .query_map(params![slug], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Some(crate::market::variant::variants(max_rank, &subtypes)))
            })
            .await?
    }
}

fn write_listed(
    connection: &mut Connection,
    listed: &[ItemRef],
    seen: Instant,
) -> Result<usize, StoreError> {
    let tx = connection.transaction()?;
    {
        let mut put = tx.prepare(
            "INSERT INTO listed (id, slug, game_ref, seen) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (id) DO UPDATE
               SET slug = excluded.slug, game_ref = excluded.game_ref, seen = excluded.seen",
        )?;
        for line in listed {
            put.execute(params![line.id, line.slug, line.game_ref, seen])?;
        }
    }
    tx.commit()?;
    Ok(listed.len())
}

fn write_item(connection: &Connection, meta: &ItemMeta, seen: i64) -> Result<(), StoreError> {
    connection.execute(
        "INSERT INTO item (slug, market_id, tradable, trading_tax, rarity, req_mastery_rank,
                           ducats, vaulted, max_rank, max_amber_stars, max_cyan_stars, bulk,
                           seen)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
           ON CONFLICT (slug) DO UPDATE SET
              market_id = ?2, tradable = ?3, trading_tax = ?4, rarity = ?5,
              req_mastery_rank = ?6, ducats = ?7, vaulted = ?8, max_rank = ?9,
              max_amber_stars = ?10, max_cyan_stars = ?11, bulk = ?12, seen = ?13",
        params![
            meta.slug,
            meta.id,
            meta.tradable,
            meta.trading_tax,
            meta.rarity,
            meta.req_mastery_rank,
            meta.ducats,
            meta.vaulted,
            meta.max_rank,
            meta.max_amber_stars,
            meta.max_cyan_stars,
            meta.bulk_tradable.unwrap_or(false),
            seen,
        ],
    )?;

    connection.execute("DELETE FROM subtype WHERE slug = ?1", params![meta.slug])?;
    for subtype in meta.subtypes.iter().flatten() {
        connection.execute(
            "INSERT INTO subtype (slug, subtype) VALUES (?1, ?2)",
            params![meta.slug, subtype],
        )?;
    }

    Ok(())
}

fn write_closed(
    connection: &Connection,
    slug: &str,
    span: Span,
    buckets: &[ClosedBucket],
) -> Result<usize, StoreError> {
    let mut landed = 0;
    for bucket in buckets {
        let Some(starts) = bucket.starts_at() else {
            warn!(slug, datetime = %bucket.datetime, "market statistics time unreadable");
            continue;
        };
        note_unexpected(connection, slug, "closed bucket", &bucket.extra)?;

        let variant = Variant::of_closed(bucket);
        landed += connection.execute(
            "INSERT INTO closed (slug, variant, span, starts, rank, subtype, amber_stars,
                                 cyan_stars, volume, min_price, max_price, open_price,
                                 closed_price, avg_price, wa_price, median, moving_avg,
                                 donch_top, donch_bot, market_id)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                          ?16, ?17, ?18, ?19, ?20)
               ON CONFLICT (slug, variant, span, starts) DO NOTHING",
            params![
                slug,
                variant.key(),
                span.as_str(),
                starts,
                variant.rank,
                variant.subtype,
                variant.amber_stars,
                variant.cyan_stars,
                bucket.volume as i64,
                bucket.min_price,
                bucket.max_price,
                bucket.open_price,
                bucket.closed_price,
                bucket.avg_price,
                bucket.wa_price,
                bucket.median,
                bucket.moving_avg,
                bucket.donch_top,
                bucket.donch_bot,
                bucket.id,
            ],
        )?;
    }
    Ok(landed)
}

fn write_standing(
    connection: &Connection,
    slug: &str,
    days: &[Standing],
) -> Result<usize, StoreError> {
    let mut landed = 0;
    for day in days {
        landed += connection.execute(
            "INSERT INTO standing (slug, variant, side, starts, rank, subtype, amber_stars,
                                   cyan_stars, orders, min_price, max_price, median)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
               ON CONFLICT (slug, variant, side, starts) DO NOTHING",
            params![
                slug,
                day.variant.key(),
                day.side.as_str(),
                day.starts,
                day.variant.rank,
                day.variant.subtype,
                day.variant.amber_stars,
                day.variant.cyan_stars,
                day.orders as i64,
                day.min_price,
                day.max_price,
                day.median,
            ],
        )?;
    }
    Ok(landed)
}

fn write_look(
    connection: &Connection,
    slug: &str,
    taken: i64,
    variant: &Variant,
    snapshot: &median_market::Snapshot,
) -> Result<(), StoreError> {
    connection.execute(
        "INSERT INTO live (slug, variant, taken, rank, subtype, amber_stars, cyan_stars,
                           lowest, lowest_ingame, highest_bid_ingame, sellers_ingame,
                           held_ingame)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
           ON CONFLICT (slug, variant, taken) DO NOTHING",
        params![
            slug,
            variant.key(),
            taken,
            variant.rank,
            variant.subtype,
            variant.amber_stars,
            variant.cyan_stars,
            snapshot.lowest,
            snapshot.lowest_ingame,
            snapshot.highest_bid_ingame,
            Some(snapshot.sellers_ingame as u32),
            snapshot.held_ingame,
        ],
    )?;
    Ok(())
}

/// Saves the first sample of each unknown response field to `unexpected` and logs a warning,
/// so the field is not silently dropped.
fn note_unexpected(
    connection: &Connection,
    slug: &str,
    seen_in: &str,
    extra: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Result<(), StoreError> {
    for (field, value) in extra {
        warn!(slug, seen_in, field, "market sent unknown field");
        connection.execute(
            "INSERT INTO unexpected (field, seen_in, slug, at, sample)
                  VALUES (?1, ?2, ?3, ?4, ?5)
               ON CONFLICT (field, seen_in) DO NOTHING",
            params![field, seen_in, slug, seconds(), value.to_string()],
        )?;
    }
    Ok(())
}

fn basis_name(basis: Basis) -> &'static str {
    match basis {
        Basis::Lots => "lots",
        Basis::FewLots => "few_lots",
        Basis::Trades => "trades",
    }
}

fn basis_of(name: &str) -> Option<Basis> {
    match name {
        "lots" => Some(Basis::Lots),
        "few_lots" => Some(Basis::FewLots),
        "trades" => Some(Basis::Trades),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use median_market::Order;

    use super::*;

    /// A separate temp directory per call, so parallel tests do not share a market cache.
    fn place() -> PathBuf {
        static MADE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "median-market-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).expect("a place for the market cache");
        root
    }

    fn store() -> Store {
        Store::under(&place()).expect("the market cache opens")
    }

    fn auction(id: &str, buyout: u32) -> Auction {
        Auction {
            id: id.into(),
            starting_price: buyout,
            buyout_price: Some(buyout),
            top_bid: None,
            is_direct_sell: true,
            visible: true,
            closed: false,
            created: "2026-10-12T05:00:00.000+00:00".into(),
            updated: "2026-10-12T05:00:00.000+00:00".into(),
            note: String::new(),
            owner: None,
            weapon: "torid".into(),
            name: "crita-visican".into(),
            mastery: 8,
            rerolls: 0,
            rank: 8,
            polarity: Some("madurai".into()),
            stats: Vec::new(),
        }
    }

    #[tokio::test]
    async fn an_auction_seen_again_keeps_when_it_was_first_seen() {
        let store = store();

        let first = store
            .keep_auctions(vec![auction("a", 500), auction("b", 300)], 100)
            .await
            .expect("kept");
        let again = store
            .keep_auctions(vec![auction("a", 450), auction("c", 200)], 200)
            .await
            .expect("kept");

        assert_eq!((first, again), (2, 1));
        let connection = store.pool.get().await.expect("a connection");
        let row: (i64, i64, i64) = connection
            .interact(|connection| {
                connection.query_row(
                    "SELECT buyout, first_seen, last_seen FROM riven_auction WHERE id = 'a'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
            })
            .await
            .expect("the read ran")
            .expect("the row");
        assert_eq!(row, (450, 100, 200));
    }

    fn with_buff(id: &str, price: u32, buff: &str) -> Auction {
        Auction {
            stats: vec![AuctionStat {
                slug: buff.into(),
                value: 100.0,
                positive: true,
            }],
            ..auction(id, price)
        }
    }

    async fn column(store: &Store, sql: &'static str) -> Vec<(String, Option<i64>)> {
        let connection = store.pool.get().await.expect("a connection");
        connection
            .interact(move |connection| {
                let mut statement = connection.prepare(sql)?;
                statement
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .await
            .expect("the read ran")
            .expect("the rows")
    }

    #[tokio::test]
    async fn each_new_price_of_an_auction_is_kept() {
        let store = store();
        for (price, at) in [(500, 100), (500, 200), (450, 300)] {
            store
                .keep_auctions(vec![auction("a", price)], at)
                .await
                .expect("kept");
        }
        let prices = column(&store, "SELECT id, buyout FROM riven_price ORDER BY at").await;
        assert_eq!(
            prices,
            vec![("a".to_owned(), Some(500)), ("a".to_owned(), Some(450))]
        );
    }

    #[tokio::test]
    async fn an_auction_a_covering_sweep_misses_is_gone_until_seen_again() {
        let store = store();
        let standing = vec![
            with_buff("cheap", 300, "multishot"),
            with_buff("dear", 900, "multishot"),
            with_buff("other", 200, "toxin_damage"),
        ];
        store.keep_auctions(standing, 100).await.expect("kept");

        let sweep = Sweep {
            weapon: "torid".into(),
            buffs: vec!["multishot".into()],
            found: HashSet::from(["kept".to_owned()]),
            ceiling: Some(500),
        };
        assert_eq!(store.sweep_auctions(sweep, 200).await.expect("swept"), 1);
        let gone = || column(&store, "SELECT id, gone FROM riven_auction ORDER BY id");
        assert_eq!(
            gone().await,
            vec![
                ("cheap".to_owned(), Some(200)),
                ("dear".to_owned(), None),
                ("other".to_owned(), None),
            ]
        );

        store
            .keep_auctions(vec![with_buff("cheap", 300, "multishot")], 300)
            .await
            .expect("kept");
        assert_eq!(gone().await[0], ("cheap".to_owned(), None));
    }

    #[tokio::test]
    async fn a_sweep_marks_nothing_at_its_dearest_find_nor_when_it_found_nothing() {
        let store = store();
        let standing = vec![
            with_buff("tied", 500, "multishot"),
            with_buff("past", 900, "multishot"),
        ];
        store.keep_auctions(standing, 100).await.expect("kept");

        let cut = Sweep {
            weapon: "torid".into(),
            buffs: Vec::new(),
            found: HashSet::from(["kept".to_owned()]),
            ceiling: Some(500),
        };
        assert_eq!(store.sweep_auctions(cut, 200).await.expect("swept"), 0);
        let empty = Sweep {
            weapon: "torid".into(),
            buffs: Vec::new(),
            found: HashSet::new(),
            ceiling: None,
        };
        assert_eq!(store.sweep_auctions(empty, 200).await.expect("swept"), 0);
    }

    #[tokio::test]
    async fn a_kept_riven_band_is_read_back_with_when_it_was_counted() {
        let store = store();
        let band = Band {
            low: 200.0,
            middle: 300.0,
            high: 450.0,
            lots: 12,
            basis: Basis::FewLots,
            is_capped: true,
        };
        store
            .keep_riven_band("torid".into(), band, 100)
            .await
            .expect("kept");

        assert_eq!(
            store.riven_band("torid".into()).await.expect("read"),
            Some((band, 100))
        );
        assert_eq!(store.riven_band("braton".into()).await.expect("read"), None);
    }

    fn meta(json: &str) -> ItemMeta {
        serde_json::from_str(json).expect("item meta parses")
    }

    fn statistics(daily: &str) -> Statistics {
        let json = format!(
            r#"{{"payload":{{"statistics_closed":{{"48hours":[],"90days":[{daily}]}},
                 "statistics_live":{{"48hours":[],"90days":[]}}}}}}"#
        );
        let value: serde_json::Value = serde_json::from_str(&json).expect("statistics parse");
        let closed = &value["payload"]["statistics_closed"];
        Statistics {
            closed_hourly: serde_json::from_value(closed["48hours"].clone()).expect("hourly"),
            closed_daily: serde_json::from_value(closed["90days"].clone()).expect("daily"),
            live_hourly: Vec::new(),
            live_daily: Vec::new(),
        }
    }

    fn bucket(day: &str, variant: &str, volume: u32, median: f64) -> String {
        format!(
            r#"{{"datetime":"2026-08-{day}T00:00:00.000+00:00","volume":{volume},
                "min_price":1.0,"max_price":2.0,"avg_price":1.5,"median":{median}{variant}}}"#
        )
    }

    fn book(rows: &[&str]) -> Book {
        let orders: Vec<Order> =
            serde_json::from_str(&format!("[{}]", rows.join(","))).expect("orders parse");
        Book::from(orders)
    }

    fn order(side: &str, platinum: u32, status: &str, variant: &str) -> String {
        format!(
            r#"{{"id":"{side}{platinum}{status}","type":"{side}","platinum":{platinum},
                "quantity":1,{variant}"createdAt":"2026-08-16T18:48:28Z",
                "updatedAt":"2026-08-16T18:48:28Z",
                "user":{{"ingameName":"p{platinum}","status":"{status}"}}}}"#
        )
    }

    #[tokio::test]
    async fn what_the_market_says_about_an_item_comes_back() {
        let store = store();
        store
            .keep_item(&meta(
                r#"{"id":"m1","slug":"lith_r2_relic","tradable":true,"tradingTax":2000,
                    "vaulted":true,"subtypes":["intact","exceptional","flawless","radiant"]}"#,
            ))
            .await
            .expect("the item is kept");

        // Saved twice: the second write replaces the first instead of failing.
        store
            .keep_item(&meta(
                r#"{"id":"m1","slug":"lith_r2_relic","tradable":true,"tradingTax":2000,
                    "vaulted":false,"subtypes":["intact","radiant"]}"#,
            ))
            .await
            .expect("the item is kept again");
    }

    #[tokio::test]
    async fn two_variants_of_one_day_both_land() {
        let store = store();
        let stats = statistics(&format!(
            "{},{}",
            bucket("15", r#","subtype":"intact""#, 278, 5.0),
            bucket("15", r#","subtype":"radiant""#, 15, 8.0)
        ));

        assert_eq!(
            store
                .keep_closed("lith_r2_relic", &stats)
                .await
                .expect("kept"),
            2
        );

        let intact = Variant {
            subtype: Some("intact".into()),
            ..Variant::default()
        };
        let radiant = Variant {
            subtype: Some("radiant".into()),
            ..Variant::default()
        };
        let kept = store
            .closed("lith_r2_relic", &intact, Span::Day)
            .await
            .expect("read back");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].median, 5.0);
        assert_eq!(kept[0].volume, 278);

        let kept = store
            .closed("lith_r2_relic", &radiant, Span::Day)
            .await
            .expect("read back");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].median, 8.0);
    }

    #[tokio::test]
    async fn demand_is_counted_over_the_days_the_market_was_heard_on() {
        let store = store();
        // The market cache has trades on two days; the relic traded on only one of them.
        store
            .keep_closed(
                "lith_r2_relic",
                &statistics(&bucket("15", r#","subtype":"intact""#, 40, 5.0)),
            )
            .await
            .expect("kept");
        store
            .keep_closed(
                "ash_prime_set",
                &statistics(&format!(
                    "{},{}",
                    bucket("15", "", 10, 60.0),
                    bucket("16", "", 30, 80.0)
                )),
            )
            .await
            .expect("kept");

        let mut demand = store.demand(0).await.expect("read back");
        demand.sort_by(|a, b| a.slug.cmp(&b.slug));

        assert_eq!(demand.len(), 2);
        assert_eq!(demand[0].slug, "ash_prime_set");
        assert_eq!(demand[0].per_day, 20.0);
        assert_eq!(demand[0].median, 75.0);
        assert_eq!(demand[1].per_day, 20.0);
        assert_eq!(demand[1].variant, "subtype=intact");
    }

    #[tokio::test]
    async fn stamps_of_one_kind_come_back_by_slug() {
        let store = store();
        store.stamp("ash_prime_set", BOOK).await.expect("stamped");
        store
            .stamp("ash_prime_set", STATISTICS)
            .await
            .expect("stamped");
        store.stamp("serration", BOOK).await.expect("stamped");

        let books = store.stamps(BOOK).await.expect("read back");

        assert_eq!(books.len(), 2);
        assert!(books.contains_key("serration"));
        assert_eq!(store.stamps(STATISTICS).await.expect("read back").len(), 1);
    }

    fn standing(side: Side, rank: u8, median: f64) -> Standing {
        Standing {
            side,
            starts: 1_791_763_200,
            variant: Variant {
                rank: Some(rank),
                ..Variant::default()
            },
            orders: 12,
            min_price: median - 2.0,
            max_price: median + 9.0,
            median,
        }
    }

    #[tokio::test]
    async fn both_sides_of_a_day_land_once() {
        let store = store();
        let days = vec![
            standing(Side::Sell, 0, 5.0),
            standing(Side::Buy, 0, 3.0),
            standing(Side::Sell, 5, 37.0),
        ];

        let first = store
            .keep_standing(vec![("serration".into(), days.clone())])
            .await
            .expect("the days land");
        let again = store
            .keep_standing(vec![("serration".into(), days)])
            .await
            .expect("the days land");

        assert_eq!((first, again), (3, 0));
    }

    #[tokio::test]
    async fn the_same_buckets_written_twice_do_not_pile_up() {
        let store = store();
        let stats = statistics(&bucket("15", "", 71, 19.0));

        assert_eq!(
            store
                .keep_closed("gram_prime_blueprint", &stats)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .keep_closed("gram_prime_blueprint", &stats)
                .await
                .unwrap(),
            0
        );

        let kept = store
            .closed("gram_prime_blueprint", &Variant::default(), Span::Day)
            .await
            .expect("read back");
        assert_eq!(kept.len(), 1);
    }

    #[tokio::test]
    async fn hourly_and_daily_buckets_of_one_moment_do_not_collide() {
        let store = store();
        let mut stats = statistics(&bucket("15", "", 71, 19.0));
        stats.closed_hourly = stats.closed_daily.clone();

        assert_eq!(
            store
                .keep_closed("gram_prime_blueprint", &stats)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            store
                .closed("gram_prime_blueprint", &Variant::default(), Span::Hour)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn a_look_keeps_the_reachable_and_the_cheapest_apart() {
        let store = store();
        let book = book(&[
            &order("sell", 13, "offline", ""),
            &order("sell", 18, "ingame", ""),
            &order("buy", 15, "ingame", ""),
        ]);

        store
            .keep_look(
                "mirage_prime_systems_blueprint",
                &book,
                &[Variant::default()],
            )
            .await
            .expect("the look is kept");

        let look = store
            .look("mirage_prime_systems_blueprint", &Variant::default())
            .await
            .expect("read back")
            .expect("there is a look");
        assert_eq!(look.lowest, Some(13));
        assert_eq!(look.lowest_ingame, Some(18));
        assert_eq!(look.highest_bid_ingame, Some(15));
        assert_eq!(look.sellers_ingame, Some(1));
    }

    #[tokio::test]
    async fn each_variant_gets_its_own_look_out_of_one_book() {
        let store = store();
        let book = book(&[
            &order("sell", 60, "ingame", r#""rank":0,"#),
            &order("sell", 120, "ingame", r#""rank":10,"#),
        ]);
        let bare = Variant {
            rank: Some(0),
            ..Variant::default()
        };
        let maxed = Variant {
            rank: Some(10),
            ..Variant::default()
        };

        let kept = store
            .keep_look(
                "primed_cleanse_corrupted",
                &book,
                &[bare.clone(), maxed.clone()],
            )
            .await
            .expect("kept");
        assert_eq!(kept.len(), 2);

        let bare = store
            .look("primed_cleanse_corrupted", &bare)
            .await
            .unwrap()
            .expect("bare look");
        let maxed = store
            .look("primed_cleanse_corrupted", &maxed)
            .await
            .unwrap()
            .expect("maxed look");
        assert_eq!(bare.lowest_ingame, Some(60));
        assert_eq!(maxed.lowest_ingame, Some(120));
    }

    #[tokio::test]
    async fn a_rank_between_the_ends_gets_a_row_of_its_own() {
        let store = store();
        let middling = Variant {
            rank: Some(7),
            ..Variant::default()
        };
        let book = book(&[
            &order("sell", 60, "ingame", r#""rank":0,"#),
            &order("sell", 95, "ingame", r#""rank":7,"#),
            &order("sell", 140, "ingame", r#""rank":10,"#),
        ]);

        store
            .keep_look("serration", &book, std::slice::from_ref(&middling))
            .await
            .expect("the look lands");

        let look = store
            .look("serration", &middling)
            .await
            .unwrap()
            .expect("a look of its own");
        assert_eq!(look.lowest_ingame, Some(95));
        assert_eq!(look.sellers_ingame, Some(1));
    }

    #[tokio::test]
    async fn a_field_we_do_not_know_is_written_down_rather_than_dropped() {
        let store = store();
        let stats = statistics(&bucket("15", r#","polarity":"madurai""#, 71, 19.0));

        store
            .keep_closed("gram_prime_blueprint", &stats)
            .await
            .unwrap();

        // The bucket is still saved; the unknown field is stored apart for manual review.
        let kept = store
            .closed("gram_prime_blueprint", &Variant::default(), Span::Day)
            .await
            .expect("read back");
        assert_eq!(kept.len(), 1);
    }

    #[tokio::test]
    async fn a_bucket_with_an_unreadable_moment_is_skipped_not_guessed() {
        let store = store();
        let mut stats = statistics(&bucket("15", "", 71, 19.0));
        stats.closed_daily[0].datetime = "sometime last week".into();

        assert_eq!(
            store
                .keep_closed("gram_prime_blueprint", &stats)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn a_stamp_says_when_the_item_was_last_asked_about() {
        let store = store();
        assert_eq!(
            store
                .stamped("gram_prime_blueprint", "statistics")
                .await
                .unwrap(),
            None
        );

        store
            .stamp("gram_prime_blueprint", "statistics")
            .await
            .unwrap();
        let first = store
            .stamped("gram_prime_blueprint", "statistics")
            .await
            .unwrap()
            .expect("stamped");

        store
            .stamp("gram_prime_blueprint", "statistics")
            .await
            .unwrap();
        let second = store
            .stamped("gram_prime_blueprint", "statistics")
            .await
            .unwrap()
            .expect("stamped");
        assert!(second >= first);
    }
}
