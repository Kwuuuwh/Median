use std::{fs::OpenOptions, io::Write, path::PathBuf};

use median_engine::{QuarantineRecord, QuarantineSink};

/// Quarantine sink that appends each rejected input as one line to a file for review; write
/// errors are ignored.
pub struct FileQuarantine {
    path: PathBuf,
}

impl FileQuarantine {
    /// Creates a quarantine that appends to `path`, creating the file on the first record.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl QuarantineSink for FileQuarantine {
    fn record(&self, record: QuarantineRecord) {
        let line = format!("[{}] {} | {}\n", record.source, record.error, record.shape);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }
}
