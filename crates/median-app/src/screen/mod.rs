mod error;
mod place;

#[cfg(windows)]
mod copy;
#[cfg(windows)]
mod front;
#[cfg(windows)]
mod grab;

pub use error::ScreenError;
pub use place::placed;

/// One frame of the game's window, BGRA rows top to bottom.
pub struct Shot {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// How frames of the game are taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// Windows Graphics Capture of the game window.
    Window,
    /// GDI copy of the game window's area of the screen.
    Screen,
}

impl Capture {
    /// Name of the way in the log.
    pub fn as_code(self) -> &'static str {
        match self {
            Capture::Window => "window",
            Capture::Screen => "screen",
        }
    }
}

/// How frames of the game are taken on this Windows; `None` where frames are not taken.
#[cfg(windows)]
pub fn capture() -> Option<Capture> {
    Some(grab::capture())
}

#[cfg(not(windows))]
pub fn capture() -> Option<Capture> {
    None
}

/// Position and size of the game window on the desktop, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Current bounds of the game window.
#[cfg(windows)]
pub fn bounds() -> Result<Bounds, ScreenError> {
    grab::bounds()
}

#[cfg(not(windows))]
pub fn bounds() -> Result<Bounds, ScreenError> {
    Err(ScreenError::Unsupported)
}

/// Calls `seen` whenever the game window's foreground state or bounds change: with its bounds
/// while it is in the foreground and not minimized, `None` otherwise. The window `layer` (the
/// overlay) is ignored. Calls `running` with `true` once a game window is seen, at the start or in
/// the foreground, and with `false` once no game window is left.
#[cfg(windows)]
pub fn watch_front(
    layer: isize,
    seen: impl Fn(Option<Bounds>) + Send + 'static,
    running: impl Fn(bool) + Send + 'static,
) -> Result<(), ScreenError> {
    front::watch(layer, Box::new(seen), Box::new(running))
}

#[cfg(not(windows))]
pub fn watch_front(
    _: isize,
    _: impl Fn(Option<Bounds>) + Send + 'static,
    _: impl Fn(bool) + Send + 'static,
) -> Result<(), ScreenError> {
    Err(ScreenError::Unsupported)
}

/// Captures one frame of the game window, blocking until the capture delivers it.
#[cfg(windows)]
pub fn shot() -> Result<Shot, ScreenError> {
    grab::once()
}

#[cfg(not(windows))]
pub fn shot() -> Result<Shot, ScreenError> {
    Err(ScreenError::Unsupported)
}
