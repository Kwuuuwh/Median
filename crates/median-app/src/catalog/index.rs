use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

/// One picture in a pack: byte offset and length in the archive, and the BLAKE3 hash of the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub offset: u64,
    pub len: u64,
    pub blake3: String,
}

/// Parses a pack index of `id offset len hash` lines; blank lines are skipped and any malformed
/// line fails the whole parse.
pub fn parse(text: &str) -> Result<Vec<Entry>, String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(entry)
        .collect()
}

fn entry(line: &str) -> Result<Entry, String> {
    let mut fields = line.split_whitespace();
    let (Some(id), Some(offset), Some(len), Some(blake3), None) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) else {
        return Err(format!("index line is not `id offset len hash`: {line:?}"));
    };
    Ok(Entry {
        id: id.to_string(),
        offset: number(offset, line)?,
        len: number(len, line)?,
        blake3: blake3.to_string(),
    })
}

fn number(field: &str, line: &str) -> Result<u64, String> {
    field
        .parse()
        .map_err(|_| format!("index line carries an unreadable number {field:?}: {line:?}"))
}

/// Index of the pictures on disk, keyed by picture id, holding the entries of the release each
/// picture came from.
#[derive(Debug, Default)]
pub struct Held(BTreeMap<String, Entry>);

impl Held {
    /// Reads an index from disk; an absent or unparseable file yields an empty index.
    pub fn load(path: &Path) -> Self {
        let held = fs::read_to_string(path)
            .ok()
            .and_then(|text| parse(&text).ok())
            .unwrap_or_default();
        Self(held.into_iter().map(|e| (e.id.clone(), e)).collect())
    }

    /// Writes the index through a temporary file, so a crash leaves the old one intact.
    pub fn store(&self, path: &Path) -> io::Result<()> {
        let mut text = String::with_capacity(self.0.len() * 48);
        for e in self.0.values() {
            text.push_str(&format!("{} {} {} {}\n", e.id, e.offset, e.len, e.blake3));
        }
        let partial = path.with_extension("writing");
        fs::write(&partial, text)?;
        fs::rename(&partial, path)
    }

    /// Whether the picture is recorded as on disk with the same hash.
    pub fn has(&self, entry: &Entry) -> bool {
        self.0
            .get(&entry.id)
            .is_some_and(|held| held.blake3 == entry.blake3)
    }

    pub fn add(&mut self, entry: Entry) {
        self.0.insert(entry.id.clone(), entry);
    }

    /// Records everything in `entries` as held.
    pub fn extend(&mut self, entries: impl IntoIterator<Item = Entry>) {
        for entry in entries {
            self.add(entry);
        }
    }

    /// Removes every entry whose id is not in `present`.
    pub fn retain(&mut self, present: &BTreeSet<String>) {
        self.0.retain(|id, _| present.contains(id));
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = "\
000074f177a0df5f 512 6434 44cd0a2e4cc9b34c
0005948540be781a 7680 10614 db39f59b602225a7
";

    #[test]
    fn reads_every_field() {
        let entries = parse(INDEX).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "000074f177a0df5f");
        assert_eq!(entries[0].offset, 512);
        assert_eq!(entries[0].len, 6434);
        assert_eq!(entries[0].blake3, "44cd0a2e4cc9b34c");
        assert_eq!(entries[1].offset, 7680);
    }

    #[test]
    fn blank_lines_are_skipped() {
        assert_eq!(parse("\n\n").unwrap(), Vec::new());
    }

    #[test]
    fn a_short_line_is_rejected() {
        assert!(parse("000074f177a0df5f 512 6434").is_err());
    }

    #[test]
    fn a_long_line_is_rejected() {
        assert!(parse("000074f177a0df5f 512 6434 44cd0a2e 7").is_err());
    }

    #[test]
    fn an_unreadable_number_is_rejected() {
        assert!(parse("000074f177a0df5f start 6434 44cd0a2e").is_err());
    }

    #[test]
    fn held_matches_on_the_hash_not_the_name() {
        let entries = parse(INDEX).unwrap();
        let mut held = Held::default();
        held.add(entries[0].clone());

        assert!(held.has(&entries[0]));
        assert!(!held.has(&entries[1]));

        let recoded = Entry {
            blake3: "ffffffffffffffff".into(),
            ..entries[0].clone()
        };
        assert!(!held.has(&recoded));
    }

    #[test]
    fn retaining_drops_what_is_no_longer_on_disk() {
        let entries = parse(INDEX).unwrap();
        let mut held = Held::default();
        held.add(entries[0].clone());
        held.add(entries[1].clone());

        held.retain(&BTreeSet::from(["0005948540be781a".to_string()]));

        assert_eq!(held.ids().collect::<Vec<_>>(), vec!["0005948540be781a"]);
    }
}
