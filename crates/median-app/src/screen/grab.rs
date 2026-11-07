use std::convert::Infallible;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};

use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl};
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use windows_capture::window::Window;
use windows_sys::Win32::UI::WindowsAndMessaging::IsIconic;

use super::{Bounds, Capture, ScreenError, Shot, copy};

/// Process name of the game, as the module base name spells it.
const GAME: &str = "Warframe.x64.exe";

/// How frames are taken on this Windows: window capture where it can go without a border, a
/// screen copy otherwise.
pub(super) fn capture() -> Capture {
    static CHOSEN: OnceLock<Capture> = OnceLock::new();
    *CHOSEN.get_or_init(|| {
        if GraphicsCaptureApi::is_border_settings_supported().unwrap_or(false) {
            Capture::Window
        } else {
            Capture::Screen
        }
    })
}

/// Takes one frame of the game window, the way `capture` chose.
pub(super) fn once() -> Result<Shot, ScreenError> {
    let window = game()?;
    if is_minimized(&window) {
        return Err(ScreenError::Minimized);
    }
    match capture() {
        Capture::Window => first_frame(window),
        Capture::Screen => copy::area(bounds_of(&window)?),
    }
}

/// `wanted` where this Windows has the setting, `fallback` otherwise.
fn supported<T>(
    has: Result<bool, windows_capture::graphics_capture_api::Error>,
    wanted: T,
    fallback: T,
) -> T {
    if has.unwrap_or(false) {
        wanted
    } else {
        fallback
    }
}

/// Captures the first frame of `window` and stops the capture.
fn first_frame(window: Window) -> Result<Shot, ScreenError> {
    let (taken, arrived) = mpsc::channel();
    let settings = Settings::new(
        window,
        supported(
            GraphicsCaptureApi::is_cursor_settings_supported(),
            CursorCaptureSettings::WithoutCursor,
            CursorCaptureSettings::Default,
        ),
        DrawBorderSettings::WithoutBorder,
        supported(
            GraphicsCaptureApi::is_secondary_windows_supported(),
            SecondaryWindowSettings::Exclude,
            SecondaryWindowSettings::Default,
        ),
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        taken,
    );

    Grab::start(settings).map_err(|trouble| ScreenError::Capture(trouble.to_string()))?;

    arrived
        .try_recv()
        .map_err(|_| ScreenError::Capture("the window handed over no frame".into()))?
        .map_err(ScreenError::Capture)
}

/// Bounds of the game window on the desktop.
pub(super) fn bounds() -> Result<Bounds, ScreenError> {
    bounds_of(&game()?)
}

/// Bounds of `window` on the desktop; a negative width or height counts as 0.
pub(super) fn bounds_of(window: &Window) -> Result<Bounds, ScreenError> {
    let rect = window
        .rect()
        .map_err(|trouble| ScreenError::Capture(trouble.to_string()))?;

    Ok(Bounds {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(0) as u32,
        height: (rect.bottom - rect.top).max(0) as u32,
    })
}

/// Whether `window` is minimized; a window that is gone reads as not minimized.
pub(super) fn is_minimized(window: &Window) -> bool {
    // SAFETY: IsIconic only reads the window state.
    unsafe { IsIconic(window.as_raw_hwnd()) != 0 }
}

/// Whether `window` belongs to the game process.
pub(super) fn is_game(window: &Window) -> bool {
    window
        .process_name()
        .is_ok_and(|name| name.eq_ignore_ascii_case(GAME))
}

/// First capturable window of the game process; `NoWindow` when there is none.
pub(super) fn game() -> Result<Window, ScreenError> {
    Window::enumerate()
        .map_err(|trouble| ScreenError::Capture(trouble.to_string()))?
        .into_iter()
        .find(is_game)
        .ok_or(ScreenError::NoWindow)
}

/// Capture handler that sends the first frame to `taken`, then stops.
struct Grab {
    taken: Sender<Result<Shot, String>>,
}

impl GraphicsCaptureApiHandler for Grab {
    type Flags = Sender<Result<Shot, String>>;
    type Error = Infallible;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            taken: context.flags,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let _ = self.taken.send(pixels(frame));
        capture.stop();
        Ok(())
    }
}

/// Copies the frame's pixels without the padding the capture adds at the end of each row.
fn pixels(frame: &mut Frame) -> Result<Shot, String> {
    let buffer = frame.buffer().map_err(|trouble| trouble.to_string())?;
    let (width, height) = (buffer.width(), buffer.height());
    let mut packed = Vec::new();

    Ok(Shot {
        bytes: buffer.as_nopadding_buffer(&mut packed).to_vec(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the game running, so it is ignored in a normal test run.
    #[test]
    #[ignore]
    fn takes_a_frame_of_the_running_game() {
        let shot = once().expect("a frame of the game");

        assert!(shot.width > 0 && shot.height > 0);
        assert_eq!(
            shot.bytes.len(),
            shot.width as usize * shot.height as usize * 4
        );
    }
}
