use median_vision::Area;

/// Reference resolution at which the game's interface areas are measured.
const MEASURED: (f32, f32) = (1920.0, 1080.0);

/// Maps a 1920×1080 rectangle `[x, y, width, height]` to a frame of this size, scaled to fit
/// and centred.
pub fn placed(width: u32, height: u32, rect: [f32; 4]) -> Area {
    let scale = (width as f32 / MEASURED.0).min(height as f32 / MEASURED.1);
    let left = (width as f32 - MEASURED.0 * scale) / 2.0;
    let top = (height as f32 - MEASURED.1 * scale) / 2.0;
    let [x, y, wide, tall] = rect;
    Area {
        x: (left + x * scale).round().max(0.0) as u32,
        y: (top + y * scale).round().max(0.0) as u32,
        width: (wide * scale).round() as u32,
        height: (tall * scale).round() as u32,
    }
}
