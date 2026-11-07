use median_core::Instant;
use rusqlite::{Connection, params};

use crate::account::store::StoreError;

/// Opens an in-game interval starting at `at`.
pub fn began(connection: &Connection, at: Instant) -> Result<(), StoreError> {
    connection.execute(
        "INSERT OR IGNORE INTO presence (began, ended) VALUES (?1, NULL)",
        params![at],
    )?;
    Ok(())
}

/// Closes the open in-game interval at `at`, never before its start.
pub fn ended(connection: &Connection, at: Instant) -> Result<(), StoreError> {
    connection.execute(
        "UPDATE presence SET ended = MAX(began, ?1) WHERE ended IS NULL",
        params![at],
    )?;
    Ok(())
}

/// Closes an in-game interval a previous run left open, at the last event recorded in it: an
/// exchange, an own-order step or an account snapshot. Without any, it ends at its start.
pub fn settle(connection: &Connection) -> Result<(), StoreError> {
    connection.execute(
        "UPDATE presence SET ended = MAX(
             began,
             COALESCE((SELECT MAX(at) FROM exchange WHERE at >= presence.began), began),
             COALESCE((SELECT MAX(at) FROM order_step WHERE at >= presence.began), began),
             COALESCE((SELECT MAX(taken) FROM snapshot WHERE taken >= presence.began), began))
          WHERE ended IS NULL",
        [],
    )?;
    Ok(())
}

/// In-game intervals `(began, ended)` that end at `from` or later, oldest first; the open
/// interval ends at `now`.
pub fn since(
    connection: &Connection,
    from: Instant,
    now: Instant,
) -> Result<Vec<(Instant, Instant)>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT began, COALESCE(ended, ?2) FROM presence
          WHERE COALESCE(ended, ?2) >= ?1 ORDER BY began",
    )?;
    let rows = statement
        .query_map(params![from, now], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history() -> Connection {
        crate::account::laid()
    }

    #[test]
    fn a_stretch_runs_from_going_in_game_to_leaving_it() {
        let connection = history();
        began(&connection, 100).expect("began");
        ended(&connection, 400).expect("ended");
        began(&connection, 1_000).expect("began again");

        assert_eq!(
            since(&connection, 0, 1_600).expect("the stretches"),
            [(100, 400), (1_000, 1_600)],
            "the open one runs to now"
        );
        assert_eq!(
            since(&connection, 500, 1_600).expect("the stretches"),
            [(1_000, 1_600)]
        );
    }

    #[test]
    fn a_stretch_a_run_left_open_ends_at_the_last_thing_seen_in_it() {
        let connection = history();
        began(&connection, 100).expect("began");
        connection
            .execute(
                "INSERT INTO order_step (own, at, platinum, quantity, step)
                 VALUES ('x', 250, 40, 1, 'placed'), ('y', 90, 40, 1, 'placed')",
                [],
            )
            .expect("steps");

        settle(&connection).expect("settled");

        assert_eq!(
            since(&connection, 0, 9_999).expect("the stretches"),
            [(100, 250)]
        );
    }

    #[test]
    fn a_stretch_with_nothing_seen_in_it_lasted_nothing() {
        let connection = history();
        began(&connection, 100).expect("began");

        settle(&connection).expect("settled");

        assert_eq!(
            since(&connection, 0, 9_999).expect("the stretches"),
            [(100, 100)]
        );
    }
}
