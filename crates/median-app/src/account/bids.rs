use median_core::{BidLife, Instant};
use rusqlite::{Connection, params};

use crate::account::store::StoreError;

/// One buy order the agent placed, with the slug and variant of its pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lived {
    pub slug: String,
    pub variant: String,
    pub life: BidLife,
}

/// Every buy order the agent placed that was still open at `from` or later, with its price
/// changes and fills.
pub fn lives(connection: &Connection, from: Instant) -> Result<Vec<Lived>, StoreError> {
    let mut orders = connection.prepare(
        "SELECT id, slug, variant, platinum, created_at, gone FROM own_order
          WHERE by_agent = 1 AND side = 'buy' AND slug IS NOT NULL
            AND (gone IS NULL OR gone >= ?1)",
    )?;
    let mut steps = connection.prepare_cached(
        "SELECT at, platinum FROM order_step
          WHERE own = ?1 AND step <> 'gone' ORDER BY at",
    )?;
    let mut fills = connection
        .prepare_cached("SELECT at FROM exchange WHERE own = ?1 AND side = 'buy' ORDER BY at")?;

    let rows = orders.query_map(params![from], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, u32>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<Instant>>(5)?,
        ))
    })?;

    let mut lived = Vec::new();
    for row in rows {
        let (id, slug, variant, platinum, created_at, gone) = row?;
        let stepped = steps
            .query_map(params![id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<(Instant, u32)>>>()?;
        let prices = match changes(&stepped) {
            prices if !prices.is_empty() => prices,
            _ => match median_market::moment(&created_at) {
                Some(created) => vec![(created, platinum)],
                None => continue,
            },
        };
        let filled = fills
            .query_map(params![id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<Instant>>>()?;
        lived.push(Lived {
            slug,
            variant,
            life: BidLife {
                prices,
                ended: gone,
                fills: filled,
            },
        });
    }
    Ok(lived)
}

/// Steps at which the price changed; steps that only changed the quantity are dropped.
fn changes(steps: &[(Instant, u32)]) -> Vec<(Instant, u32)> {
    let mut prices: Vec<(Instant, u32)> = Vec::new();
    for &(at, platinum) in steps {
        if prices.last().is_none_or(|last| last.1 != platinum) {
            prices.push((at, platinum));
        }
    }
    prices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_that_only_changed_the_size_is_not_a_new_price() {
        assert_eq!(
            changes(&[(0, 11), (60, 11), (120, 14), (180, 14)]),
            [(0, 11), (120, 14)]
        );
    }
}
