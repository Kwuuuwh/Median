use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use tracing_subscriber::fmt::MakeWriter;

/// Log file size in bytes (8 MiB) at which `open` moves `median.log` to `median.log.1`.
const MOST: u64 = 8 * 1024 * 1024;

/// Name of the log file in the data folder.
pub const NAME: &str = "median.log";

static FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// Starts appending the log to `root/median.log`; lines logged before this call are not in the
/// file. A file of at least `MOST` bytes is first moved to `median.log.1`.
pub fn open(root: &Path) -> io::Result<()> {
    let path = root.join(NAME);
    if fs::metadata(&path).is_ok_and(|held| held.len() >= MOST) {
        let _ = fs::rename(&path, root.join("median.log.1"));
    }

    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    let _ = FILE.set(Mutex::new(file));
    Ok(())
}

/// Logs every panic before the default hook prints it. A panic in a spawned task ends only that
/// task, and a release build has no console, so without this hook the panic is lost.
pub fn log_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let location = panic
            .location()
            .map(|at| format!("{}:{}", at.file(), at.line()))
            .unwrap_or_default();
        let payload = panic.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("none");
        let thread = std::thread::current();
        tracing::error!(
            thread = thread.name().unwrap_or("unnamed"),
            %location,
            message,
            "thread panicked"
        );
        default(panic);
    }));
}

/// Returns the Windows version as `major.minor.build`, such as `10.0.26100`, or `unknown` when
/// the call fails; build 22000 and above is Windows 11. `RtlGetVersion` is used rather than
/// `GetVersionEx`, which reports 6.2 to programs without a compatibility manifest.
#[cfg(windows)]
pub fn windows_version() -> String {
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;

    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is a fully initialized OSVERSIONINFOW with `dwOSVersionInfoSize` set, as
    // RtlGetVersion requires.
    if unsafe { RtlGetVersion(&mut info) } != 0 {
        return "unknown".to_owned();
    }
    format!(
        "{}.{}.{}",
        info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
    )
}

#[cfg(not(windows))]
pub fn windows_version() -> String {
    "none".to_owned()
}

/// Writer of the tracing file layer. Until `open` succeeds it discards every line, so the app
/// starts even when no log file can be created.
#[derive(Clone, Copy)]
pub struct ToFile;

impl Write for ToFile {
    fn write(&mut self, line: &[u8]) -> io::Result<usize> {
        let Some(file) = FILE.get() else {
            return Ok(line.len());
        };
        let mut file = file.lock().unwrap_or_else(|held| held.into_inner());
        file.write_all(line)?;
        Ok(line.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let Some(file) = FILE.get() else {
            return Ok(());
        };
        let mut file = file.lock().unwrap_or_else(|held| held.into_inner());
        file.flush()
    }
}

impl<'a> MakeWriter<'a> for ToFile {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        *self
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn the_windows_version_is_three_numbers() {
        let version = super::windows_version();
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "{version}");
        assert!(
            parts.iter().all(|part| part.parse::<u32>().is_ok()),
            "{version}"
        );
    }
}
