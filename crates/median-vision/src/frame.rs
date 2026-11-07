use crate::error::VisionError;

/// Bytes per pixel in every frame this crate reads.
pub(crate) const CHANNELS: usize = 4;

/// Byte order of a pixel's colour channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pixels {
    Rgba,
    Bgra,
}

/// A rectangle of a frame, in pixels from its top left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// One captured frame: four bytes per pixel, rows top to bottom.
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    bytes: &'a [u8],
    width: u32,
    height: u32,
    pixels: Pixels,
}

impl<'a> Frame<'a> {
    /// Wraps captured bytes; `Err(VisionError::Frame)` when the buffer is shorter than
    /// `width × height × 4`.
    pub fn new(
        bytes: &'a [u8],
        width: u32,
        height: u32,
        pixels: Pixels,
    ) -> Result<Self, VisionError> {
        let needed = width as usize * height as usize * CHANNELS;
        if bytes.len() < needed {
            return Err(VisionError::Frame {
                width,
                height,
                needed,
                given: bytes.len(),
            });
        }
        Ok(Self {
            bytes,
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Red, green and blue of one pixel, whatever the byte order.
    pub(crate) fn rgb(&self, x: u32, y: u32) -> [u8; 3] {
        let at = (y as usize * self.width as usize + x as usize) * CHANNELS;
        let (one, two, three) = (self.bytes[at], self.bytes[at + 1], self.bytes[at + 2]);
        match self.pixels {
            Pixels::Rgba => [one, two, three],
            Pixels::Bgra => [three, two, one],
        }
    }

    /// Whether `area` lies wholly inside the frame and contains at least one pixel.
    pub(crate) fn holds(&self, area: Area) -> bool {
        area.width > 0
            && area.height > 0
            && area.x.saturating_add(area.width) <= self.width
            && area.y.saturating_add(area.height) <= self.height
    }
}
