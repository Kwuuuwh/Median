use std::cell::{Cell, OnceCell, RefCell};
use std::ptr;
use std::sync::mpsc::{self, Sender};
use std::thread;

use tracing::debug;
use windows_capture::window::Window;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND, GetMessageW, IsWindow,
    MSG, OBJID_WINDOW, SetTimer, TranslateMessage, WINEVENT_OUTOFCONTEXT,
};

use super::grab::{bounds_of, game, is_game, is_minimized};
use super::{Bounds, ScreenError};

/// Callback that receives the game window's bounds while it is in the foreground, and `None`
/// once it is not.
pub(super) type Seen = Box<dyn Fn(Option<Bounds>) + Send>;

/// Callback that receives `true` once a game window is seen and `false` once none is left.
pub(super) type Running = Box<dyn Fn(bool) + Send>;

/// The game window while it is in the foreground, and the move hook on its process.
#[derive(Default)]
struct Watch {
    game: Option<HWND>,
    moves: Option<Moves>,
}

/// A location-change hook on the windows of one process.
struct Moves {
    hook: HWINEVENTHOOK,
    process: u32,
}

thread_local! {
    static SEEN: OnceCell<Seen> = const { OnceCell::new() };
    static RUNNING: OnceCell<Running> = const { OnceCell::new() };
    /// A game window known to exist, in the foreground or not; null when none is known.
    static GAME_WINDOW: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
    /// Running state last passed to the `RUNNING` callback.
    static IS_RUNNING: Cell<bool> = const { Cell::new(false) };
    static WATCH: RefCell<Watch> = RefCell::new(Watch::default());
    static LAYER: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
    /// Bounds last passed to the callback; `None` before the first call.
    static LAST: RefCell<Option<Option<Bounds>>> = const { RefCell::new(None) };
}

/// Interval in milliseconds of the foreground re-check, which corrects a missed or out-of-order
/// event.
const RECHECK_MS: u32 = 1000;

/// Starts a thread that calls `seen` on every change of the game window's foreground state or
/// bounds, ignoring the window `layer`, and `running` when the game starts or stops.
pub(super) fn watch(layer: isize, seen: Seen, running: Running) -> Result<(), ScreenError> {
    let (started, outcome) = mpsc::channel();
    thread::Builder::new()
        .name("game-front".into())
        .spawn(move || run(layer as HWND, seen, running, &started))
        .map_err(|_| ScreenError::Unwatched)?;
    outcome.recv().map_err(|_| ScreenError::Unwatched)?
}

fn run(layer: HWND, seen: Seen, running: Running, started: &Sender<Result<(), ScreenError>>) {
    // SAFETY: an out-of-context hook with a callback that lives as long as the program.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            ptr::null_mut(),
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_null() {
        let _ = started.send(Err(ScreenError::Unwatched));
        return;
    }
    SEEN.with(|cell| {
        let _ = cell.set(seen);
    });
    RUNNING.with(|cell| {
        let _ = cell.set(running);
    });
    LAYER.set(layer);
    let _ = started.send(Ok(()));

    // The game may already be running behind other windows.
    find_game();
    look();
    // SAFETY: a thread timer whose callback lives as long as the program; it is delivered
    // through this thread's message queue below.
    unsafe { SetTimer(ptr::null_mut(), 0, RECHECK_MS, Some(on_tick)) };

    // SAFETY: MSG is plain data; GetMessageW fills it before it is read.
    let mut message: MSG = unsafe { std::mem::zeroed() };
    // SAFETY: the hooks deliver their events through this thread's message queue.
    while unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn on_foreground(
    _: HWINEVENTHOOK,
    _: u32,
    _: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    look();
}

unsafe extern "system" fn on_tick(_: HWND, _: u32, _: usize, _: u32) {
    look();
    check_game();
}

/// Looks for another game window when the known one is gone; the game has stopped when there is
/// none. A game can replace its window (a splash screen, then the main window).
fn check_game() {
    let known = GAME_WINDOW.get();
    // SAFETY: IsWindow only tests the handle; a stale handle tests as not a window.
    if !known.is_null() && unsafe { IsWindow(known) } == 0 {
        GAME_WINDOW.set(ptr::null_mut());
        find_game();
    }
}

/// Remembers a game window found among all top-level windows, or reports the game stopped.
fn find_game() {
    match game() {
        Ok(window) => {
            GAME_WINDOW.set(window.as_raw_hwnd());
            report_running(true);
        }
        Err(_) => report_running(false),
    }
}

/// Passes `running` to the callback when it differs from the last value passed.
fn report_running(running: bool) {
    if IS_RUNNING.replace(running) == running {
        return;
    }
    RUNNING.with(|cell| {
        if let Some(report) = cell.get() {
            report(running);
        }
    });
}

/// Handles the current foreground window rather than the one an event names: hook events
/// arrive queued and may name a window that is no longer in the foreground. When no window is
/// in the foreground, nothing changes.
fn look() {
    if let Ok(window) = Window::foreground() {
        fronted(window.as_raw_hwnd());
    }
}

unsafe extern "system" fn on_move(
    _: HWINEVENTHOOK,
    _: u32,
    window: HWND,
    object: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if object == OBJID_WINDOW {
        moved(window);
    }
}

/// Handles a new foreground window; the overlay window is ignored.
fn fronted(handle: HWND) {
    if handle.is_null() || handle == LAYER.get() {
        return;
    }
    let window = Window::from_raw_hwnd(handle);
    let bounds = if is_game(&window) {
        GAME_WINDOW.set(handle);
        report_running(true);
        WATCH.with(|watch| {
            let mut watch = watch.borrow_mut();
            watch.game = Some(handle);
            if let Ok(process) = window.process_id() {
                watch.follow(process);
            }
        });
        shown(handle)
    } else {
        WATCH.with(|watch| watch.borrow_mut().game = None);
        if LAST.with(|last| matches!(*last.borrow(), Some(Some(_)))) {
            debug!(
                process = window.process_name().unwrap_or_default(),
                "game window lost foreground"
            );
        }
        None
    };
    tell(bounds);
}

/// Handles a moved window; only a move of the game window in the foreground is passed on.
fn moved(handle: HWND) {
    let is_front_game = WATCH.with(|watch| watch.borrow().game == Some(handle));
    if is_front_game {
        tell(shown(handle));
    }
}

/// Bounds of the game window on the screen; `None` when it is minimized or its bounds cannot
/// be read.
fn shown(handle: HWND) -> Option<Bounds> {
    let window = Window::from_raw_hwnd(handle);
    if is_minimized(&window) {
        return None;
    }
    bounds_of(&window).ok()
}

/// Passes `bounds` to the callback when they differ from the last value passed.
fn tell(bounds: Option<Bounds>) {
    let is_new = LAST.with(|last| last.replace(Some(bounds)) != Some(bounds));
    if !is_new {
        return;
    }
    SEEN.with(|cell| {
        if let Some(seen) = cell.get() {
            seen(bounds);
        }
    });
}

impl Watch {
    /// Hooks the window moves of `process`, removing the hook on the previous process.
    fn follow(&mut self, process: u32) {
        if self
            .moves
            .as_ref()
            .is_some_and(|moves| moves.process == process)
        {
            return;
        }
        if let Some(moves) = self.moves.take() {
            // SAFETY: the hook was set by this thread and is unhooked once.
            unsafe { UnhookWinEvent(moves.hook) };
        }
        // SAFETY: an out-of-context hook with a callback that lives as long as the program.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                ptr::null_mut(),
                Some(on_move),
                process,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if !hook.is_null() {
            self.moves = Some(Moves { hook, process });
        }
    }
}
