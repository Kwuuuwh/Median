use median_core::Instant;
use median_market::Mine;
use rusqlite::{Connection, OptionalExtension, params};

use crate::account::store::StoreError;

/// One of the account's own orders, as stored in the account store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub id: String,
    pub item_id: String,
    pub slug: Option<String>,
    pub variant: String,
    pub selling: bool,
    pub platinum: u32,
    pub quantity: u32,
    pub per_trade: u32,
    pub visible: bool,
    /// Whether the agent placed it.
    pub by_agent: bool,
    /// Expected value of the opportunity the agent placed it for; `None` for other orders.
    pub promised: Option<u32>,
    /// Creation time on warframe.market, as the raw timestamp string.
    pub created_at: String,
    /// Last update time on warframe.market, as the raw timestamp string.
    pub updated_at: String,
}

/// One order from the market's response, with its slug from the market cache and its variant key.
pub type Answered = (Mine, Option<String>, String);

const PUT: &str = "
INSERT INTO own_order
  (id, item_id, slug, variant, side, platinum, quantity, per_trade, visible,
   created_at, updated_at, seen, gone)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL)
ON CONFLICT (id) DO UPDATE SET
  -- The market's own answer about our orders carries no item id, so the slug it comes with
  -- is empty; an order never changes what it is about, so what is known already stands.
  slug = COALESCE(excluded.slug, own_order.slug), platinum = excluded.platinum,
  quantity = excluded.quantity, per_trade = excluded.per_trade,
  visible = excluded.visible, updated_at = excluded.updated_at,
  seen = excluded.seen, gone = NULL";

/// Kind of change recorded for an own order in `order_step`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Placed,
    Moved,
    Gone,
}

impl Step {
    fn as_str(self) -> &'static str {
        match self {
            Step::Placed => "placed",
            Step::Moved => "moved",
            Step::Gone => "gone",
        }
    }
}

/// Records a step of order `id` at `at`: placed or moved to `platinum` for `quantity` copies,
/// or gone.
fn step(
    connection: &Connection,
    id: &str,
    at: Instant,
    (platinum, quantity): (u32, u32),
    step: Step,
) -> Result<(), StoreError> {
    connection
        .prepare_cached(
            "INSERT INTO order_step (own, at, platinum, quantity, step) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?
        .execute(params![id, at, platinum, quantity, step.as_str()])?;
    Ok(())
}

/// Records a step when the market's response about an order differs from the account store:
/// `Placed` for a new or previously gone order, `Moved` when price or quantity changed.
fn note(connection: &Connection, (order, _, _): &Answered, at: Instant) -> Result<(), StoreError> {
    let held: Option<(u32, u32, Option<Instant>)> = connection
        .prepare_cached("SELECT platinum, quantity, gone FROM own_order WHERE id = ?1")?
        .query_row(params![order.id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .optional()?;
    let answered = (order.platinum, order.quantity);
    match held {
        None | Some((_, _, Some(_))) => step(connection, &order.id, at, answered, Step::Placed),
        Some((platinum, quantity, None)) if (platinum, quantity) != answered => {
            step(connection, &order.id, at, answered, Step::Moved)
        }
        Some(_) => Ok(()),
    }
}

/// Records a `Gone` step for an open order; does nothing when the order is already gone.
fn note_gone(connection: &Connection, id: &str, at: Instant) -> Result<(), StoreError> {
    connection
        .prepare_cached(
            "INSERT INTO order_step (own, at, platinum, quantity, step)
             SELECT id, ?2, platinum, quantity, ?3 FROM own_order
              WHERE id = ?1 AND gone IS NULL",
        )?
        .execute(params![id, at, Step::Gone.as_str()])?;
    Ok(())
}

/// Seconds a stored order stays open while missing from the market's responses before it is
/// marked gone: the market lists a newly placed order only some time after accepting it.
const SETTLING: Instant = 120;

/// Stores the market's full list of own orders and marks gone every open order not seen in it
/// for more than `SETTLING` seconds.
pub fn keep_all(
    connection: &mut Connection,
    mine: &[Answered],
    seen: Instant,
) -> Result<(), StoreError> {
    let tx = connection.transaction()?;
    {
        let mut put = tx.prepare(PUT)?;
        for order in mine {
            bind(&tx, &mut put, order, seen)?;
        }
    }
    tx.execute(
        "INSERT INTO order_step (own, at, platinum, quantity, step)
         SELECT id, ?1, platinum, quantity, ?3 FROM own_order
          WHERE gone IS NULL AND seen < ?2",
        params![seen, seen - SETTLING, Step::Gone.as_str()],
    )?;
    tx.execute(
        "UPDATE own_order SET gone = ?1 WHERE gone IS NULL AND seen < ?2",
        params![seen, seen - SETTLING],
    )?;
    tx.commit()?;
    Ok(())
}

/// Stores one order the agent placed, marking it as the agent's, with its expected value.
pub fn keep_placed(
    connection: &mut Connection,
    order: &Answered,
    promised: Option<u32>,
    seen: Instant,
) -> Result<(), StoreError> {
    let tx = connection.transaction()?;
    {
        let mut put = tx.prepare(PUT)?;
        bind(&tx, &mut put, order, seen)?;
    }
    tx.execute(
        "UPDATE own_order SET by_agent = 1, promised = ?2 WHERE id = ?1",
        params![order.0.id, promised],
    )?;
    tx.commit()?;
    Ok(())
}

/// Stores one order without marking any other order gone.
pub fn keep_one(
    connection: &mut Connection,
    order: &Answered,
    seen: Instant,
) -> Result<(), StoreError> {
    let tx = connection.transaction()?;
    {
        let mut put = tx.prepare(PUT)?;
        bind(&tx, &mut put, order, seen)?;
    }
    tx.commit()?;
    Ok(())
}

fn bind(
    connection: &Connection,
    put: &mut rusqlite::Statement<'_>,
    answered: &Answered,
    seen: Instant,
) -> Result<(), StoreError> {
    note(connection, answered, seen)?;
    let (order, slug, variant) = answered;
    put.execute(params![
        order.id,
        order.item_id.clone().unwrap_or_default(),
        slug,
        variant,
        order.side.as_str(),
        order.platinum,
        order.quantity,
        order.per_trade.unwrap_or(1),
        order.visible,
        order.created_at,
        order.updated_at,
        seen,
    ])?;
    Ok(())
}

/// Records the quantity left on an order; zero marks it gone.
pub fn shrink(
    connection: &mut Connection,
    id: &str,
    left: u32,
    at: Instant,
) -> Result<(), StoreError> {
    let tx = connection.transaction()?;
    match left {
        0 => {
            note_gone(&tx, id, at)?;
            tx.execute(
                "UPDATE own_order SET quantity = 0, gone = ?1 WHERE id = ?2 AND gone IS NULL",
                params![at, id],
            )?
        }
        left => {
            tx.execute(
                "INSERT INTO order_step (own, at, platinum, quantity, step)
                 SELECT id, ?2, platinum, ?3, ?4 FROM own_order
                  WHERE id = ?1 AND gone IS NULL AND quantity <> ?3",
                params![id, at, left, Step::Moved.as_str()],
            )?;
            tx.execute(
                "UPDATE own_order SET quantity = ?1 WHERE id = ?2",
                params![left, id],
            )?
        }
    };
    tx.commit()?;
    Ok(())
}

/// Marks the given orders gone; returns the number of orders updated.
pub fn drop_them(
    connection: &mut Connection,
    ids: &[String],
    at: Instant,
) -> Result<usize, StoreError> {
    let tx = connection.transaction()?;
    let mut dropped = 0;
    {
        let mut mark =
            tx.prepare("UPDATE own_order SET gone = ?1 WHERE id = ?2 AND gone IS NULL")?;
        for id in ids {
            note_gone(&tx, id, at)?;
            dropped += mark.execute(params![at, id])?;
        }
    }
    tx.commit()?;
    Ok(dropped)
}

/// Assigns an open order to the agent; returns the number of rows updated.
pub fn hand(connection: &Connection, id: &str) -> Result<usize, StoreError> {
    Ok(connection.execute(
        "UPDATE own_order SET by_agent = 1 WHERE id = ?1 AND gone IS NULL",
        params![id],
    )?)
}

/// Every open order, highest price first.
pub fn standing(connection: &Connection) -> Result<Vec<Standing>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT id, item_id, slug, variant, side, platinum, quantity, per_trade, visible,
                by_agent, promised, created_at, updated_at
           FROM own_order WHERE gone IS NULL ORDER BY platinum DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(Standing {
            id: row.get(0)?,
            item_id: row.get(1)?,
            slug: row.get(2)?,
            variant: row.get(3)?,
            selling: row.get::<_, String>(4)? == "sell",
            platinum: row.get(5)?,
            quantity: row.get(6)?,
            per_trade: row.get(7)?,
            visible: row.get(8)?,
            by_agent: row.get(9)?,
            promised: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(StoreError::from)
}
