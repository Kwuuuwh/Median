use std::fs;
use std::io::Read;

use flate2::read::GzDecoder;
use median_core::Release;

use crate::catalog::db::Open;
use crate::catalog::paths::Layout;

/// Checks the archive's BLAKE3 hash against the manifest, writes the unpacked catalog into
/// `layout` and opens it; deletes the file and fails when its version differs from the release.
pub fn unpack(layout: &Layout, release: &Release, archive: &[u8]) -> Result<Open, String> {
    let digest = blake3::hash(archive).to_hex();
    if digest.as_str() != release.db.blake3 {
        return Err(format!(
            "archive hashes to {digest}, the manifest says {}",
            release.db.blake3
        ));
    }

    let mut database = Vec::new();
    GzDecoder::new(archive)
        .read_to_end(&mut database)
        .map_err(|e| format!("unpack: {e}"))?;

    let path = layout.catalog(release.version);
    let partial = path.with_extension("part");
    fs::write(&partial, &database).map_err(|e| format!("write {}: {e}", partial.display()))?;
    fs::rename(&partial, &path).map_err(|e| format!("rename to {}: {e}", path.display()))?;

    match Open::at(path.clone()) {
        Ok(open) if open.version() == release.version => Ok(open),
        Ok(open) => {
            let declared = open.version();
            drop(open);
            let _ = fs::remove_file(&path);
            Err(format!(
                "catalog declares {declared}, the manifest promised {}",
                release.version
            ))
        }
        Err(e) => {
            let _ = fs::remove_file(&path);
            Err(e)
        }
    }
}
