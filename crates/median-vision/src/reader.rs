use std::sync::Arc;

use tract_onnx::prelude::*;

use crate::decode::{Alphabet, Reading};
use crate::error::VisionError;
use crate::frame::{Area, Frame};
use crate::prepare::{self, HEIGHT, WIDTH};

/// The `eslav_PP-OCRv5_mobile_rec` recognition model.
const MODEL: &[u8] = include_bytes!("../assets/eslav.onnx");

/// Recognises lines of text in captured frames.
pub struct Reader {
    plan: Arc<TypedRunnableModel>,
    alphabet: Alphabet,
}

impl Reader {
    /// Loads the embedded recognition model; `Err(VisionError::Model)` when it does not load.
    pub fn new() -> Result<Self, VisionError> {
        let shape = f32::fact([1, 3, HEIGHT as usize, WIDTH as usize]);
        let plan = tract_onnx::onnx()
            .model_for_read(&mut &MODEL[..])
            .and_then(|read| read.with_input_fact(0, shape.into()))
            .and_then(|shaped| shaped.into_optimized())
            .and_then(|optimised| optimised.into_runnable())
            .map_err(|trouble| VisionError::Model(trouble.to_string()))?;

        Ok(Self {
            plan,
            alphabet: Alphabet::new(),
        })
    }

    /// Recognises the single line of text in `area` of `frame`; `Err` when `area` is not inside
    /// the frame or the model output has an unexpected shape.
    pub fn read(&self, frame: &Frame<'_>, area: Area) -> Result<Reading, VisionError> {
        let laid = prepare::line(frame, area)?;
        let answer = self
            .plan
            .run(tvec!(laid.into()))
            .map_err(|trouble| VisionError::Answer(trouble.to_string()))?;
        let plain = answer[0]
            .try_as_plain()
            .map_err(|trouble| VisionError::Answer(trouble.to_string()))?;
        let scores = plain
            .to_array_view::<f32>()
            .map_err(|trouble| VisionError::Answer(trouble.to_string()))?;

        let shape = scores.shape();
        if shape.len() != 3 || shape[0] != 1 || shape[2] != self.alphabet.classes() {
            return Err(VisionError::Answer(format!("{shape:?}")));
        }

        let steps: Vec<(usize, f32)> = (0..shape[1])
            .map(|step| {
                (0..shape[2]).fold((0usize, f32::MIN), |best, class| {
                    let score = scores[[0, step, class]];
                    match score > best.1 {
                        true => (class, score),
                        false => best,
                    }
                })
            })
            .collect();

        Ok(self.alphabet.spell(&steps))
    }
}
