use median_core::Instant;
use rusqlite::{Connection, OptionalExtension, params};

use crate::account::exchanges::Made;
use crate::account::store::StoreError;

/// Copies of an item bought for resale, with their total purchase cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// Game unique name of the item.
    pub item: String,
    pub slug: String,
    pub variant: String,
    pub quantity: u32,
    /// Total purchase price in platinum of the copies still held.
    pub spent: u32,
    /// When the position was first opened.
    pub opened: Instant,
    /// Number of ranked-up copies of the item in the inventory when the position was first
    /// checked, so copies fused since then can be told apart from those already held.
    pub ranked_before: Option<u32>,
}

impl Position {
    /// Purchase cost of one copy in platinum, rounded up so that a price floor based on it is
    /// never below the real cost; 0 for an empty position.
    pub fn basis(&self) -> u32 {
        match self.quantity {
            0 => 0,
            quantity => self.spent.div_ceil(quantity),
        }
    }
}

/// Updates the position on the exchange's pair: a buy opens or adds to it, a sale removes
/// copies at their average cost, and a position with no copies left is deleted.
pub fn settle(connection: &Connection, made: &Made, at: Instant) -> Result<(), StoreError> {
    let held = of(connection, &made.slug, &made.variant)?;
    let moved = made.platinum.saturating_mul(made.quantity);

    match (held, made.selling) {
        (None, false) => {
            connection.execute(
                "INSERT INTO position (slug, variant, item, quantity, spent, opened)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![made.slug, made.variant, made.item, made.quantity, moved, at],
            )?;
        }
        (Some(held), false) => {
            connection.execute(
                "UPDATE position SET quantity = ?3, spent = ?4 WHERE slug = ?1 AND variant = ?2",
                params![
                    made.slug,
                    made.variant,
                    held.quantity.saturating_add(made.quantity),
                    held.spent.saturating_add(moved),
                ],
            )?;
        }
        (Some(held), true) => {
            let gone = made.quantity.min(held.quantity);
            let left = held.quantity - gone;
            match left {
                0 => drop_it(connection, &made.slug, &made.variant)?,
                left => {
                    connection.execute(
                        "UPDATE position SET quantity = ?3, spent = ?4
                          WHERE slug = ?1 AND variant = ?2",
                        params![
                            made.slug,
                            made.variant,
                            left,
                            held.spent.saturating_sub(held.basis().saturating_mul(gone)),
                        ],
                    )?;
                }
            }
        }
        (None, true) => {}
    }
    Ok(())
}

/// Copies removed from positions to make another item, and the item they make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversion {
    /// Consumed pairs as `(slug, variant, copies)`.
    pub from: Vec<(String, String, u32)>,
    pub into_slug: String,
    pub into_variant: String,
    pub into_item: String,
    pub into_quantity: u32,
}

/// Moves copies from the consumed positions into the position of the result, carrying over
/// their cost; e.g. parts combined into a set, or a copy ranked up.
pub fn convert(
    connection: &Connection,
    conversion: &Conversion,
    at: Instant,
) -> Result<(), StoreError> {
    let mut cost: u32 = 0;
    for (slug, variant, copies) in &conversion.from {
        let Some(held) = of(connection, slug, variant)? else {
            continue;
        };
        let gone = (*copies).min(held.quantity);
        let spent = held.basis().saturating_mul(gone).min(held.spent);
        cost = cost.saturating_add(spent);
        match held.quantity - gone {
            0 => drop_it(connection, slug, variant)?,
            left => {
                connection.execute(
                    "UPDATE position SET quantity = ?3, spent = ?4
                      WHERE slug = ?1 AND variant = ?2",
                    params![slug, variant, left, held.spent - spent],
                )?;
            }
        }
    }
    let made = Made {
        item: conversion.into_item.clone(),
        slug: conversion.into_slug.clone(),
        variant: conversion.into_variant.clone(),
        selling: false,
        platinum: cost.div_ceil(conversion.into_quantity.max(1)),
        quantity: conversion.into_quantity,
        origin: String::new(),
        own: None,
    };
    settle(connection, &made, at)
}

/// Records on the position how many ranked-up copies of the item the account holds.
pub fn mark_ranked(
    connection: &Connection,
    slug: &str,
    variant: &str,
    ranked: u32,
) -> Result<(), StoreError> {
    connection.execute(
        "UPDATE position SET ranked_before = ?3 WHERE slug = ?1 AND variant = ?2",
        params![slug, variant, ranked],
    )?;
    Ok(())
}

/// Position on one pair; `None` when there is none.
pub fn of(
    connection: &Connection,
    slug: &str,
    variant: &str,
) -> Result<Option<Position>, StoreError> {
    Ok(connection
        .query_row(
            "SELECT item, slug, variant, quantity, spent, opened, ranked_before FROM position
              WHERE slug = ?1 AND variant = ?2",
            params![slug, variant],
            row,
        )
        .optional()?)
}

/// Every open position, oldest first.
pub fn open(connection: &Connection) -> Result<Vec<Position>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT item, slug, variant, quantity, spent, opened, ranked_before FROM position
          ORDER BY opened",
    )?;
    let rows = statement.query_map([], row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<Position>>>()?)
}

/// Removes a position from the agent's control: deletes it and returns the agent's open sell
/// order on the same pair to the user.
pub fn release(connection: &Connection, slug: &str, variant: &str) -> Result<(), StoreError> {
    drop_it(connection, slug, variant)?;
    connection.execute(
        "UPDATE own_order SET by_agent = 0
          WHERE by_agent = 1 AND side = 'sell' AND gone IS NULL AND slug = ?1 AND variant = ?2",
        params![slug, variant],
    )?;
    Ok(())
}

fn drop_it(connection: &Connection, slug: &str, variant: &str) -> Result<(), StoreError> {
    connection.execute(
        "DELETE FROM position WHERE slug = ?1 AND variant = ?2",
        params![slug, variant],
    )?;
    Ok(())
}

fn row(row: &rusqlite::Row) -> rusqlite::Result<Position> {
    Ok(Position {
        item: row.get(0)?,
        slug: row.get(1)?,
        variant: row.get(2)?,
        quantity: row.get(3)?,
        spent: row.get(4)?,
        opened: row.get(5)?,
        ranked_before: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Connection {
        let connection = Connection::open_in_memory().expect("memory opens");
        connection
            .execute_batch(
                "CREATE TABLE position (
                   slug     TEXT    NOT NULL,
                   variant  TEXT    NOT NULL,
                   item     TEXT    NOT NULL,
                   quantity INTEGER NOT NULL,
                   spent    INTEGER NOT NULL,
                   opened   INTEGER NOT NULL,
                   ranked_before INTEGER,
                   PRIMARY KEY (slug, variant)
                 ) WITHOUT ROWID;",
            )
            .expect("the table lands");
        connection
    }

    fn made(selling: bool, platinum: u32, quantity: u32) -> Made {
        Made {
            item: "/Lotus/Powersuits/Hoplite/StyanaxPrime".into(),
            slug: "styanax_prime_set".into(),
            variant: String::new(),
            selling,
            platinum,
            quantity,
            origin: "hand".into(),
            own: None,
        }
    }

    #[test]
    fn a_buy_opens_a_position_at_what_it_cost() {
        let connection = table();
        settle(&connection, &made(false, 30, 1), 100).expect("settles");

        let held = of(&connection, "styanax_prime_set", "")
            .expect("reads")
            .expect("there is a position");
        assert_eq!(held.quantity, 1);
        assert_eq!(held.spent, 30);
        assert_eq!(held.basis(), 30);
        assert_eq!(held.opened, 100);
    }

    #[test]
    fn buying_again_averages_what_the_copies_cost() {
        let connection = table();
        settle(&connection, &made(false, 30, 1), 100).expect("settles");
        settle(&connection, &made(false, 40, 3), 200).expect("settles");

        let held = of(&connection, "styanax_prime_set", "")
            .expect("reads")
            .expect("there is a position");
        assert_eq!(held.quantity, 4);
        assert_eq!(held.spent, 150);
        assert_eq!(held.basis(), 38);
        assert_eq!(held.opened, 100, "the position keeps the day it opened");
    }

    #[test]
    fn a_sale_takes_copies_out_and_the_last_one_closes_it() {
        let connection = table();
        settle(&connection, &made(false, 30, 2), 100).expect("settles");
        settle(&connection, &made(true, 45, 1), 300).expect("settles");

        let held = of(&connection, "styanax_prime_set", "")
            .expect("reads")
            .expect("one copy is left");
        assert_eq!(held.quantity, 1);
        assert_eq!(held.spent, 30);

        settle(&connection, &made(true, 45, 1), 400).expect("settles");
        assert!(
            of(&connection, "styanax_prime_set", "")
                .expect("reads")
                .is_none()
        );
    }

    #[test]
    fn selling_more_than_is_held_closes_it_rather_than_going_negative() {
        let connection = table();
        settle(&connection, &made(false, 30, 1), 100).expect("settles");
        settle(&connection, &made(true, 45, 6), 300).expect("settles");

        assert!(
            of(&connection, "styanax_prime_set", "")
                .expect("reads")
                .is_none()
        );
    }

    #[test]
    fn a_sale_of_something_never_bought_opens_nothing() {
        let connection = table();
        settle(&connection, &made(true, 45, 1), 100).expect("settles");

        assert!(open(&connection).expect("reads").is_empty());
    }

    /// Position table plus the `own_order` rows that a release returns to the user.
    fn with_orders() -> Connection {
        let connection = table();
        connection
            .execute_batch(
                "CREATE TABLE own_order (
                   id       TEXT PRIMARY KEY,
                   slug     TEXT,
                   variant  TEXT NOT NULL,
                   side     TEXT NOT NULL,
                   by_agent INTEGER NOT NULL DEFAULT 0,
                   gone     INTEGER
                 );
                 INSERT INTO own_order (id, slug, variant, side, by_agent) VALUES
                   ('ours', 'styanax_prime_set', '', 'sell', 1),
                   ('bid', 'styanax_prime_set', '', 'buy', 1),
                   ('other', 'styanax_prime_set', 'mod_rank=0', 'sell', 1);",
            )
            .expect("the orders land");
        connection
    }

    fn by_agent(connection: &Connection, id: &str) -> bool {
        connection
            .query_row(
                "SELECT by_agent FROM own_order WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("the order reads")
    }

    #[test]
    fn a_released_position_is_gone_and_its_offer_is_the_persons_again() {
        let connection = with_orders();
        settle(&connection, &made(false, 30, 2), 100).expect("settles");

        release(&connection, "styanax_prime_set", "").expect("releases");

        assert!(open(&connection).expect("reads").is_empty());
        assert!(!by_agent(&connection, "ours"));
        // The buy order and the sell order on another variant do not belong to the position.
        assert!(by_agent(&connection, "bid"));
        assert!(by_agent(&connection, "other"));
    }

    #[test]
    fn the_basis_rounds_up_so_the_floor_never_sits_under_the_cost() {
        let connection = table();
        settle(&connection, &made(false, 10, 3), 100).expect("settles");

        let held = of(&connection, "styanax_prime_set", "")
            .expect("reads")
            .expect("there is a position");
        assert_eq!(held.spent, 30);

        settle(&connection, &made(true, 12, 1), 200).expect("settles");
        let held = of(&connection, "styanax_prime_set", "")
            .expect("reads")
            .expect("two copies are left");
        assert_eq!(held.quantity, 2);
        assert_eq!(held.spent, 20);
        assert_eq!(held.basis(), 10);
    }

    #[test]
    fn parts_put_together_carry_what_they_cost_into_the_set() {
        let connection = table();
        let part = |slug: &str, platinum: u32, quantity: u32| Made {
            slug: slug.into(),
            item: format!("/Lotus/{slug}"),
            ..made(false, platinum, quantity)
        };
        settle(&connection, &part("blade", 10, 3), 100).expect("settles");
        settle(&connection, &part("handle", 15, 2), 100).expect("settles");

        let conversion = Conversion {
            from: vec![
                ("blade".into(), String::new(), 2),
                ("handle".into(), String::new(), 2),
            ],
            into_slug: "dual_kamas_prime_set".into(),
            into_variant: String::new(),
            into_item: "dual_kamas_prime_set".into(),
            into_quantity: 1,
        };
        convert(&connection, &conversion, 200).expect("converts");

        let set = of(&connection, "dual_kamas_prime_set", "")
            .expect("reads")
            .expect("the set is held");
        assert_eq!((set.quantity, set.spent), (1, 50));
        let blade = of(&connection, "blade", "")
            .expect("reads")
            .expect("one blade left");
        assert_eq!((blade.quantity, blade.spent), (1, 10));
        assert!(of(&connection, "handle", "").expect("reads").is_none());
    }
}
