use std::ptr::null_mut;

use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HBITMAP, HDC, ReleaseDC, SRCCOPY,
    SelectObject,
};

use super::{Bounds, ScreenError, Shot};

/// The screen's device context, released on drop.
struct ScreenDc(HDC);

impl Drop for ScreenDc {
    fn drop(&mut self) {
        // SAFETY: the context came from `GetDC(null)` and is released once.
        unsafe { ReleaseDC(null_mut(), self.0) };
    }
}

/// A memory device context, deleted on drop.
struct MemoryDc(HDC);

impl Drop for MemoryDc {
    fn drop(&mut self) {
        // SAFETY: the context came from `CreateCompatibleDC` and is deleted once.
        unsafe { DeleteDC(self.0) };
    }
}

/// A bitmap, deleted on drop.
struct Bitmap(HBITMAP);

impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY: the bitmap came from `CreateCompatibleBitmap` and is deleted once.
        unsafe { DeleteObject(self.0) };
    }
}

/// Copies `bounds` of the screen, in physical pixels, as BGRA rows top to bottom.
pub(super) fn area(bounds: Bounds) -> Result<Shot, ScreenError> {
    let failed = |call: &str| ScreenError::Capture(format!("{call} failed"));
    let (Ok(width), Ok(height)) = (i32::try_from(bounds.width), i32::try_from(bounds.height))
    else {
        return Err(ScreenError::Capture(
            "the game's window is larger than a screen".into(),
        ));
    };
    if width == 0 || height == 0 {
        return Err(ScreenError::Capture("the game's window has no area".into()));
    }

    // SAFETY: every handle is checked and released by its guard; the buffer holds the rows asked
    // for.
    unsafe {
        let screen = ScreenDc(GetDC(null_mut()));
        if screen.0.is_null() {
            return Err(failed("GetDC"));
        }
        let bitmap = Bitmap(CreateCompatibleBitmap(screen.0, width, height));
        if bitmap.0.is_null() {
            return Err(failed("CreateCompatibleBitmap"));
        }
        let memory = MemoryDc(CreateCompatibleDC(screen.0));
        if memory.0.is_null() {
            return Err(failed("CreateCompatibleDC"));
        }

        let before = SelectObject(memory.0, bitmap.0);
        let copied = BitBlt(
            memory.0, 0, 0, width, height, screen.0, bounds.x, bounds.y, SRCCOPY,
        );
        SelectObject(memory.0, before);
        if copied == 0 {
            return Err(failed("BitBlt"));
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Rows top to bottom.
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bytes = vec![0u8; bounds.width as usize * bounds.height as usize * 4];
        let rows = GetDIBits(
            screen.0,
            bitmap.0,
            0,
            bounds.height,
            bytes.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        );
        if rows != height {
            return Err(failed("GetDIBits"));
        }

        Ok(Shot {
            bytes,
            width: bounds.width,
            height: bounds.height,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corner_of_the_screen_comes_back_as_bgra_rows() {
        let shot = area(Bounds {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
        })
        .expect("a copy of the screen");

        assert_eq!((shot.width, shot.height), (64, 48));
        assert_eq!(shot.bytes.len(), 64 * 48 * 4);
    }

    #[test]
    fn an_empty_area_is_refused() {
        let empty = Bounds {
            x: 0,
            y: 0,
            width: 0,
            height: 10,
        };
        assert!(matches!(area(empty), Err(ScreenError::Capture(_))));
    }
}
