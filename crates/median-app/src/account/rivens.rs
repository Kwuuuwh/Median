use median_core::Instant;
use rusqlite::{Connection, params};

use crate::account::store::StoreError;

/// A riven bought for resale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RivenBought {
    /// warframe.market id of the auction it was bought from.
    pub auction: String,
    pub weapon: String,
    pub name: String,
    /// The riven as JSON of the IPC `Riven` type.
    pub riven: String,
    pub rank: u32,
    pub price: u32,
    pub bought: Instant,
    pub sold: Option<Instant>,
    pub sale: Option<u32>,
}

/// Records a bought riven; a riven already recorded for the same auction is left unchanged.
pub fn keep(connection: &Connection, riven: &RivenBought) -> Result<(), StoreError> {
    connection.execute(
        "INSERT OR IGNORE INTO riven_buy (auction, weapon, name, riven, rank, price, bought)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            riven.auction,
            riven.weapon,
            riven.name,
            riven.riven,
            riven.rank,
            riven.price,
            riven.bought
        ],
    )?;
    Ok(())
}

/// Records that the riven bought from `auction` sold for `sale` platinum at `at`; returns the
/// number of rows updated.
pub fn sell(
    connection: &Connection,
    auction: &str,
    sale: u32,
    at: Instant,
) -> Result<usize, StoreError> {
    Ok(connection.execute(
        "UPDATE riven_buy SET sold = ?2, sale = ?3 WHERE auction = ?1",
        params![auction, at, sale],
    )?)
}

/// Every riven bought, newest first.
pub fn bought(connection: &Connection) -> Result<Vec<RivenBought>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT auction, weapon, name, riven, rank, price, bought, sold, sale
           FROM riven_buy ORDER BY bought DESC",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(RivenBought {
                auction: row.get(0)?,
                weapon: row.get(1)?,
                name: row.get(2)?,
                riven: row.get(3)?,
                rank: row.get(4)?,
                price: row.get(5)?,
                bought: row.get(6)?,
                sold: row.get(7)?,
                sale: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Total purchase price in platinum of the bought rivens not sold yet; saturates at `u32::MAX`.
pub fn held(connection: &Connection) -> Result<u32, StoreError> {
    let spent: i64 = connection.query_row(
        "SELECT coalesce(sum(price), 0) FROM riven_buy WHERE sold IS NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(u32::try_from(spent).unwrap_or(u32::MAX))
}
