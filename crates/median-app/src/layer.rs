use std::sync::{Arc, Mutex, PoisonError};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};
use tracing::{debug, warn};

use crate::screen::{self, Bounds};

/// Foreground state of the game window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Front {
    /// The foreground window is not tracked.
    Unwatched,
    /// The game window is in the foreground, at these bounds.
    Game(Bounds),
    /// Another window is in the foreground.
    Elsewhere,
}

impl Front {
    /// Whether both are the same variant, ignoring the game window's bounds.
    fn is_same_kind(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// Inputs that decide whether the overlay is shown.
#[derive(Debug, Clone, Copy)]
struct Standing {
    /// An in-game screen the overlay draws over is open.
    is_wanted: bool,
    front: Front,
}

/// Overlay window over the game: shown while an in-game screen it draws over is open and the
/// game window is in the foreground.
pub struct Layer {
    app: AppHandle,
    standing: Mutex<Standing>,
}

impl Layer {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            standing: Mutex::new(Standing {
                is_wanted: false,
                front: Front::Unwatched,
            }),
        })
    }

    /// Starts tracking the foreground window; until then, or if tracking fails, the overlay is
    /// shown over the game window whenever it is requested. `running` is told when the game
    /// starts or stops.
    pub fn watch(self: &Arc<Self>, running: impl Fn(bool) + Send + 'static) {
        let layer = Arc::clone(self);
        let seen = move |bounds: Option<Bounds>| {
            layer.change(|standing| {
                let front = bounds.map_or(Front::Elsewhere, Front::Game);
                if !front.is_same_kind(standing.front) {
                    debug!(foreground = ?front, overlay_requested = standing.is_wanted, "foreground window changed");
                }
                standing.front = front;
            });
        };
        let watched = screen::watch_front(self.handle(), seen, running);
        if let Err(why) = watched {
            warn!(error = %why, "foreground watch failed, overlay shown over all windows");
        }
    }

    /// Requests (`true`) or releases (`false`) the overlay as an in-game screen it draws over
    /// opens or closes.
    pub fn want(&self, is_wanted: bool) {
        self.change(|standing| standing.is_wanted = is_wanted);
    }

    /// Raw HWND of the overlay window; 0 when there is none or off Windows.
    fn handle(&self) -> isize {
        #[cfg(windows)]
        if let Some(window) = self.app.get_webview_window(crate::window::OVERLAY)
            && let Ok(handle) = window.hwnd()
        {
            return handle.0 as isize;
        }
        0
    }

    fn change(&self, edit: impl FnOnce(&mut Standing)) {
        let mut standing = self.standing.lock().unwrap_or_else(PoisonError::into_inner);
        edit(&mut standing);
        self.draw(*standing);
    }

    fn draw(&self, standing: Standing) {
        // The overlay is created with the main window, once game data is ready.
        let Some(window) = self.app.get_webview_window(crate::window::OVERLAY) else {
            debug!("overlay window not created yet");
            return;
        };

        let bounds = match (standing.is_wanted, standing.front) {
            (false, _) | (true, Front::Elsewhere) => {
                let _ = window.hide();
                return;
            }
            (true, Front::Game(bounds)) => bounds,
            (true, Front::Unwatched) => match screen::bounds() {
                Ok(bounds) => bounds,
                Err(why) => {
                    warn!(error = %why, "game window not found, overlay not drawn");
                    return;
                }
            },
        };
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.set_position(PhysicalPosition::new(bounds.x, bounds.y));
        let _ = window.set_size(PhysicalSize::new(bounds.width, bounds.height));
        let _ = window.show();
    }
}
