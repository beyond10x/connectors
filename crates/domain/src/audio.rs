//! Protocol-neutral local-audio vocabulary.
//!
//! Nothing here names a synthesizer product, a voice, or an executable. The variants classify
//! **device stacks**, which is a fact about the machine an operation is placed on, not a fact about
//! any vendor. The closed `audio_v1` driver is the only code that knows how a stack is driven.

use serde::{Deserialize, Serialize};

/// The local audio sink family a deployment admitted, or that a driver probe resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioSink {
    /// PipeWire.
    PipeWire,
    /// PulseAudio.
    PulseAudio,
    /// ALSA.
    Alsa,
}

impl AudioSink {
    /// The sink candidates, in the exact order a probe considers them.
    ///
    /// The order is a deployment fact, not a preference a caller can express: a probe takes the
    /// first stack present and never retries a failed one through another.
    #[must_use]
    pub const fn candidates() -> [Self; 3] {
        [Self::PipeWire, Self::PulseAudio, Self::Alsa]
    }

    /// The stable token this sink is recorded as.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PipeWire => "pipe-wire",
            Self::PulseAudio => "pulse-audio",
            Self::Alsa => "alsa",
        }
    }
}

/// The PCM shape a local device is driven at, in either direction.
///
/// Signed 16-bit little-endian is the only encoding, so it is stated here rather than carried as a
/// field: every stack is told `s16` explicitly at open time, and a second encoding would be a
/// second negotiation rather than a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PcmFormat {
    /// Frames per second. Belongs to whatever produced or expects the audio, never to the device.
    pub sample_rate_hz: u32,
    /// Channel count. The first profile is mono.
    pub channels: u8,
}

impl PcmFormat {
    /// One channel at `sample_rate_hz`.
    #[must_use]
    pub const fn mono(sample_rate_hz: u32) -> Self {
        Self {
            sample_rate_hz,
            channels: 1,
        }
    }

    /// Bytes one sample occupies across every channel.
    #[must_use]
    pub const fn frame_bytes(self) -> usize {
        2 * self.channels as usize
    }
}

/// What a device turned out to be, recorded as evidence rather than assumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioDeviceDescription {
    /// The stack family — absent on a device that reaches no OS sound stack at all.
    ///
    /// A deployment with no sound card is the ordinary server case, and it says so here instead of
    /// naming a stack it does not have.
    pub stack: Option<AudioSink>,
    /// The exact executable the stack was found as, deliberately not canonicalized; or the device's
    /// own name where there is no executable.
    pub path: String,
}

/// A local device refused, and why.
#[derive(Debug, thiserror::Error)]
pub enum AudioDeviceError {
    /// This machine has no such device. Distinct from one that exists and failed.
    #[error("no local audio device is available: {reason}")]
    Unavailable { reason: String },
    /// The device existed and did not do what was asked.
    #[error("the local audio device failed: {reason}")]
    Failed { reason: String },
    /// The stream has already ended.
    #[error("the audio stream is closed")]
    Closed,
}

/// The write direction: PCM on its way to this machine's speaker.
pub trait AudioPlayback: Send {
    /// Hand `pcm` to the device. Blocks while the device applies backpressure, which is what keeps
    /// a producer from running ahead of real time.
    ///
    /// # Errors
    ///
    /// [`AudioDeviceError::Closed`] once the device has ended, [`AudioDeviceError::Failed`] on a
    /// device fault.
    fn write(&mut self, pcm: &[u8]) -> Result<(), AudioDeviceError>;

    /// Flush, then wait for the device to drain.
    ///
    /// Consuming, because a drained stream cannot be written to again — the type says so rather
    /// than a runtime check.
    ///
    /// # Errors
    ///
    /// [`AudioDeviceError::Failed`] if the device did not drain cleanly.
    fn finish(self: Box<Self>) -> Result<(), AudioDeviceError>;
}

/// The read direction: PCM arriving from this machine's microphone.
pub trait AudioCapture: Send {
    /// Fill `into`, returning how many bytes were read.
    ///
    /// `Ok(0)` means the device ended, and is not an error.
    ///
    /// # Errors
    ///
    /// [`AudioDeviceError::Failed`] on a device fault.
    fn read(&mut self, into: &mut [u8]) -> Result<usize, AudioDeviceError>;

    /// Stop capturing and release the device.
    ///
    /// # Errors
    ///
    /// [`AudioDeviceError::Failed`] if the device did not stop cleanly.
    fn stop(self: Box<Self>) -> Result<(), AudioDeviceError>;
}

/// A local sound device, in both directions.
///
/// # Why this is a port, and why it is synchronous
///
/// It is a **port** because the same code must run on a workstation with a sound card and on a
/// server with none. A deployment binds the device it has; nothing above this trait branches on
/// which one, and no capability is lost by having neither speaker nor microphone — the calls
/// succeed and the audio goes nowhere, which is the honest headless behaviour.
///
/// It is **synchronous** because every implementation is a pipe to a subprocess or a memory buffer,
/// and both block. An async caller runs it on a blocking task rather than making an entire
/// synthesis path async for the sake of a file descriptor.
pub trait AudioDevice: Send + Sync {
    /// What this device is, for evidence.
    fn description(&self) -> AudioDeviceDescription;

    /// Open the write direction at `format`.
    ///
    /// # Errors
    ///
    /// [`AudioDeviceError::Unavailable`] when the machine has no such device.
    fn open_playback(&self, format: PcmFormat) -> Result<Box<dyn AudioPlayback>, AudioDeviceError>;

    /// Open the read direction at `format`.
    ///
    /// # Errors
    ///
    /// [`AudioDeviceError::Unavailable`] when the machine has no such device.
    fn open_capture(&self, format: PcmFormat) -> Result<Box<dyn AudioCapture>, AudioDeviceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mono_format_occupies_two_bytes_a_sample() {
        // The bound every buffer size in the drivers is computed from. Getting it wrong renders at
        // the wrong rate rather than failing, which is why it is pinned rather than inlined.
        assert_eq!(PcmFormat::mono(8_000).frame_bytes(), 2);
        assert_eq!(PcmFormat::mono(8_000).channels, 1);
    }

    #[test]
    fn a_device_with_no_stack_says_so_rather_than_naming_one() {
        let headless = AudioDeviceDescription {
            stack: None,
            path: "null".to_owned(),
        };
        let encoded = serde_json::to_value(&headless).expect("a description encodes");
        assert!(
            encoded.get("stack").is_some_and(serde_json::Value::is_null),
            "a server with no sound card records an absent stack, not a fabricated one"
        );
    }

    #[test]
    fn every_candidate_is_distinct_and_ordered() {
        let candidates = AudioSink::candidates();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0], AudioSink::PipeWire);
        for pair in candidates.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }
    }

    #[test]
    fn the_recorded_token_round_trips_through_serde() {
        for sink in AudioSink::candidates() {
            let encoded = serde_json::to_string(&sink).expect("sink encodes");
            assert_eq!(encoded, format!("\"{}\"", sink.as_str()));
        }
    }
}
