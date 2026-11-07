use median_core::{Instant, Swap, SwapKind, SwapPart};
use rusqlite::{Connection, params};

use crate::account::store::StoreError;

const GIVEN: &str = "given";
const TAKEN: &str = "taken";

/// Records one in-game trade with another player, both the items given and taken; returns the
/// id of its `swap` row.
pub fn record(connection: &Connection, swap: &Swap) -> Result<i64, StoreError> {
    connection.execute(
        "INSERT INTO swap (at, partner, outcome, platinum, buying)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            swap.at,
            swap.partner,
            swap.outcome.as_code(),
            swap.platinum_moved(),
            swap.buying(),
        ],
    )?;
    let id = connection.last_insert_rowid();

    let mut put = connection.prepare(
        "INSERT INTO swap_part (swap, side, name, count, kind, rank)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (side, parts) in [(GIVEN, &swap.given), (TAKEN, &swap.taken)] {
        for part in parts {
            put.execute(params![
                id,
                side,
                part.name,
                part.count,
                part.kind.as_code(),
                part.rank
            ])?;
        }
    }
    Ok(id)
}

/// Every in-game trade from `from` on, newest first.
pub fn since(connection: &Connection, from: Instant) -> Result<Vec<Swap>, StoreError> {
    let mut heads = connection.prepare(
        "SELECT id, at, partner, outcome FROM swap WHERE at >= ?1 ORDER BY at DESC, id DESC",
    )?;
    let rows = heads.query_map(params![from], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Instant>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut swaps = Vec::new();
    for row in rows {
        let (id, at, partner, outcome) = row?;
        let (given, taken) = parts(connection, id)?;
        swaps.push(Swap {
            at,
            partner,
            given,
            taken,
            outcome: outcome_of(&outcome),
        });
    }
    Ok(swaps)
}

fn parts(connection: &Connection, swap: i64) -> Result<(Vec<SwapPart>, Vec<SwapPart>), StoreError> {
    let mut statement = connection.prepare(
        "SELECT side, name, count, kind, rank FROM swap_part WHERE swap = ?1 ORDER BY rowid",
    )?;
    let rows = statement.query_map(params![swap], |row| {
        Ok((
            row.get::<_, String>(0)?,
            SwapPart {
                name: row.get(1)?,
                count: row.get(2)?,
                kind: kind_of(&row.get::<_, String>(3)?),
                rank: row.get(4)?,
            },
        ))
    })?;

    let (mut given, mut taken) = (Vec::new(), Vec::new());
    for row in rows {
        let (side, part) = row?;
        match side.as_str() {
            TAKEN => taken.push(part),
            _ => given.push(part),
        }
    }
    Ok((given, taken))
}

fn kind_of(code: &str) -> SwapKind {
    match code {
        "platinum" => SwapKind::Platinum,
        "credits" => SwapKind::Credits,
        _ => SwapKind::Item,
    }
}

fn outcome_of(code: &str) -> median_core::Outcome {
    match code {
        "failed" => median_core::Outcome::Failed,
        "cancelled" => median_core::Outcome::Cancelled,
        _ => median_core::Outcome::Done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use median_core::Outcome;

    fn table() -> Connection {
        crate::account::laid()
    }

    fn part(name: &str, count: u32, kind: SwapKind) -> SwapPart {
        SwapPart {
            name: name.into(),
            count,
            kind,
            rank: None,
        }
    }

    fn swap() -> Swap {
        Swap {
            at: 1_787_867_459,
            partner: "kafasi".into(),
            given: vec![part("Платина", 108, SwapKind::Platinum)],
            taken: vec![part("Мистическая зарядка", 1, SwapKind::Item)],
            outcome: Outcome::Done,
        }
    }

    #[test]
    fn an_exchange_reads_back_with_both_its_sides() {
        let connection = table();
        record(&connection, &swap()).expect("it lands");

        let read = since(&connection, 0).expect("it reads");

        assert_eq!(read.len(), 1);
        assert_eq!(read[0], swap());
        assert!(read[0].buying());
    }

    #[test]
    fn several_lines_on_one_side_all_land_in_order() {
        let connection = table();
        let mut many = swap();
        many.taken = vec![
            part("Лит G12 Реликвия", 3, SwapKind::Item),
            part("Мезо F3 Реликвия", 1, SwapKind::Item),
        ];
        record(&connection, &many).expect("it lands");

        let read = since(&connection, 0).expect("it reads");

        assert_eq!(read[0].taken, many.taken);
    }

    #[test]
    fn the_rank_the_game_named_comes_back_with_the_copy() {
        let connection = table();
        let mut ranked = swap();
        ranked.taken = vec![SwapPart {
            name: "Гальванизированный Прицел".into(),
            count: 1,
            kind: SwapKind::Item,
            rank: Some(10),
        }];
        record(&connection, &ranked).expect("it lands");

        let read = since(&connection, 0).expect("it reads");

        assert_eq!(read[0].taken, ranked.taken);
    }

    #[test]
    fn an_exchange_before_the_window_is_not_read() {
        let connection = table();
        record(&connection, &swap()).expect("it lands");

        assert!(
            since(&connection, 1_787_867_460)
                .expect("it reads")
                .is_empty()
        );
    }
}
