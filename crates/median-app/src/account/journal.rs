use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::warn;

/// Maximum number of parsed inventory dumps kept on disk.
const KEPT: usize = 16;

/// Maximum number of unparseable inventory dumps kept on disk.
const UNREAD: usize = 8;

/// Inventory dumps received from the collector, kept on disk only.
pub struct Journal {
    kept: PathBuf,
    unread: PathBuf,
}

impl Journal {
    /// Opens the journal under `root`, creating its directories when missing.
    pub fn under(root: &Path) -> Self {
        let journal = Self {
            kept: root.join("dumps"),
            unread: root.join("dumps-unread"),
        };
        for directory in [&journal.kept, &journal.unread] {
            if let Err(e) = fs::create_dir_all(directory) {
                warn!(path = %directory.display(), error = %e, "dump journal open failed");
            }
        }
        journal
    }

    /// Saves a parsed dump, deleting the oldest ones beyond `KEPT`.
    pub fn keep(&self, dump: &str) {
        self.write(&self.kept, dump, KEPT);
    }

    /// Saves an unparseable dump, deleting the oldest ones beyond `UNREAD`; returns its path,
    /// `None` when the write fails.
    pub fn capture(&self, dump: &str) -> Option<PathBuf> {
        self.write(&self.unread, dump, UNREAD)
    }

    fn write(&self, directory: &Path, dump: &str, room: usize) -> Option<PathBuf> {
        let path = directory.join(format!("{}.json", stamp()));
        if let Err(e) = fs::write(&path, dump) {
            warn!(path = %path.display(), error = %e, "dump journal write failed");
            return None;
        }
        prune(directory, room);
        Some(path)
    }
}

/// Deletes the oldest files until at most `room` are left.
fn prune(directory: &Path, room: usize) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut names: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    if names.len() <= room {
        return;
    }
    names.sort();
    for path in &names[..names.len() - room] {
        if let Err(e) = fs::remove_file(path) {
            warn!(path = %path.display(), error = %e, "dump journal cleanup failed");
        }
    }
}

/// File name that sorts by creation time. The millisecond clock can return the same value for
/// several calls, so a counter breaks ties.
fn stamp() -> String {
    static TAKEN: AtomicU64 = AtomicU64::new(0);
    let millis = median_core::millis();
    let taken = TAKEN.fetch_add(1, Ordering::Relaxed);
    format!("{millis:015}-{taken:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(directory: &Path) -> usize {
        fs::read_dir(directory)
            .map(|entries| entries.count())
            .unwrap_or(0)
    }

    #[test]
    fn the_journal_holds_no_more_than_it_promised() {
        let root = std::env::temp_dir().join(format!("median-journal-{}", stamp()));
        let journal = Journal::under(&root);

        for index in 0..KEPT + 5 {
            journal.keep(&format!("{{\"n\":{index}}}"));
        }

        assert_eq!(count(&journal.kept), KEPT);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_dump_that_did_not_read_is_written_where_it_can_be_found() {
        let root = std::env::temp_dir().join(format!("median-unread-{}", stamp()));
        let journal = Journal::under(&root);

        let path = journal
            .capture("{\"broken\":true}")
            .expect("a captured dump");

        assert_eq!(
            fs::read_to_string(&path).expect("the dump"),
            "{\"broken\":true}"
        );
        fs::remove_dir_all(&root).ok();
    }
}
