use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use median_core::{AccountEvent, Event};
use median_engine::{Input, Reading};
use tokio::sync::mpsc::Sender;
use tracing::{info, warn};

use crate::account::journal::Journal;

/// Parses the inventory dumps the collector sends and passes the results to the engine as
/// account events.
pub struct Reader {
    inputs: Sender<Input>,
    journal: Arc<Journal>,
    /// Top-level keys seen so far, so that a key new in the game's dumps is logged.
    keys: Mutex<HashSet<String>>,
    /// Hash of the last dump received, so that an identical dump is skipped.
    last: Mutex<Seen>,
}

/// Identity of a dump that can be compared without parsing it.
#[derive(Default)]
struct Seen {
    said: Option<blake3::Hash>,
}

impl Reader {
    pub fn new(inputs: Sender<Input>, journal: Arc<Journal>) -> Self {
        Self {
            inputs,
            journal,
            keys: Mutex::new(HashSet::new()),
            last: Mutex::new(Seen::default()),
        }
    }

    /// Parses one collector payload on a blocking thread and sends the resulting account event to
    /// the engine; payloads that are not inventory dumps or repeat the last dump are ignored.
    pub async fn take(&self, payload: &str) {
        // An inventory dump contains the `"inventory"` key; other collector messages do not.
        if !payload.contains(r#""inventory""#) {
            return;
        }
        if self.repeats(payload) {
            return;
        }

        let dump = payload.to_string();
        let journal = Arc::clone(&self.journal);

        let outcome = match tokio::task::spawn_blocking(move || read(&dump, &journal)).await {
            Ok(Some(outcome)) => outcome,
            Ok(None) => return,
            Err(e) => {
                warn!(error = %e, "inventory dump read aborted");
                return;
            }
        };

        let event = match outcome {
            Outcome::Read { event, keys } => {
                self.note(&keys);
                event
            }
            Outcome::Unreadable => AccountEvent::Unreadable,
        };

        self.say(event).await;
    }

    /// Sends one account event to the engine.
    async fn say(&self, event: AccountEvent) {
        let _ = self
            .inputs
            .send(Input::Command(Event::Account(event)))
            .await;
    }

    /// Whether this dump is identical to the last one; otherwise records its hash.
    ///
    /// Compared by content, not by the sync marker: the marker changes with the game's own
    /// bookkeeping, while platinum, credits and trades left change without it.
    fn repeats(&self, payload: &str) -> bool {
        let said = blake3::hash(payload.as_bytes());

        let mut last = self.last.lock().expect("last dump lock poisoned");
        if last.said == Some(said) {
            return true;
        }
        last.said = Some(said);
        false
    }

    /// Logs the top-level keys of this dump that no earlier dump had; the first dump only fills
    /// the set.
    fn note(&self, keys: &[String]) {
        let mut seen = self.keys.lock().expect("seen keys lock poisoned");
        let first = seen.is_empty();
        let fresh: Vec<&str> = keys
            .iter()
            .filter(|key| !seen.contains(key.as_str()))
            .map(String::as_str)
            .collect();

        if !first && !fresh.is_empty() {
            info!(keys = fresh.join(", "), "inventory dump has new keys");
        }
        seen.extend(keys.iter().cloned());
    }
}

/// Result of parsing one dump, with the top-level keys the engine does not use.
enum Outcome {
    Read {
        event: AccountEvent,
        keys: Vec<String>,
    },
    Unreadable,
}

/// Parses a dump and saves it to the journal, parsed or not. Blocking; `None` when the dump
/// yields no reading.
fn read(dump: &str, journal: &Journal) -> Option<Outcome> {
    let started = Instant::now();

    match median_engine::read(dump) {
        Ok(None) => None,
        Ok(Some(Reading { inventory, keys })) => {
            let digest = inventory.digest();
            let positions = inventory.holdings.len() as u32;
            let identified = inventory.profile.is_some();

            info!(
                took_us = started.elapsed().as_micros() as u64,
                bytes = dump.len(),
                holdings = positions,
                digest = digest.short(),
                synced = inventory.synced.as_deref().unwrap_or("none"),
                has_profile = identified,
                "inventory dump read"
            );
            journal.keep(dump);

            let event = match identified {
                true => AccountEvent::Read(Arc::new(inventory)),
                false => AccountEvent::Unidentified,
            };
            Some(Outcome::Read { event, keys })
        }
        Err(error) => {
            let held = journal.capture(dump);
            warn!(bytes = dump.len(), kept_at = held.map(|path| path.display().to_string()).unwrap_or_default(), error = %error, "inventory dump parse failed");
            Some(Outcome::Unreadable)
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::{self, Receiver};

    use super::*;

    /// Dump with one item and the given `LastInventorySync` marker.
    fn dump(marker: &str, rubedo: u32) -> String {
        let document = format!(
            r#"{{"Created":{{"$date":{{"$numberLong":"1700000000000"}}}},
                 "LastInventorySync":{{"$oid":"{marker}"}},
                 "MiscItems":[{{"ItemType":"Rubedo","ItemCount":{rubedo}}}]}}"#
        );
        let quoted = serde_json::to_string(&document).expect("a string");
        format!(
            r#"{{"source":"poll","info":{{"res":{{"game_info":{{"username":"Tenno"}},"match_info":{{"inventory":{quoted}}}}}}}}}"#
        )
    }

    fn reader() -> (Reader, Receiver<Input>) {
        let (tx, rx) = mpsc::channel(8);
        let journal = Arc::new(Journal::under(
            &std::env::temp_dir().join("median-reader-tests"),
        ));
        (Reader::new(tx, journal), rx)
    }

    #[tokio::test]
    async fn a_dump_saying_exactly_what_the_last_one_said_is_not_read_again() {
        let (reader, mut rx) = reader();

        reader.take(&dump("6a7306dd0785235c0a0ddea1", 1)).await;
        reader.take(&dump("6a7306dd0785235c0a0ddea1", 1)).await;

        assert!(rx.recv().await.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn a_dump_that_moved_under_the_same_marker_is_read() {
        let (reader, mut rx) = reader();

        reader.take(&dump("6a7306dd0785235c0a0ddea1", 1)).await;
        reader.take(&dump("6a7306dd0785235c0a0ddea1", 2)).await;

        assert!(rx.recv().await.is_some());
        assert!(
            rx.recv().await.is_some(),
            "the marker moves on the game's own schedule"
        );
    }

    #[tokio::test]
    async fn a_dump_with_a_marker_of_its_own_is_read() {
        let (reader, mut rx) = reader();

        reader.take(&dump("6a7306dd0785235c0a0ddea1", 1)).await;
        reader.take(&dump("6a7306dd0785235c0a0ddea2", 2)).await;

        assert!(rx.recv().await.is_some());
        assert!(rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn a_dump_carrying_no_marker_is_read_when_it_says_something_new() {
        let (reader, mut rx) = reader();
        let unmarked = |count: u32| {
            format!(
                r#"{{"source":"poll","info":{{"res":{{"match_info":{{"inventory":"{{\"MiscItems\":[{{\"ItemType\":\"Rubedo\",\"ItemCount\":{count}}}]}}"}}}}}}}}"#
            )
        };

        reader.take(&unmarked(1)).await;
        reader.take(&unmarked(1)).await;
        reader.take(&unmarked(2)).await;

        assert!(rx.recv().await.is_some());
        assert!(rx.recv().await.is_some());
        assert!(rx.try_recv().is_err(), "the middle one repeated the first");
    }
}
