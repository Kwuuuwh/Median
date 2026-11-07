use thiserror::Error;

/// Why a frame could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VisionError {
    /// The buffer is smaller than its stated frame size requires.
    #[error("a {width}x{height} frame needs {needed} bytes, got {given}")]
    Frame {
        width: u32,
        height: u32,
        needed: usize,
        given: usize,
    },

    /// The area asked for does not lie inside the frame.
    #[error("area {x},{y} {width}x{height} falls outside a {across}x{down} frame")]
    Area {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        across: u32,
        down: u32,
    },

    /// The model could not be loaded.
    #[error("the model could not be loaded: {0}")]
    Model(String),

    /// The model ran, but its output has an unexpected shape.
    #[error("the model answered in an unreadable shape: {0}")]
    Answer(String),
}
