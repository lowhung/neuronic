//! Sound feedback for graph events.
//!
//! Provides audio cues for message activity:
//! - Node firing: short high-pitched tick
//! - Critical health: low warning tone
//! - Pulse ring (heavy burst): resonant ping

use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use std::time::Duration;

/// Sound engine for audio feedback.
pub struct SoundEngine {
    _stream: OutputStream,
    enabled: bool,
    /// Volume from 0.0 to 1.0
    volume: f32,
}

impl SoundEngine {
    /// Create a new sound engine. Returns None if audio output is unavailable.
    pub fn new() -> Option<Self> {
        let stream = OutputStreamBuilder::open_default_stream().ok()?;
        Some(Self {
            _stream: stream,
            enabled: false, // Default to disabled
            volume: 0.3,
        })
    }

    /// Enable or disable sound.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if sound is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set volume (0.0 to 1.0).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Get current volume.
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Play a short tick sound for node activity.
    pub fn play_tick(&self) {
        if !self.enabled {
            return;
        }
        self.play_tone(880.0, Duration::from_millis(30), self.volume * 0.3);
    }

    /// Play a warning tone for critical health status.
    pub fn play_warning(&self) {
        if !self.enabled {
            return;
        }
        self.play_tone(220.0, Duration::from_millis(150), self.volume * 0.6);
    }

    /// Play a ping sound for heavy burst (pulse ring).
    pub fn play_ping(&self) {
        if !self.enabled {
            return;
        }
        self.play_tone(660.0, Duration::from_millis(80), self.volume * 0.5);
    }

    /// Play a sine wave tone at the given frequency, duration, and volume.
    fn play_tone(&self, freq: f32, duration: Duration, volume: f32) {
        let sink = Sink::connect_new(self._stream.mixer());
        let source = rodio::source::SineWave::new(freq)
            .take_duration(duration)
            .amplify(volume);
        sink.append(source);
        sink.detach();
    }
}
