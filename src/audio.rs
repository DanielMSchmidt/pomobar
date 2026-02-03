//! Audio playback for timer completion sounds.

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to initialize audio output: {0}")]
    Stream(#[from] rodio::StreamError),
    #[error("Failed to play audio: {0}")]
    Play(#[from] rodio::PlayError),
    #[error("Failed to decode audio")]
    Decode,
}

pub struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

impl AudioPlayer {
    /// Creates a new audio player.
    pub fn new() -> Result<Self, AudioError> {
        let (stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream: stream,
            handle,
        })
    }

    /// Plays the completion chime sound.
    pub fn play_chime(&self) {
        // For now, use a simple system sound or embedded sound
        // The sound data would normally be embedded like:
        // let sound_data = include_bytes!("../resources/chime.mp3");

        // Generate a simple beep tone as fallback
        if let Err(e) = self.play_generated_tone() {
            eprintln!("Failed to play chime: {}", e);
        }
    }

    /// Plays a simple generated tone as a fallback.
    fn play_generated_tone(&self) -> Result<(), AudioError> {
        use rodio::source::{SineWave, Source};

        let sink = Sink::try_new(&self.handle)?;

        // Create a pleasant two-tone chime
        // First tone: 880 Hz (A5) for 150ms
        let tone1 = SineWave::new(880.0)
            .take_duration(std::time::Duration::from_millis(150))
            .amplify(0.3);

        // Short pause
        let silence = rodio::source::Zero::<f32>::new(1, 44100)
            .take_duration(std::time::Duration::from_millis(50));

        // Second tone: 1046.5 Hz (C6) for 200ms
        let tone2 = SineWave::new(1046.5)
            .take_duration(std::time::Duration::from_millis(200))
            .amplify(0.3);

        sink.append(tone1);
        sink.append(silence);
        sink.append(tone2);
        sink.detach(); // Play in background

        Ok(())
    }

    /// Plays a sound from raw bytes (MP3 or WAV).
    #[allow(dead_code)]
    fn play_from_bytes(&self, data: &'static [u8]) -> Result<(), AudioError> {
        let cursor = Cursor::new(data);
        let source = Decoder::new(cursor).map_err(|_| AudioError::Decode)?;
        let sink = Sink::try_new(&self.handle)?;
        sink.append(source);
        sink.detach();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_player_creation() {
        // This test may fail on systems without audio output
        // That's acceptable for CI environments
        let result = AudioPlayer::new();
        // Don't assert success, just ensure it doesn't panic
        match result {
            Ok(_) => println!("Audio player created successfully"),
            Err(e) => println!("Audio player creation failed (expected on CI): {}", e),
        }
    }
}
