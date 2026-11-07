use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use median_core::{Release, Version};
use tauri::async_runtime::JoinHandle;
use tracing::info;

use crate::catalog::index::{self, Entry, Held};
use crate::catalog::paths::Layout;
use crate::catalog::source::{self, Source};

/// Largest gap between missing pictures that is downloaded rather than split into a new
/// request: about what a connection transfers during one request's round trip, so reading the
/// gap costs less than a new request.
const MERGE_GAP: u64 = 512 << 10;

/// Maximum bytes per range request, which bounds the memory a sync holds.
const RUN_LIMIT: u64 = 8 << 20;

/// Maximum number of downloaded runs whose pictures are still being written; at this count the
/// download waits for the oldest write to finish.
const WRITES_AHEAD: usize = 3;

/// Interval at which the partial index (pictures already on disk) is saved during a sync.
const STORE_EVERY: Duration = Duration::from_secs(3);

/// Extension every picture in the pack has.
const PICTURE: &str = "webp";

/// Contiguous byte range `[start, end)` of the pack that holds missing pictures and is fetched
/// in one request.
#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    pub start: u64,
    pub end: u64,
    pub entries: Vec<Entry>,
}

/// Groups the missing pictures into byte ranges: entries within `MERGE_GAP` of each other
/// are read in one request, and no request carries more than `RUN_LIMIT`.
pub fn runs(mut missing: Vec<Entry>) -> Vec<Run> {
    if missing.is_empty() {
        return Vec::new();
    }
    missing.sort_by_key(|entry| entry.offset);

    let mut out: Vec<Run> = Vec::new();
    for entry in missing {
        let end = entry.offset + entry.len;
        match out.last_mut() {
            Some(run)
                if entry.offset.saturating_sub(run.end) <= MERGE_GAP
                    && end - run.start <= RUN_LIMIT =>
            {
                run.end = end;
                run.entries.push(entry);
            }
            _ => out.push(Run {
                start: entry.offset,
                end,
                entries: vec![entry],
            }),
        }
    }
    out
}

/// Newest pack version whose pictures are complete on disk; `None` when no pack is complete.
pub fn version(layout: &Layout) -> Option<Version> {
    layout.indexes().first().map(|(version, _)| *version)
}

/// Deletes pictures not listed in the newest complete index, and every older index. Does
/// nothing while a sync is in progress, because the pictures it has written are not yet listed
/// in any complete index.
pub fn prune(layout: &Layout) {
    if layout.partial_index().exists() {
        return;
    }
    let indexes = layout.indexes();
    let Some((_, newest)) = indexes.first() else {
        return;
    };

    let held = Held::load(newest);
    let keep: BTreeSet<&str> = held.ids().collect();
    let dir = layout.pack();
    let mut dropped = 0;
    for id in present(&dir) {
        if !keep.contains(id.as_str()) {
            let _ = fs::remove_file(path(&dir, &id));
            dropped += 1;
        }
    }
    for (_, stale) in indexes.iter().skip(1) {
        let _ = fs::remove_file(stale);
    }
    if dropped > 0 {
        info!(files = dropped, "unused pictures removed");
    }
}

/// Updates the pack to `release`, downloading only pictures that are missing on disk or have a
/// different hash.
pub async fn sync(
    source: &Source,
    layout: &Layout,
    release: &Release,
    mut progress: impl FnMut(u64, u64) + Send,
) -> Result<(), String> {
    let published = index::parse(&source.text(source::INDEX).await?)?;
    let dir = layout.pack();
    let partial = layout.partial_index();

    let mut held = starting_point(layout, &partial);
    held.retain(&present(&dir));

    let missing: Vec<Entry> = published
        .iter()
        .filter(|entry| !held.has(entry))
        .cloned()
        .collect();
    let plan = runs(missing);
    let total: u64 = plan.iter().map(|run| run.end - run.start).sum();
    info!(
        pictures = published.len(),
        requests = plan.len(),
        bytes = total,
        "pictures download planned"
    );
    progress(0, total);

    // Writing a run's pictures takes about as long as downloading the next run, so writes run
    // in parallel with the download instead of between requests.
    let mut done = 0;
    let mut writing: VecDeque<JoinHandle<Result<Vec<Entry>, String>>> = VecDeque::new();
    let mut stored = Instant::now();

    for run in &plan {
        let fetched = fetch(source, &release.pack.file, run, &mut |bytes| {
            done += bytes;
            progress(done, total);
        })
        .await?;

        while writing.len() >= WRITES_AHEAD {
            held.extend(collect(&mut writing).await?);
        }
        writing.push_back(spawn_write(dir.clone(), fetched));

        if stored.elapsed() >= STORE_EVERY {
            held.store(&partial)
                .map_err(|e| format!("write {}: {e}", partial.display()))?;
            stored = Instant::now();
        }
    }

    while !writing.is_empty() {
        held.extend(collect(&mut writing).await?);
    }

    let complete = layout.index(release.version);
    held.store(&complete)
        .map_err(|e| format!("write {}: {e}", complete.display()))?;
    let _ = fs::remove_file(&partial);
    Ok(())
}

/// Index of the pictures already on disk: the partial index of an interrupted sync if present,
/// else the newest complete index, else empty.
fn starting_point(layout: &Layout, partial: &Path) -> Held {
    if partial.exists() {
        return Held::load(partial);
    }
    match layout.indexes().first() {
        Some((_, newest)) => Held::load(newest),
        None => Held::default(),
    }
}

/// Downloads one run and splits it into its pictures; fails when a picture does not match its
/// hash or the range ends before the last picture.
async fn fetch(
    source: &Source,
    file: &str,
    run: &Run,
    on_bytes: &mut (dyn FnMut(u64) + Send),
) -> Result<Vec<(Entry, Vec<u8>)>, String> {
    let mut cursor = run.start;
    let mut at = 0usize;
    let mut buffer: Vec<u8> = Vec::new();
    let mut out: Vec<(Entry, Vec<u8>)> = Vec::with_capacity(run.entries.len());

    source
        .stream(file, Some((run.start, run.end)), |chunk| {
            on_bytes(chunk.len() as u64);
            let mut rest = chunk;
            while !rest.is_empty() && at < run.entries.len() {
                let entry = &run.entries[at];
                if cursor < entry.offset {
                    let skip = ((entry.offset - cursor) as usize).min(rest.len());
                    rest = &rest[skip..];
                    cursor += skip as u64;
                    continue;
                }
                let take = (entry.len as usize - buffer.len()).min(rest.len());
                buffer.extend_from_slice(&rest[..take]);
                rest = &rest[take..];
                cursor += take as u64;

                if buffer.len() as u64 == entry.len {
                    let bytes = std::mem::take(&mut buffer);
                    if digest(&bytes) != entry.blake3 {
                        return Err(format!("picture {} does not match its hash", entry.id));
                    }
                    out.push((entry.clone(), bytes));
                    at += 1;
                }
            }
            Ok(())
        })
        .await?;

    if at != run.entries.len() {
        return Err(format!(
            "range {}-{} ended after {at} of {} pictures",
            run.start,
            run.end,
            run.entries.len()
        ));
    }
    Ok(out)
}

/// Writes a run's pictures on a blocking thread; the handle returns the entries written.
fn spawn_write(
    dir: PathBuf,
    fetched: Vec<(Entry, Vec<u8>)>,
) -> JoinHandle<Result<Vec<Entry>, String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut entries = Vec::with_capacity(fetched.len());
        for (entry, bytes) in fetched {
            let path = path(&dir, &entry.id);
            fs::write(&path, &bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
            entries.push(entry);
        }
        Ok(entries)
    })
}

/// Waits for the oldest pending write and returns the entries it wrote; empty when no write is
/// pending.
async fn collect(
    writing: &mut VecDeque<JoinHandle<Result<Vec<Entry>, String>>>,
) -> Result<Vec<Entry>, String> {
    match writing.pop_front() {
        Some(handle) => handle.await.map_err(|e| e.to_string())?,
        None => Ok(Vec::new()),
    }
}

/// Ids of the pictures in the pack directory; empty when the directory cannot be read.
fn present(dir: &Path) -> BTreeSet<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != PICTURE {
                return None;
            }
            Some(path.file_stem()?.to_str()?.to_string())
        })
        .collect()
}

fn path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.{PICTURE}"))
}

/// Bytes of the picture `id`; `None` when `id` is not 16 hex digits or the file is missing.
pub fn picture(layout: &Layout, id: &str) -> Option<Vec<u8>> {
    if id.len() != 16 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    fs::read(path(&layout.pack(), id)).ok()
}

/// First 16 hex digits of the BLAKE3 hash, the length the index stores.
fn digest(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().as_str()[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(offset: u64, len: u64) -> Entry {
        Entry {
            id: format!("{offset:016x}"),
            offset,
            len,
            blake3: "0".repeat(16),
        }
    }

    #[test]
    fn nothing_missing_asks_for_nothing() {
        assert_eq!(runs(Vec::new()), Vec::new());
    }

    #[test]
    fn neighbours_are_read_in_one_request() {
        let plan = runs(vec![entry(512, 100), entry(1024, 100)]);

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].start, 512);
        assert_eq!(plan[0].end, 1124);
        assert_eq!(plan[0].entries.len(), 2);
    }

    #[test]
    fn a_wide_gap_splits_the_request() {
        let plan = runs(vec![entry(512, 100), entry(64 << 20, 100)]);

        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].entries.len(), 1);
        assert_eq!(plan[1].start, 64 << 20);
    }

    #[test]
    fn entries_are_ordered_before_they_are_grouped() {
        let plan = runs(vec![entry(2048, 100), entry(512, 100)]);

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].start, 512);
        assert_eq!(plan[0].entries[0].offset, 512);
    }

    #[test]
    fn a_run_never_grows_past_the_limit() {
        let missing: Vec<Entry> = (0..40).map(|n| entry(n * (1 << 20), 1 << 20)).collect();

        let plan = runs(missing);

        assert!(plan.len() > 1);
        assert!(plan.iter().all(|run| run.end - run.start <= RUN_LIMIT));
    }

    #[test]
    fn scattered_pictures_are_asked_for_one_by_one() {
        let missing: Vec<Entry> = (0..500).map(|n| entry(n * (32 << 20), 1000)).collect();

        let plan = runs(missing);

        assert_eq!(plan.len(), 500);
        let asked: u64 = plan.iter().map(|run| run.end - run.start).sum();
        assert_eq!(asked, 500 * 1000);
    }

    #[test]
    fn a_full_pack_collapses_into_few_requests() {
        let missing: Vec<Entry> = (0..12_000)
            .map(|n| entry(512 + n * 11_264, 10_000))
            .collect();

        let plan = runs(missing);

        let span: u64 = plan.iter().map(|run| run.end - run.start).sum();
        assert!(plan.len() <= 1 + span.div_ceil(RUN_LIMIT) as usize);
        assert_eq!(
            plan.iter().map(|run| run.entries.len()).sum::<usize>(),
            12_000
        );
    }
}
