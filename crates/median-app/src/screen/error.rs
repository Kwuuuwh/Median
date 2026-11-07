use thiserror::Error;

/// Why no frame of the game window was captured.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScreenError {
    /// The game is not running, or its window cannot be found.
    #[error("the game's window is not there")]
    NoWindow,

    /// The game's window is minimized.
    #[error("the game's window is minimized")]
    Minimized,

    /// The capture failed or returned no frame.
    #[error("the game's window could not be captured: {0}")]
    Capture(String),

    /// Foreground window tracking could not be started.
    #[error("the window in front could not be watched")]
    Unwatched,

    /// Capture uses a Windows API and is not available on other systems.
    #[cfg(not(windows))]
    #[error("frames are only taken on Windows")]
    Unsupported,
}
