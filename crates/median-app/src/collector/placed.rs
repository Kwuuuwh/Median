use std::fs;
use std::path::{Path, PathBuf};

/// The collector manifest, copied last.
const MANIFEST: &str = "manifest.json";

/// Folder of the collector Overwolf loads, held in Tauri state.
pub struct CollectorFolder(pub PathBuf);

/// What placing the shipped collector found.
#[derive(Debug, PartialEq, Eq)]
pub enum Placed {
    /// This build carries no collector.
    Unshipped,
    /// The folder already holds this version.
    Current(String),
    /// The folder held another version, or none, and now holds the shipped one.
    Replaced { was: Option<String>, now: String },
}

/// Copies the collector in `shipped` into `folder` when the folder holds another version or none.
pub fn place(shipped: &Path, folder: &Path) -> Result<Placed, String> {
    let Some(now) = version_of(shipped) else {
        return Ok(Placed::Unshipped);
    };
    let was = version_of(folder);
    if was.as_deref() == Some(now.as_str()) {
        return Ok(Placed::Current(now));
    }

    copy_tree(shipped, folder)?;
    fs::copy(shipped.join(MANIFEST), folder.join(MANIFEST))
        .map_err(|e| format!("{MANIFEST}: {e}"))?;
    Ok(Placed::Replaced { was, now })
}

/// `meta.version` of the collector manifest in `dir`; `None` when there is none or it is unreadable.
fn version_of(dir: &Path) -> Option<String> {
    let raw = fs::read_to_string(dir.join(MANIFEST)).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&raw).ok()?;
    manifest["meta"]["version"].as_str().map(str::to_owned)
}

/// Copies every file under `from` into `to` except the top manifest, keeping subfolders.
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let target = to.join(path.file_name().unwrap_or_default());
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else if path != from.join(MANIFEST) {
            fs::copy(&path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty folder under the system temp folder, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("median-placed-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("a scratch folder");
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn collector(dir: &Path, version: &str, script: &str) {
        fs::create_dir_all(dir).expect("the folder");
        fs::write(
            dir.join(MANIFEST),
            format!(r#"{{"meta":{{"name":"median collector","version":"{version}"}}}}"#),
        )
        .expect("the manifest");
        fs::write(dir.join("background.js"), script).expect("the script");
    }

    #[test]
    fn a_local_build_ships_no_collector() {
        let scratch = Scratch::new("unshipped");

        let placed = place(&scratch.0.join("shipped"), &scratch.0.join("folder"));

        assert_eq!(placed, Ok(Placed::Unshipped));
        assert!(!scratch.0.join("folder").exists());
    }

    #[test]
    fn the_first_start_places_the_collector() {
        let scratch = Scratch::new("first");
        collector(&scratch.0.join("shipped"), "2.1.0", "new");

        let placed = place(&scratch.0.join("shipped"), &scratch.0.join("folder"));

        assert_eq!(
            placed,
            Ok(Placed::Replaced {
                was: None,
                now: "2.1.0".into()
            })
        );
        assert_eq!(
            version_of(&scratch.0.join("folder")).as_deref(),
            Some("2.1.0")
        );
    }

    #[test]
    fn the_same_version_is_left_alone() {
        let scratch = Scratch::new("same");
        collector(&scratch.0.join("shipped"), "2.1.0", "new");
        collector(&scratch.0.join("folder"), "2.1.0", "edited by hand");

        let placed = place(&scratch.0.join("shipped"), &scratch.0.join("folder"));

        assert_eq!(placed, Ok(Placed::Current("2.1.0".into())));
        let script = fs::read_to_string(scratch.0.join("folder/background.js")).unwrap();
        assert_eq!(script, "edited by hand");
    }

    #[test]
    fn another_version_is_replaced_whole() {
        let scratch = Scratch::new("other");
        collector(&scratch.0.join("shipped"), "2.2.0", "new");
        collector(&scratch.0.join("folder"), "2.1.0", "old");

        let placed = place(&scratch.0.join("shipped"), &scratch.0.join("folder"));

        assert_eq!(
            placed,
            Ok(Placed::Replaced {
                was: Some("2.1.0".into()),
                now: "2.2.0".into()
            })
        );
        let script = fs::read_to_string(scratch.0.join("folder/background.js")).unwrap();
        assert_eq!(script, "new");
        assert_eq!(
            version_of(&scratch.0.join("folder")).as_deref(),
            Some("2.2.0")
        );
    }
}
