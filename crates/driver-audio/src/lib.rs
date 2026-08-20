#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! The closed built-in `audio_v1` protocol driver: **this machine's sound device, and nothing
//! else**.
//!
//! The only Connectors crate allowed to reach the local sound stack and to spawn the processes that
//! feed it. It knows about PipeWire, PulseAudio and ALSA. It knows nothing about speech: no voice,
//! no synthesizer, no transcript, no recognizer.
//!
//! # Why the driver word is `audio_v1` and speech lives elsewhere
//!
//! The `protocol_driver` axis answers *which closed implementation speaks to the external system*.
//! The external system here is the machine's sound stack. Speech synthesis and speech recognition
//! are **transformations that produce or consume PCM** — they are not protocols, they reach no
//! external system of their own, and they belong in `driver-speech`, which depends on this crate
//! through [`domain::audio::AudioDevice`] and never touches a device directly.
//!
//! That separation is what keeps the vocabulary honest as the driver grows:
//!
//! | Capability | Lives in |
//! |---|---|
//! | playback to a speaker | here, [`process`] |
//! | capture from a microphone | here, [`process`] |
//! | a host with neither | here, [`null`] |
//! | speech synthesis (TTS) | `driver-speech` |
//! | speech recognition (STT) | `driver-speech` |
//! | carrying call media to a device | a binding over [`domain::audio::AudioDevice`] |
//!
//! Had synthesis stayed in this crate, every one of the last three would have had to reach through
//! it to get at a file descriptor, and "the only crate that touches the sound stack" would have
//! stopped being true the first time recognition shipped.
//!
//! # The port is the deployment seam
//!
//! [`ProcessAudioDevice`] is what a workstation binds: real output, real input, resolved from the
//! stack this machine actually has. [`NullAudioDevice`] is what a server binds: the same contract,
//! paced to real time, with no sound. **Nothing above the port branches on which one is bound** —
//! that is the whole point, and it is why a headless deployment is a configuration rather than a
//! disabled code path.
//!
//! # Two facts about the local audio stack are load-bearing, and both fail silently
//!
//! 1. `pw-play`, `paplay` and their recording counterparts are **multi-call binaries** that select
//!    their behavior from `argv[0]`. A resolved path is never canonicalized; executing the
//!    `pw-cat`/`pacat` target instead would change what the program does while still exiting zero.
//! 2. `parecord` and `arecord` default to writing a **WAV header**. Read as samples, those 44 bytes
//!    are a click followed by permanently misaligned frames — noise, never an error.
//!
//! Both are pinned by tests in [`device`].

pub mod device;
pub mod null;
pub mod process;

pub use crate::device::{
    capture_arguments, capture_executable, discover_executable, executable_at, resolve_capture,
    resolve_sink, sink_arguments, sink_executable,
};
pub use crate::null::NullAudioDevice;
pub use crate::process::ProcessAudioDevice;

use std::sync::Arc;

use domain::audio::{AudioDevice, AudioSink};

/// Bind the best device this host has: the real stack when there is one, silence when there is not.
///
/// This is the one place the fallback is allowed to exist, and it is a *deployment* fallback rather
/// than a retry: a host either has a sound stack or does not, and that answer does not change
/// between calls. Nothing downstream learns which arm was taken except through
/// [`AudioDevice::description`], which records it as evidence.
#[must_use]
pub fn local_device(explicit: Option<AudioSink>) -> Arc<dyn AudioDevice> {
    ProcessAudioDevice::resolve(explicit).map_or_else(
        |_| Arc::new(NullAudioDevice::new()) as Arc<dyn AudioDevice>,
        |device| Arc::new(device) as Arc<dyn AudioDevice>,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_is_always_bound_and_always_says_which_it_is() {
        // The property the deployment seam rests on: composition never fails for want of a sound
        // card, and the result is never ambiguous about what it got.
        let device = local_device(None);
        let description = device.description();
        if description.stack.is_none() {
            assert_eq!(description.path, "null");
        } else {
            assert!(!description.path.is_empty());
        }
    }
}
