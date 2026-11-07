use std::cmp::Reverse;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use median_core::Version;

const CATALOG: (&str, &str) = ("catalog-", ".sqlite");
const INDEX: (&str, &str) = ("pack-", ".index");
const PACK: &str = "pack";
const PARTIAL: &str = "pack.part.index";

/// Directory that holds the catalog files, the picture pack and the pack indexes.
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    /// Creates the root and pack directories when missing.
    pub fn create(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(root.join(PACK))?;
        Ok(Self { root })
    }

    /// Path a catalog of `version` is stored at.
    pub fn catalog(&self, version: Version) -> PathBuf {
        self.root.join(name(CATALOG, version))
    }

    /// Catalog files present, newest version first.
    pub fn catalogs(&self) -> Vec<(Version, PathBuf)> {
        self.matching(CATALOG)
    }

    /// The directory pictures are written to, one file per picture.
    pub fn pack(&self) -> PathBuf {
        self.root.join(PACK)
    }

    /// Path the index of a complete pack of `version` is stored at.
    pub fn index(&self, version: Version) -> PathBuf {
        self.root.join(name(INDEX, version))
    }

    /// Index files of complete packs, newest version first.
    pub fn indexes(&self) -> Vec<(Version, PathBuf)> {
        self.matching(INDEX)
    }

    /// Path of the index of a pack whose download is in progress.
    pub fn partial_index(&self) -> PathBuf {
        self.root.join(PARTIAL)
    }

    /// Files in the root named `<prefix><version><suffix>` for `shape`, newest version first.
    fn matching(&self, shape: (&str, &str)) -> Vec<(Version, PathBuf)> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut found: Vec<(Version, PathBuf)> = entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                Some((version_of(&path, shape)?, path))
            })
            .collect();
        found.sort_by_key(|(version, _)| Reverse(*version));
        found
    }
}

fn name(shape: (&str, &str), version: Version) -> String {
    format!("{}{version}{}", shape.0, shape.1)
}

/// Version in a file name of the form `<prefix><version><suffix>`; `None` for any other name.
fn version_of(path: &Path, shape: (&str, &str)) -> Option<Version> {
    let name = path.file_name()?.to_str()?;
    Version::parse(name.strip_prefix(shape.0)?.strip_suffix(shape.1)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_version_out_of_a_file_name() {
        assert_eq!(
            version_of(Path::new("/data/catalog-5.1.0.sqlite"), CATALOG),
            Version::parse("5.1.0")
        );
        assert_eq!(
            version_of(Path::new("/data/pack-5.1.0.index"), INDEX),
            Version::parse("5.1.0")
        );
    }

    #[test]
    fn ignores_anything_else_in_the_directory() {
        assert_eq!(
            version_of(Path::new("/data/catalog-5.1.0.part"), CATALOG),
            None
        );
        assert_eq!(version_of(Path::new("/data/pack.part.index"), INDEX), None);
        assert_eq!(
            version_of(Path::new("/data/pack-5.1.0.index"), CATALOG),
            None
        );
        assert_eq!(
            version_of(Path::new("/data/catalog-.sqlite"), CATALOG),
            None
        );
    }
}
