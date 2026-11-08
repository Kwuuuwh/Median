use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use median_core::Status;
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};
use tracing::{error, info};

use crate::market::stream::{Ask, Stream};
use crate::update;

pub const MAIN: &str = "main";
pub const OVERLAY: &str = "overlay";
pub const LAUNCHER: &str = "launcher";

/// Whether game data is ready for the main window.
static DATA: AtomicBool = AtomicBool::new(false);
/// Whether the update step is over.
static SETTLED: AtomicBool = AtomicBool::new(false);
/// Whether the main window and the overlay were created.
static OPENED: AtomicBool = AtomicBool::new(false);
/// Whether the start failed, so only the launcher may open.
static FAILED: AtomicBool = AtomicBool::new(false);

/// Delay between sending the invisible status and exiting.
pub const PARTING: Duration = Duration::from_millis(600);

/// Opens the main window and the overlay when game data is `ready` and the build takes no
/// updates; shows the launcher otherwise.
pub fn start(app: &AppHandle, ready: bool) {
    if !update::enabled(app) {
        SETTLED.store(true, Ordering::SeqCst);
    }
    if ready {
        DATA.store(true, Ordering::SeqCst);
    }
    if open(app) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(why) = create(&app, LAUNCHER) {
            error!(error = %why, "launcher window not created");
        }
    });
}

/// Opens only the launcher, for a start that failed; the main window and the overlay never open.
pub fn fail(app: &AppHandle) {
    FAILED.store(true, Ordering::SeqCst);
    if app.get_webview_window(LAUNCHER).is_some() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(why) = create(&app, LAUNCHER) {
            error!(error = %why, "launcher window not created");
        }
    });
}

/// Records that game data is ready.
pub fn data_ready(app: &AppHandle) {
    DATA.store(true, Ordering::SeqCst);
    open(app);
}

/// Records that the launcher is done with updates.
pub fn update_settled(app: &AppHandle) {
    SETTLED.store(true, Ordering::SeqCst);
    open(app);
}

/// Creates the main window and the overlay once game data is ready and the update step is over;
/// `false` while either is pending.
fn open(app: &AppHandle) -> bool {
    if FAILED.load(Ordering::SeqCst) {
        return false;
    }
    if !(DATA.load(Ordering::SeqCst) && SETTLED.load(Ordering::SeqCst)) {
        return false;
    }
    if OPENED.swap(true, Ordering::SeqCst) {
        return true;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match create(&app, OVERLAY) {
            Ok(overlay) => {
                let _ = overlay.set_ignore_cursor_events(true);
                // Keeps the overlay out of screen copies.
                if crate::screen::capture() == Some(crate::screen::Capture::Screen) {
                    let _ = overlay.set_content_protected(true);
                }
                info!(size = ?overlay.outer_size().ok(), position = ?overlay.outer_position().ok(), "overlay window opened");
            }
            Err(why) => error!(error = %why, "overlay window not created"),
        }
        match create(&app, MAIN) {
            Ok(main) => {
                info!("main window opened");
                hide_on_close(&app, &main);
            }
            Err(why) => error!(error = %why, "main window not created"),
        }
    });
    true
}

/// Shows the main window once its page has rendered, and closes the launcher.
pub fn main_rendered(app: &AppHandle) {
    if let Some(main) = app.get_webview_window(MAIN) {
        let _ = main.show();
    }
    if let Some(launcher) = app.get_webview_window(LAUNCHER) {
        let _ = launcher.close();
    }
}

/// Brings the main window forward, or the launcher while there is no main window.
pub fn bring_forward(app: &AppHandle) {
    let Some(window) = app
        .get_webview_window(MAIN)
        .or_else(|| app.get_webview_window(LAUNCHER))
    else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// Builds the window `label` from its entry in `tauri.conf.json`, where it is `create: false`.
fn create(app: &AppHandle, label: &str) -> Result<WebviewWindow, String> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == label)
        .ok_or_else(|| format!("no window {label} in tauri.conf.json"))?;
    WebviewWindowBuilder::from_config(app, config)
        .and_then(|builder| builder.build())
        .map_err(|e| e.to_string())
}

/// On close, sets the warframe.market status to invisible, then destroys the overlay and the
/// main window.
fn hide_on_close(app: &AppHandle, main: &WebviewWindow) {
    let stream = app.state::<Stream>().inner().clone();
    let app = app.clone();
    let window = main.clone();
    let parting = std::sync::Arc::new(AtomicBool::new(false));
    main.on_window_event(move |event| {
        let tauri::WindowEvent::CloseRequested { api, .. } = event else {
            return;
        };
        // A second close request closes at once.
        if parting.swap(true, Ordering::Relaxed) {
            return;
        }

        api.prevent_close();
        stream.ask(Ask::Status(Status::Invisible));
        let window = window.clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(PARTING).await;
            if let Some(overlay) = app.get_webview_window(OVERLAY) {
                let _ = overlay.destroy();
            }
            let _ = window.destroy();
        });
    });
}
