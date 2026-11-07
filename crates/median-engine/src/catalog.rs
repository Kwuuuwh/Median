use median_core::{Asset, Release, Version};
use serde::Deserialize;

/// The fields of the published game data manifest that the client reads.
#[derive(Deserialize)]
struct RawManifest {
    schema: u32,
    version: String,
    db: RawAsset,
    pack: RawAsset,
}

#[derive(Deserialize)]
struct RawAsset {
    file: String,
    blake3: String,
    size: u64,
}

impl From<RawAsset> for Asset {
    fn from(raw: RawAsset) -> Self {
        Self {
            file: raw.file,
            blake3: raw.blake3,
            size: raw.size,
        }
    }
}

/// Parses a raw manifest body, or the reason it is unusable.
pub(crate) fn parse(body: &str) -> Result<Release, String> {
    let raw: RawManifest = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let version = Version::parse(&raw.version)
        .ok_or_else(|| format!("unreadable version {:?}", raw.version))?;
    Ok(Release {
        schema: raw.schema,
        version,
        db: raw.db.into(),
        pack: raw.pack.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    const MANIFEST: &str = r#"{
        "schema": 5,
        "version": "5.1.0",
        "fetched_ms": 1785037752337,
        "sources": { "de": "de-5138d412" },
        "db": { "file": "catalog.sqlite.gz", "blake3": "aca5c7ec", "size": 6903047, "unpacked": 52121600 },
        "pack": { "file": "pack.tar", "blake3": "148f8310", "size": 135200000, "count": 12631 },
        "total": 142103047
    }"#;

    #[test]
    fn reads_both_parts() {
        let release = parse(MANIFEST).unwrap();
        assert_eq!(release.schema, 5);
        assert_eq!(release.version.to_string(), "5.1.0");
        assert_eq!(release.db.file, "catalog.sqlite.gz");
        assert_eq!(release.db.size, 6903047);
        assert_eq!(release.pack.file, "pack.tar");
        assert_eq!(release.pack.size, 135200000);
    }

    #[test]
    fn an_unreadable_version_is_rejected() {
        let raw = MANIFEST.replace("\"5.1.0\"", "\"5.1\"");
        assert!(parse(&raw).is_err());
    }

    #[test]
    fn a_missing_part_is_rejected() {
        assert!(parse(r#"{"schema":5,"version":"5.1.0"}"#).is_err());
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(parse("<!doctype html>").is_err());
    }
}
