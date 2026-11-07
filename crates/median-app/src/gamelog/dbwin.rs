use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null;
use std::sync::atomic::{AtomicBool, Ordering};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_READ, MapViewOfFile, PAGE_READWRITE, UnmapViewOfFile,
};
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};

/// Size in bytes of the DBWIN shared buffer: the writer's 4-byte process id, then the message.
const BUFFER: usize = 4096;

/// Milliseconds a wait for a message lasts before the loop checks whether it should stop.
const TICK: u32 = 500;

const BUFFER_READY: &str = "DBWIN_BUFFER_READY";
const DATA_READY: &str = "DBWIN_DATA_READY";
const SHARED: &str = "DBWIN_BUFFER";

fn wide(name: &str) -> Vec<u16> {
    OsStr::new(name).encode_wide().chain(Some(0)).collect()
}

/// Windows handle that is closed on drop.
struct Held(HANDLE);

impl Drop for Held {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Reads the Windows debug output (DBWIN) and passes each complete line to `say`, until
/// `running` becomes false.
///
/// `shared` is called once with whether another debug output reader already had the buffer open.
pub fn listen(
    running: &AtomicBool,
    mut shared: impl FnMut(bool),
    mut say: impl FnMut(&str),
) -> Result<(), String> {
    unsafe {
        let ready = Held(CreateEventW(null(), 0, 1, wide(BUFFER_READY).as_ptr()));
        if ready.0.is_null() {
            return Err(format!("the ready event did not open: {}", GetLastError()));
        }
        let data = Held(CreateEventW(null(), 0, 0, wide(DATA_READY).as_ptr()));
        if data.0.is_null() {
            return Err(format!("the data event did not open: {}", GetLastError()));
        }

        let map = Held(CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            null(),
            PAGE_READWRITE,
            0,
            BUFFER as u32,
            wide(SHARED).as_ptr(),
        ));
        if map.0.is_null() {
            return Err(format!(
                "the shared buffer did not open: {}",
                GetLastError()
            ));
        }
        shared(GetLastError() == ERROR_ALREADY_EXISTS);

        let view = MapViewOfFile(map.0, FILE_MAP_READ, 0, 0, BUFFER);
        if view.Value.is_null() {
            return Err(format!("the shared buffer did not map: {}", GetLastError()));
        }

        let mut held = String::new();
        SetEvent(ready.0);
        while running.load(Ordering::Relaxed) {
            if WaitForSingleObject(data.0, TICK) != WAIT_OBJECT_0 {
                SetEvent(ready.0);
                continue;
            }

            let bytes = std::slice::from_raw_parts(view.Value.cast::<u8>(), BUFFER);
            let message = &bytes[4..];
            let end = message.iter().position(|byte| *byte == 0).unwrap_or(0);
            held.push_str(&String::from_utf8_lossy(&message[..end]));
            SetEvent(ready.0);

            while let Some(split) = held.find('\n') {
                let line: String = held.drain(..=split).collect();
                say(line.trim_end_matches(['\r', '\n']));
            }
            // Log lines are short; a partial line this long will never get its newline.
            if held.len() > BUFFER * 8 {
                held.clear();
            }
        }

        UnmapViewOfFile(view);
        Ok(())
    }
}
