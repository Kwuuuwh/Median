use median_core::{Deal, Instant};
use rusqlite::{Connection, params};

use crate::account::store::StoreError;

/// One exchange to record in the account store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Made {
    /// Game unique name of the item; empty when no catalog item matches.
    pub item: String,
    pub slug: String,
    pub variant: String,
    pub selling: bool,
    /// Price of one copy in platinum.
    pub platinum: u32,
    pub quantity: u32,
    /// Label of the source that reported the exchange, e.g. `stream` or `hand`.
    pub origin: String,
    /// Id of the account's own order it filled; `None` when it filled none.
    pub own: Option<String>,
}

/// One exchange as stored in the account store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    pub at: Instant,
    pub item: String,
    pub slug: String,
    pub variant: String,
    pub selling: bool,
    pub platinum: u32,
    pub quantity: u32,
    pub origin: String,
    /// Whether it filled an order the agent placed.
    pub agent: bool,
}

impl Exchange {
    /// The exchange as a `Deal` for the earnings calculation.
    pub fn deal(&self) -> Deal {
        Deal {
            at: self.at,
            pair: format!("{}:{}", self.slug, self.variant),
            selling: self.selling,
            platinum: self.platinum,
            quantity: self.quantity,
        }
    }
}

/// Records one exchange at `at`.
pub fn record(connection: &Connection, made: &Made, at: Instant) -> Result<(), StoreError> {
    connection.execute(
        "INSERT INTO exchange (at, item, slug, variant, side, platinum, quantity, origin, own)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            at,
            made.item,
            made.slug,
            made.variant,
            side(made.selling),
            made.platinum,
            made.quantity,
            made.origin,
            made.own,
        ],
    )?;
    Ok(())
}

/// Whether this exchange is already recorded from another origin since `from`: matching
/// exchanges (same own order, side, price and quantity) from other origins outnumber those from
/// its own origin. Always `false` when it filled no own order.
pub fn recorded(connection: &Connection, made: &Made, from: Instant) -> Result<bool, StoreError> {
    let Some(own) = &made.own else {
        return Ok(false);
    };
    let (alike, elsewhere): (i64, i64) = connection.query_row(
        "SELECT COUNT(*) FILTER (WHERE origin = ?6), COUNT(*) FILTER (WHERE origin <> ?6)
           FROM exchange
          WHERE own = ?1 AND side = ?2 AND platinum = ?3 AND quantity = ?4 AND at >= ?5",
        params![
            own,
            side(made.selling),
            made.platinum,
            made.quantity,
            from,
            made.origin
        ],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(elsewhere > alike)
}

/// Every exchange from `from` on, newest first.
pub fn since(connection: &Connection, from: Instant) -> Result<Vec<Exchange>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT exchange.at, exchange.item, exchange.slug, exchange.variant, exchange.side,
                exchange.platinum, exchange.quantity, exchange.origin,
                COALESCE(own_order.by_agent, 0)
           FROM exchange LEFT JOIN own_order ON own_order.id = exchange.own
          WHERE exchange.at >= ?1 ORDER BY exchange.at DESC",
    )?;
    let rows = statement.query_map(params![from], |row| {
        Ok(Exchange {
            at: row.get(0)?,
            item: row.get(1)?,
            slug: row.get(2)?,
            variant: row.get(3)?,
            selling: row.get::<_, String>(4)? == SELL,
            platinum: row.get(5)?,
            quantity: row.get(6)?,
            origin: row.get(7)?,
            agent: row.get(8)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(StoreError::from)
}

const SELL: &str = "sell";

fn side(selling: bool) -> &'static str {
    match selling {
        true => SELL,
        false => "buy",
    }
}
