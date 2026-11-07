use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};

/// File of keys the app looked up that have no catalog row, collected for the data project. A
/// missing row is a data gap, not a broken input, so the player's items are still shown.
pub struct Gaps {
    path: PathBuf,
    seen: Mutex<HashSet<String>>,
}

impl Gaps {
    /// Opens the file at `path`, loading lines written by earlier runs so they are not repeated.
    pub fn new(path: PathBuf) -> Self {
        let seen = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();

        Self {
            path,
            seen: Mutex::new(seen),
        }
    }

    /// Appends `<kind> <key>` for each key of `wanted` missing from `found`; a line already in
    /// the file is not written again.
    pub fn note<'a>(&self, kind: &str, wanted: &[String], found: impl Iterator<Item = &'a str>) {
        let found: HashSet<&str> = found.collect();
        self.append(
            wanted
                .iter()
                .filter(|key| !found.contains(key.as_str()))
                .map(|key| format!("{kind} {key}")),
        );
    }

    /// Appends `<kind> <key> <name>` for keys that are in the catalog but lack the data `kind`
    /// names (e.g. `way`: no known source); a line already in the file is not written again.
    pub fn note_named<'a>(&self, kind: &str, named: impl Iterator<Item = (&'a str, &'a str)>) {
        self.append(named.map(|(key, name)| format!("{kind} {key} {name}")));
    }

    fn append(&self, lines: impl Iterator<Item = String>) {
        let mut seen = self.seen.lock().expect("gaps lock poisoned");
        let fresh: Vec<String> = lines.filter(|line| seen.insert(line.clone())).collect();
        if fresh.is_empty() {
            return;
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            for line in fresh {
                let _ = writeln!(file, "{line}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Gaps;

    #[test]
    fn only_what_is_missing_is_written_and_only_once() {
        let file = std::env::temp_dir().join(format!("median-gaps-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&file);

        let wanted = ["known".to_string(), "lost".to_string()];
        let gaps = Gaps::new(file.clone());
        gaps.note("item", &wanted, ["known"].into_iter());
        gaps.note("item", &wanted, ["known"].into_iter());

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "item lost\n");

        // A later run loads the file and does not repeat lines already in it.
        Gaps::new(file.clone()).note("item", &wanted, ["known"].into_iter());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "item lost\n");

        let _ = std::fs::remove_file(&file);
    }
}
