//! The device a host with no sound card binds.
//!
//! # Why it is paced, and not a no-op
//!
//! A discarding device that returns immediately is the obvious implementation and the wrong one.
//! Playback and capture are the only backpressure in the whole audio path: a synthesizer writes
//! until the speaker accepts more, and a call reads until the microphone produces more. Remove the
//! wait and a server deployment does not "run without audio" — it burns a core spinning, and every
//! timing-dependent behaviour above it (barge-in, duration accounting, a call's own clock) silently
//! stops matching the workstation it was developed on.
//!
//! So this device does exactly what a real one does, minus the sound: it takes as long as the audio
//! it was handed represents, and produces silence at the rate silence would arrive.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use domain::audio::{
    AudioCapture, AudioDevice, AudioDeviceDescription, AudioDeviceError, AudioPlayback, PcmFormat,
};

/// A device with no OS sound stack behind it.
#[derive(Debug, Default, Clone)]
pub struct NullAudioDevice {
    /// Bytes this device has been handed, so a headless deployment can still account for what it
    /// would have played. Shared with every stream it opens.
    written: Arc<AtomicU64>,
}

impl NullAudioDevice {
    /// A device that plays and captures nothing, at the speed of real audio.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Total bytes handed to playback across every stream this device opened.
    #[must_use]
    pub fn written_bytes(&self) -> u64 {
        self.written.load(Ordering::Relaxed)
    }
}

/// How long `bytes` of `format` occupies.
///
/// Saturating rather than panicking on a zero rate: a malformed format must not take a deployment
/// down, and zero-length silence is the harmless reading.
fn duration_of(bytes: usize, format: PcmFormat) -> Duration {
    let bytes_per_second = format.sample_rate_hz as u64 * format.frame_bytes() as u64;
    if bytes_per_second == 0 {
        return Duration::ZERO;
    }
    Duration::from_nanos(bytes as u64 * 1_000_000_000 / bytes_per_second)
}

impl AudioDevice for NullAudioDevice {
    fn description(&self) -> AudioDeviceDescription {
        AudioDeviceDescription {
            stack: None,
            path: "null".to_owned(),
        }
    }

    fn open_playback(&self, format: PcmFormat) -> Result<Box<dyn AudioPlayback>, AudioDeviceError> {
        Ok(Box::new(NullPlayback {
            format,
            written: Arc::clone(&self.written),
        }))
    }

    fn open_capture(&self, format: PcmFormat) -> Result<Box<dyn AudioCapture>, AudioDeviceError> {
        Ok(Box::new(NullCapture { format }))
    }
}

struct NullPlayback {
    format: PcmFormat,
    written: Arc<AtomicU64>,
}

impl AudioPlayback for NullPlayback {
    fn write(&mut self, pcm: &[u8]) -> Result<(), AudioDeviceError> {
        self.written.fetch_add(pcm.len() as u64, Ordering::Relaxed);
        std::thread::sleep(duration_of(pcm.len(), self.format));
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), AudioDeviceError> {
        Ok(())
    }
}

struct NullCapture {
    format: PcmFormat,
}

impl AudioCapture for NullCapture {
    fn read(&mut self, into: &mut [u8]) -> Result<usize, AudioDeviceError> {
        // Silence, at the rate silence arrives. Returning `Ok(0)` instead would mean "the device
        // ended", which is a different fact and would tear down a call that is perfectly healthy.
        std::thread::sleep(duration_of(into.len(), self.format));
        into.fill(0);
        Ok(into.len())
    }

    fn stop(self: Box<Self>) -> Result<(), AudioDeviceError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// 8 kHz mono, 20 ms — one G.711 packet, the unit a call actually moves.
    const FRAME: usize = 320;

    #[test]
    fn a_headless_host_records_no_stack_rather_than_a_fabricated_one() {
        let description = NullAudioDevice::new().description();
        assert_eq!(description.stack, None);
        assert_eq!(description.path, "null");
    }

    #[test]
    fn playback_takes_as_long_as_the_audio_it_was_handed() {
        // The property that stops a headless deployment spinning. Ten frames is 200 ms of audio;
        // the bound is generous below and absent above, because a slow machine is not a defect.
        let device = NullAudioDevice::new();
        let mut playback = device
            .open_playback(PcmFormat::mono(8_000))
            .expect("a null device always opens");
        let started = Instant::now();
        for _ in 0..10 {
            playback.write(&[0; FRAME]).expect("silence is accepted");
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_millis(150),
            "playback returned in {elapsed:?}, so it is not pacing"
        );
        assert_eq!(device.written_bytes(), (FRAME * 10) as u64);
        playback.finish().expect("a null stream drains");
    }

    #[test]
    fn capture_yields_silence_and_never_reports_the_device_ended() {
        let device = NullAudioDevice::new();
        let mut capture = device
            .open_capture(PcmFormat::mono(8_000))
            .expect("a null device always opens");
        let mut frame = [0xFF_u8; FRAME];
        let read = capture.read(&mut frame).expect("silence is produced");
        assert_eq!(read, FRAME, "a full buffer, not an end-of-stream");
        assert!(
            frame.iter().all(|byte| *byte == 0),
            "capture must overwrite the caller's buffer with silence"
        );
        capture.stop().expect("a null stream stops");
    }

    #[test]
    fn a_zero_rate_is_instant_rather_than_a_panic() {
        // Reached only through a malformed format, and a division by zero there would take down a
        // deployment for audio it was never going to play.
        assert_eq!(
            duration_of(
                FRAME,
                PcmFormat {
                    sample_rate_hz: 0,
                    channels: 1
                }
            ),
            Duration::ZERO
        );
    }
}
