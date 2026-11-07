use std::time::Duration;

use median_core::{CatalogEvent, Event, seconds};
use median_engine::Input;
use median_ipc::CatalogActivity;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::debug;

use crate::state_sink::SharedState;

/// Timing of the checks for a newly published catalog.
#[derive(Debug, Clone, Copy)]
pub struct Schedule {
    /// Publishing period of the data pipeline.
    pub every: Duration,
    /// Delay after each publishing slot, so the check finds a finished build rather than one
    /// still running.
    pub after: Duration,
    /// Delay before the next check when the last check or install failed.
    pub retry: Duration,
}

impl Default for Schedule {
    /// The data pipeline runs on a six-hour cron in UTC, so the slots are 00:00, 06:00,
    /// 12:00 and 18:00; checks run an hour after each, at 01:00, 07:00, 13:00 and 19:00 UTC.
    fn default() -> Self {
        Self {
            every: Duration::from_secs(6 * 60 * 60),
            after: Duration::from_secs(60 * 60),
            retry: Duration::from_secs(90),
        }
    }
}

impl Schedule {
    /// Time from `now` (Unix seconds) until the next check slot; never zero, because a slot
    /// equal to `now` counts as passed.
    fn until_next(&self, now: u64) -> Duration {
        let every = self.every.as_secs().max(1);
        let after = self.after.as_secs() % every;
        let since = (now + every - after) % every;
        Duration::from_secs(every - since)
    }
}

/// Sends a catalog check request at each slot, or after `retry` while the last check or install
/// failed; returns once the engine's input channel is closed.
pub async fn watch(
    inputs: Sender<Input>,
    state: SharedState,
    schedule: Schedule,
) -> Result<(), String> {
    loop {
        let wait = match unreachable(&state) {
            true => schedule.retry,
            false => schedule.until_next(seconds() as u64),
        };
        debug!(wait_s = wait.as_secs(), "catalog check scheduled");
        sleep(wait).await;

        if inputs
            .send(Input::Command(Event::Catalog(CatalogEvent::SyncRequested)))
            .await
            .is_err()
        {
            return Ok(()); // engine stopped
        }
    }
}

/// Whether the last check or install could not be finished.
fn unreachable(state: &SharedState) -> bool {
    matches!(
        state
            .0
            .read()
            .expect("state lock poisoned")
            .catalog
            .activity,
        CatalogActivity::Unreachable { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seconds past the epoch at the given hour and minute of a day, in UTC.
    fn at(hour: u64, minute: u64) -> u64 {
        20_000 * 86_400 + hour * 3600 + minute * 60
    }

    #[test]
    fn a_slot_the_clock_stands_on_counts_as_passed() {
        let schedule = Schedule::default();
        assert_eq!(schedule.until_next(at(1, 0)), Duration::from_secs(6 * 3600));
        assert_eq!(
            schedule.until_next(at(19, 0)),
            Duration::from_secs(6 * 3600)
        );
    }

    #[test]
    fn waits_out_the_rest_of_the_period() {
        let schedule = Schedule::default();
        assert_eq!(schedule.until_next(at(0, 30)), Duration::from_secs(30 * 60));
        assert_eq!(schedule.until_next(at(2, 0)), Duration::from_secs(5 * 3600));
        assert_eq!(schedule.until_next(at(6, 59)), Duration::from_secs(60));
    }

    #[test]
    fn the_slots_are_an_hour_past_the_producers() {
        let schedule = Schedule::default();
        for (hour, minute) in [(23, 30), (3, 0), (12, 45)] {
            let landed = at(hour, minute) + schedule.until_next(at(hour, minute)).as_secs();
            assert_eq!(landed % 86_400 % (6 * 3600), 3600);
        }
    }

    #[test]
    fn an_offset_longer_than_the_period_wraps_into_it() {
        let schedule = Schedule {
            every: Duration::from_secs(6 * 3600),
            after: Duration::from_secs(7 * 3600),
            retry: Duration::from_secs(90),
        };
        assert_eq!(schedule.until_next(at(1, 0)), Duration::from_secs(6 * 3600));
    }
}
