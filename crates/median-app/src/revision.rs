/// Number of commits in the branch the build was made from; `0` outside a git repository.
pub const REVISION: &str = env!("MEDIAN_REVISION");

/// Short hash of the build's commit, 9 characters; empty outside a git repository.
pub const HASH: &str = env!("MEDIAN_HASH");

/// Channel the build was made for: `stable`, `preview`, or `dev` for a local build.
pub const CHANNEL: &str = env!("MEDIAN_CHANNEL");

/// Compile time, Unix seconds.
pub const BUILT: &str = env!("MEDIAN_BUILT");

/// Changes since the previous release, one per line; empty for a local build.
const NOTES: &str = include_str!(concat!(env!("OUT_DIR"), "/notes.txt"));

/// The changes this build brought, one per entry; empty for a local build.
pub fn changes() -> Vec<String> {
    lines(NOTES)
}

/// Non-blank lines of `notes`, trimmed.
pub fn lines(notes: &str) -> Vec<String> {
    notes
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_keep_one_change_per_line() {
        assert_eq!(
            lines("fix(market): read 509\r\n\n  feat(app): sign in  \n"),
            vec!["fix(market): read 509", "feat(app): sign in"]
        );
        assert!(lines("").is_empty());
    }
}
