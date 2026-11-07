use image::RgbImage;
use image::imageops::{self, FilterType};
use tract_onnx::prelude::*;

use crate::error::VisionError;
use crate::frame::{Area, Frame};

/// Height in pixels every line is scaled to before recognition.
pub(crate) const HEIGHT: u32 = 48;

/// Fixed width of the model's input, in pixels. A line keeps its aspect ratio and the rest of
/// the row is padded with zeros; only a line wider than this is squeezed to fit.
pub(crate) const WIDTH: u32 = 512;

/// Crops `area` from `frame` into the model's input tensor: three planes of blue, green and red,
/// each value scaled to `[-1, 1]`; `Err(VisionError::Area)` when `area` is not inside the frame.
pub(crate) fn line(frame: &Frame<'_>, area: Area) -> Result<Tensor, VisionError> {
    if !frame.holds(area) {
        return Err(VisionError::Area {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
            across: frame.width(),
            down: frame.height(),
        });
    }

    let mut cut = RgbImage::new(area.width, area.height);
    for y in 0..area.height {
        for x in 0..area.width {
            cut.put_pixel(x, y, image::Rgb(frame.rgb(area.x + x, area.y + y)));
        }
    }

    let scaled = u64::from(area.width)
        .saturating_mul(u64::from(HEIGHT))
        .div_ceil(u64::from(area.height))
        .clamp(1, u64::from(WIDTH)) as u32;
    let fitted = imageops::resize(&cut, scaled, HEIGHT, FilterType::CatmullRom);

    let laid = tract_ndarray::Array4::from_shape_fn(
        (1, 3, HEIGHT as usize, WIDTH as usize),
        |(_, channel, y, x)| match (x as u32) < scaled {
            true => {
                let colour = fitted.get_pixel(x as u32, y as u32).0;
                (f32::from(colour[2 - channel]) / 255.0 - 0.5) / 0.5
            }
            false => 0.0,
        },
    );
    Ok(laid.into())
}
