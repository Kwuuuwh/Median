use tracing::warn;
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_SYNC};

/// Chime WAV embedded in the binary.
const CHIME: &[u8] = include_bytes!("../assets/whisper.wav");

/// Size in bytes of the canonical WAV header; a shorter file cannot hold samples.
const HEADER: usize = 44;

/// Plays the chime at `volume` percent (0 is silent, above 100 counts as 100) on its own
/// thread, so the caller does not block.
pub fn chime(volume: u8) {
    if volume == 0 {
        return;
    }
    let samples = quieter(CHIME.to_vec(), volume).unwrap_or_else(|| {
        warn!("chime not 16-bit PCM, played at its own volume");
        CHIME.to_vec()
    });
    // SAFETY: the thread owns `samples` until PlaySoundW returns, and SND_SYNC makes it return
    // only after playback ends, so the buffer outlives the sound.
    std::thread::spawn(move || unsafe {
        PlaySoundW(
            samples.as_ptr().cast(),
            std::ptr::null_mut(),
            SND_MEMORY | SND_SYNC,
        );
        drop(samples);
    });
}

/// Returns the WAV with every sample scaled to `volume` percent (capped at 100); `None` when
/// the file is not 16-bit PCM, the only format this scaling supports.
fn quieter(mut wav: Vec<u8>, volume: u8) -> Option<Vec<u8>> {
    if wav.len() <= HEADER || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }
    let (format, bits, data) = shape(&wav)?;
    if format != 1 || bits != 16 {
        return None;
    }

    let scale = f32::from(volume.min(100)) / 100.0;
    for sample in wav[data.0..data.1].chunks_exact_mut(2) {
        let value = i16::from_le_bytes([sample[0], sample[1]]);
        let quiet = (f32::from(value) * scale) as i16;
        sample.copy_from_slice(&quiet.to_le_bytes());
    }
    Some(wav)
}

/// Returns the format tag, bits per sample and byte range of the `data` chunk; `None` when the
/// file has no `data` chunk.
fn shape(wav: &[u8]) -> Option<(u16, u16, (usize, usize))> {
    let mut at = 12;
    let (mut format, mut bits) = (0, 0);

    while at + 8 <= wav.len() {
        let name = &wav[at..at + 4];
        let size = u32::from_le_bytes(wav[at + 4..at + 8].try_into().ok()?) as usize;
        let body = at + 8;
        match name {
            b"fmt " if body + 16 <= wav.len() => {
                format = u16::from_le_bytes(wav[body..body + 2].try_into().ok()?);
                bits = u16::from_le_bytes(wav[body + 14..body + 16].try_into().ok()?);
            }
            b"data" => return Some((format, bits, (body, (body + size).min(wav.len())))),
            _ => {}
        }
        at = body + size + (size & 1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 16-bit mono PCM WAV that holds the single sample `sample`.
    fn wav(sample: i16) -> Vec<u8> {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&38u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&44_100u32.to_le_bytes());
        wav.extend_from_slice(&88_200u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&2u32.to_le_bytes());
        wav.extend_from_slice(&sample.to_le_bytes());
        wav
    }

    fn sample_of(wav: &[u8]) -> i16 {
        let (_, _, (from, _)) = shape(wav).expect("the shape reads");
        i16::from_le_bytes(wav[from..from + 2].try_into().expect("two bytes"))
    }

    #[test]
    fn a_quarter_of_the_volume_is_a_quarter_of_the_sample() {
        let quiet = quieter(wav(10_000), 25).expect("it scales");

        assert_eq!(sample_of(&quiet), 2_500);
    }

    #[test]
    fn full_volume_leaves_the_samples_alone() {
        let same = quieter(wav(-8_000), 100).expect("it scales");

        assert_eq!(sample_of(&same), -8_000);
    }

    #[test]
    fn a_volume_over_the_top_is_taken_as_the_top() {
        let same = quieter(wav(1_000), 250).expect("it scales");

        assert_eq!(sample_of(&same), 1_000);
    }

    #[test]
    fn the_built_in_chime_can_be_made_quieter() {
        assert!(quieter(CHIME.to_vec(), 50).is_some());
    }

    #[test]
    fn a_file_that_is_not_a_wav_is_left_to_the_player() {
        assert!(quieter(b"not a wav at all, not even close to it".to_vec(), 50).is_none());
    }
}
