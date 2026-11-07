use median_core::Instant;
use rusqlite::{Connection, params};

use crate::account::store::StoreError;

/// A part of an item in the roadmap, and whether it is to be bought rather than farmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// The item in the roadmap the part goes into.
    pub item: String,
    pub part: String,
    pub buy: bool,
}

/// Adds an item to the profile's favourites, or removes it when `on` is false.
pub fn set_favourite(
    connection: &Connection,
    owner: i64,
    item: &str,
    on: bool,
    at: Instant,
) -> Result<(), StoreError> {
    match on {
        true => connection.execute(
            "INSERT OR IGNORE INTO favourite (profile, item, at) VALUES (?1, ?2, ?3)",
            params![owner, item, at],
        )?,
        false => connection.execute(
            "DELETE FROM favourite WHERE profile = ?1 AND item = ?2",
            params![owner, item],
        )?,
    };
    Ok(())
}

/// Every item the profile marked a favourite, oldest mark first.
pub fn favourites(connection: &Connection, owner: i64) -> Result<Vec<String>, StoreError> {
    keys(
        connection,
        owner,
        "SELECT item FROM favourite WHERE profile = ?1 ORDER BY at, item",
    )
}

/// Adds an item to the profile's roadmap, or removes it when `on` is false, together with the
/// choices made for its parts.
pub fn set_roadmap(
    connection: &Connection,
    owner: i64,
    item: &str,
    on: bool,
    at: Instant,
) -> Result<(), StoreError> {
    match on {
        true => connection.execute(
            "INSERT OR IGNORE INTO roadmap (profile, item, at) VALUES (?1, ?2, ?3)",
            params![owner, item, at],
        )?,
        false => connection.execute(
            "DELETE FROM roadmap WHERE profile = ?1 AND item = ?2",
            params![owner, item],
        )?,
    };
    Ok(())
}

/// Every item in the profile's roadmap, in the order it was put in.
pub fn roadmap(connection: &Connection, owner: i64) -> Result<Vec<String>, StoreError> {
    keys(
        connection,
        owner,
        "SELECT item FROM roadmap WHERE profile = ?1 ORDER BY at, item",
    )
}

/// Removes every item from the profile's roadmap, together with the choices made for their parts.
pub fn clear_roadmap(connection: &Connection, owner: i64) -> Result<(), StoreError> {
    connection.execute("DELETE FROM roadmap WHERE profile = ?1", params![owner])?;
    Ok(())
}

/// Records whether a part of an item in the profile's roadmap is to be bought.
pub fn set_choice(connection: &Connection, owner: i64, choice: &Choice) -> Result<(), StoreError> {
    connection.execute(
        "INSERT OR REPLACE INTO roadmap_part (profile, item, part, buy) VALUES (?1, ?2, ?3, ?4)",
        params![owner, choice.item, choice.part, choice.buy],
    )?;
    Ok(())
}

/// The choices made for the parts of items in the profile's roadmap.
pub fn choices(connection: &Connection, owner: i64) -> Result<Vec<Choice>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT item, part, buy FROM roadmap_part WHERE profile = ?1 ORDER BY item, part",
    )?;
    let rows = statement.query_map(params![owner], |row| {
        Ok(Choice {
            item: row.get(0)?,
            part: row.get(1)?,
            buy: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Item keys that `sql` returns for the profile, one per row.
fn keys(connection: &Connection, owner: i64, sql: &str) -> Result<Vec<String>, StoreError> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params![owner], |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}
