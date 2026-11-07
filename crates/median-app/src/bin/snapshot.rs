use std::path::PathBuf;
use std::process::ExitCode;

/// Records the IPC command answers for a copy of the app's data folder and an optional
/// world-state file.
fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let (Some(root), Some(out)) = (args.next(), args.next()) else {
        eprintln!("usage: snapshot <data folder> <out folder> [world-state document]");
        return ExitCode::FAILURE;
    };
    let world = args.next();
    match tauri::async_runtime::block_on(median_app::snapshot::record(
        &root,
        &out,
        world.as_deref(),
    )) {
        Ok(()) => {
            println!("wrote {}", out.display());
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}
