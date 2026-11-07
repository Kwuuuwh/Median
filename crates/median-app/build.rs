use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Channels a build can be made for.
const CHANNELS: [&str; 3] = ["stable", "preview", "dev"];

fn main() {
    let built = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default();
    println!("cargo:rustc-env=MEDIAN_BUILT={built}");

    // Empty outside a git repository.
    let hash = git(&["rev-parse", "--short=9", "HEAD"]);
    println!("cargo:rustc-env=MEDIAN_HASH={hash}");
    // The revision is the number of commits in the branch.
    let revision: u32 = git(&["rev-list", "--count", "HEAD"])
        .parse()
        .unwrap_or_default();
    println!("cargo:rustc-env=MEDIAN_REVISION={revision}");

    // `dev` unless CI sets the channel.
    println!("cargo:rerun-if-env-changed=MEDIAN_CHANNEL");
    let channel = std::env::var("MEDIAN_CHANNEL").unwrap_or_else(|_| "dev".to_owned());
    assert!(
        CHANNELS.contains(&channel.as_str()),
        "MEDIAN_CHANNEL is {channel:?}; it must be one of {CHANNELS:?}"
    );
    println!("cargo:rustc-env=MEDIAN_CHANNEL={channel}");

    // Update manifest URL with `{channel}`; empty for a build that takes no updates.
    println!("cargo:rerun-if-env-changed=MEDIAN_UPDATE_SOURCE");
    let source = std::env::var("MEDIAN_UPDATE_SOURCE").unwrap_or_default();
    assert!(
        source.is_empty() || source.contains("{channel}"),
        "MEDIAN_UPDATE_SOURCE is {source:?}; it must hold {{channel}}"
    );
    println!("cargo:rustc-env=MEDIAN_UPDATE_SOURCE={source}");

    // File with the changes since the previous release, one per line.
    println!("cargo:rerun-if-env-changed=MEDIAN_NOTES_FILE");
    let notes = match std::env::var("MEDIAN_NOTES_FILE") {
        Ok(path) if !path.is_empty() => {
            println!("cargo:rerun-if-changed={path}");
            fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("MEDIAN_NOTES_FILE {path:?} is unreadable: {e}"))
        }
        _ => String::new(),
    };
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR for build scripts");
    fs::write(Path::new(&out).join("notes.txt"), notes).expect("notes.txt is writable");

    // Reruns on a new commit or checkout.
    let git_dir = git(&["rev-parse", "--absolute-git-dir"]);
    if !git_dir.is_empty() {
        for path in ["HEAD", "refs/heads", "packed-refs"] {
            println!("cargo:rerun-if-changed={git_dir}/{path}");
        }
    }

    tauri_build::build();
}

/// Output of `git args`, trimmed; empty when git is missing or fails.
fn git(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}
