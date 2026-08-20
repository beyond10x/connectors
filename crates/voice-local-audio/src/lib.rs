#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Binding an admitted media session to this machine's speaker and microphone.
//!
//! # What this is for
//!
//! `sip.dial` places a call and hands back a [`TelephonySession`]. Until something reads that
//! session the call is connected and inaudible: RTP flows to nobody. This is the something.
//!
//! # Why it names no protocol
//!
//! SIP and RTVBP are different protocols, and neither is a stage of the other. What they have in
//! common is that **both produce a generalized media session** — a [`TelephonySession`] — and that
//! is the only thing this crate knows about. It cannot tell which protocol established the call it
//! is carrying, and it does not have a code path per protocol. A binding that named SIP would have
//! to be written a second time the day the other one needs a speaker.
//!
//! # No transcoding happens here, and that is a property of the boundary
//!
//! A caller might expect G.711 at this seam, because that is what the trunk negotiates. It is not:
//! [`MediaDescriptor::validate`] admits exactly one profile — `pcm_s16le`, 8 kHz, mono, 20 ms,
//! 320 bytes — so the driver has already decoded to PCM by the time a frame crosses
//! [`TelephonySession`]. Those bytes are byte-identical to what the device wants, and the profile
//! is asserted at bind time rather than assumed.
//!
//! # Why two threads and two tasks
//!
//! The session is async; the device is not, because every device is a pipe to a subprocess. So each
//! direction is an async task paired with a blocking one, joined by a bounded channel:
//!
//! ```text
//! far end ──▶ read_input() ──▶ [queue] ──▶ playback.write()  ──▶ speaker
//! far end ◀── write_output() ◀─ [queue] ◀── capture.read()   ◀── microphone
//! ```
//!
//! **The queues drop rather than grow.** A speaker that falls behind must not accumulate a backlog:
//! in a live call, audio that arrives late is worthless, and buffering it converts a brief stall
//! into permanent drift where every later word is played at the wrong time. Dropping keeps the
//! conversation aligned with the clock.
//!
//! # Teardown
//!
//! Stopping aborts the two async tasks only. That is deliberate and sufficient: aborting drops the
//! channel ends the async side owns, which closes the queues, which is what the two blocking
//! threads observe. The microphone thread is parked in a blocking read at the time, so it notices
//! one frame later — 20 ms — and then kills its recorder. Trying to abort a blocking task instead
//! would do nothing at all, and the recorder would outlive the call.

use std::sync::Arc;

use domain::audio::{AudioCapture, AudioDevice, AudioDeviceError, AudioPlayback, PcmFormat};
use domain::voice::{AudioFrame, MediaDescriptor, TelephonySession};
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio::task::JoinHandle;

/// One second of 20 ms frames.
///
/// Sized to absorb a scheduling hiccup and nothing more. Larger would trade a dropped frame for
/// creeping latency, which is the worse failure: a listener notices a growing delay long before
/// they notice one lost packet.
const QUEUE_FRAMES: usize = 50;

/// The binding could not be established.
#[derive(Debug, thiserror::Error)]
pub enum BindError {
    /// The session negotiated media this device profile does not describe.
    #[error("the session media profile is not one a local device can carry")]
    UnsupportedMedia,
    /// The device refused to open one of the two directions.
    #[error(transparent)]
    Device(#[from] AudioDeviceError),
}

/// A live binding between one media session and one local device.
///
/// Dropping this does **not** stop it — the tasks are detached and the call keeps its audio. Call
/// [`LocalAudioBinding::stop`] to end it.
pub struct LocalAudioBinding {
    inbound: JoinHandle<()>,
    outbound: JoinHandle<()>,
}

impl LocalAudioBinding {
    /// Stop carrying audio, and release the device.
    ///
    /// Aborting the async halves closes both queues; the blocking halves observe that and shut the
    /// device down themselves. See the module docs for why this is not done directly.
    pub fn stop(self) {
        self.inbound.abort();
        self.outbound.abort();
    }
}

/// Carry `session`'s audio to and from `device`.
///
/// # Errors
///
/// [`BindError::UnsupportedMedia`] when the session's profile is not the one local devices are
/// driven at, and [`BindError::Device`] when either direction cannot be opened — a host with a
/// speaker and no microphone refuses here rather than half-binding.
pub fn bind(
    session: Arc<dyn TelephonySession>,
    device: &dyn AudioDevice,
) -> Result<LocalAudioBinding, BindError> {
    let media = session.descriptor().media.clone();
    let format = local_format(&media)?;
    // Both directions are opened before either task starts, so a host that can play but not record
    // fails as a refusal rather than as a call with one-way audio nobody was told about.
    let playback = device.open_playback(format)?;
    let capture = device.open_capture(format)?;

    let (to_speaker, speaker_queue) = mpsc::channel::<Vec<u8>>(QUEUE_FRAMES);
    let (to_far_end, mut far_end_queue) = mpsc::channel::<Vec<u8>>(QUEUE_FRAMES);

    let frame_bytes = media.frame_bytes;
    let reader = Arc::clone(&session);

    let inbound = tokio::spawn(async move {
        // The loop ends on `Ok(None)` — the media ending — and on an error, the session failing.
        // Both end the binding, and neither is inferred from the other.
        while let Ok(Some(frame)) = reader.read_input().await {
            match to_speaker.try_send(frame.bytes) {
                // A full queue means the speaker is behind. The frame is dropped; see the module
                // docs for why that beats buffering it.
                Ok(()) | Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Closed(_)) => break,
            }
        }
    });

    let outbound = tokio::spawn(async move {
        // The session refuses a non-monotonic sequence, so it starts at one and only ever rises.
        let mut sequence: u64 = 0;
        while let Some(bytes) = far_end_queue.recv().await {
            sequence = sequence.saturating_add(1);
            let Ok(frame) = AudioFrame::new(sequence, bytes, &media) else {
                break;
            };
            if session.write_output(frame).await.is_err() {
                break;
            }
        }
    });

    spawn_speaker(playback, speaker_queue);
    spawn_microphone(capture, to_far_end, frame_bytes);

    Ok(LocalAudioBinding { inbound, outbound })
}

/// The device profile this media descriptor is carried at.
fn local_format(media: &MediaDescriptor) -> Result<PcmFormat, BindError> {
    // The one admitted profile. Asserting it here means a future second profile arrives as a
    // refusal to bind rather than as audio played at the wrong rate.
    media.validate().map_err(|_| BindError::UnsupportedMedia)?;
    let channels = u8::try_from(media.channels).map_err(|_| BindError::UnsupportedMedia)?;
    Ok(PcmFormat {
        sample_rate_hz: media.sample_rate_hz,
        channels,
    })
}

/// Drain the queue into the speaker until the queue closes.
fn spawn_speaker(mut playback: Box<dyn AudioPlayback>, mut queue: mpsc::Receiver<Vec<u8>>) {
    tokio::task::spawn_blocking(move || {
        while let Some(bytes) = queue.blocking_recv() {
            if playback.write(&bytes).is_err() {
                break;
            }
        }
        // Drains what the device already accepted; the tail of a call is still the call.
        let _ = playback.finish();
    });
}

/// Fill whole frames from the microphone until the queue closes or the device ends.
fn spawn_microphone(
    mut capture: Box<dyn AudioCapture>,
    queue: mpsc::Sender<Vec<u8>>,
    frame_bytes: usize,
) {
    tokio::task::spawn_blocking(move || {
        let mut frame = vec![0_u8; frame_bytes];
        // Ends on `Ok(false)` — the device stopped mid-frame — and on a device fault. A short
        // final read is a partial frame, and a partial frame is not a frame: sent as one it is
        // rendered as a click.
        while let Ok(true) = fill(capture.as_mut(), &mut frame) {
            match queue.try_send(frame.clone()) {
                Ok(()) | Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Closed(_)) => break,
            }
        }
        let _ = capture.stop();
    });
}

/// Read exactly `frame.len()` bytes, or report that the device ended.
///
/// A device read is free to return less than asked for, so a single `read` is not a frame. Treating
/// one as a frame is how a capture path ends up permanently one packet out of alignment.
fn fill(capture: &mut dyn AudioCapture, frame: &mut [u8]) -> Result<bool, AudioDeviceError> {
    let mut filled = 0;
    while filled < frame.len() {
        let read = capture.read(&mut frame[filled..])?;
        if read == 0 {
            return Ok(false);
        }
        filled += read;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::voice::{
        ChannelSignal, ContextTrust, ParticipantContext, TerminationReason, VoiceError, VoiceRef,
        VoiceSessionDescriptor,
    };
    use std::sync::Mutex;
    use std::time::Duration;

    /// A session that emits a fixed number of frames and then ends, recording what was sent back.
    ///
    /// No SIP, no RTVBP, no socket: the binding is supposed to be unable to tell, and this is what
    /// asserts that it is.
    struct FakeSession {
        descriptor: VoiceSessionDescriptor,
        inbound: Mutex<u32>,
        written: Arc<Mutex<Vec<AudioFrame>>>,
    }

    impl FakeSession {
        fn new(frames: u32) -> (Arc<Self>, Arc<Mutex<Vec<AudioFrame>>>) {
            let written = Arc::new(Mutex::new(Vec::new()));
            let media = MediaDescriptor::pcm_s16le_8khz_mono_20ms();
            let session = Arc::new(Self {
                descriptor: VoiceSessionDescriptor {
                    call: VoiceRef::new("call-1").expect("a reference"),
                    session: VoiceRef::new("session-1").expect("a reference"),
                    channel: VoiceRef::new("channel-1").expect("a reference"),
                    participant: ParticipantContext {
                        reference: VoiceRef::new("participant-1").expect("a reference"),
                        trust: ContextTrust::Untrusted,
                        display: None,
                    },
                    media,
                },
                inbound: Mutex::new(frames),
                written: Arc::clone(&written),
            });
            (session, written)
        }
    }

    #[async_trait]
    impl TelephonySession for FakeSession {
        fn descriptor(&self) -> &VoiceSessionDescriptor {
            &self.descriptor
        }

        async fn read_input(&self) -> Result<Option<AudioFrame>, VoiceError> {
            let mut remaining = self.inbound.lock().expect("a lock");
            if *remaining == 0 {
                return Ok(None);
            }
            *remaining -= 1;
            let sequence = u64::from(*remaining) + 1;
            let bytes = vec![0_u8; self.descriptor.media.frame_bytes];
            Ok(Some(AudioFrame::new(
                sequence,
                bytes,
                &self.descriptor.media,
            )?))
        }

        async fn write_output(&self, frame: AudioFrame) -> Result<(), VoiceError> {
            self.written.lock().expect("a lock").push(frame);
            Ok(())
        }

        async fn next_signal(&self) -> Result<Option<ChannelSignal>, VoiceError> {
            Ok(None)
        }

        async fn wait_terminated(&self) -> Result<TerminationReason, VoiceError> {
            Ok(TerminationReason::Completed)
        }

        async fn interrupt_output(&self) -> Result<(), VoiceError> {
            Ok(())
        }

        async fn terminate(&self, _reason: TerminationReason) -> Result<(), VoiceError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn the_far_end_reaches_the_speaker() {
        // The defect this crate exists to fix: before it, the call connected and every one of these
        // frames went to a session nobody read.
        let device = driver_audio::NullAudioDevice::new();
        let (session, _) = FakeSession::new(5);
        let binding = bind(session, &device).expect("the profile binds");
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert_eq!(
            device.written_bytes(),
            5 * 320,
            "every inbound frame must reach the device"
        );
        binding.stop();
    }

    #[tokio::test]
    async fn the_microphone_reaches_the_far_end_with_a_rising_sequence() {
        let device = driver_audio::NullAudioDevice::new();
        let (session, written) = FakeSession::new(0);
        let binding = bind(session, &device).expect("the profile binds");
        tokio::time::sleep(Duration::from_millis(300)).await;
        binding.stop();

        let frames = written.lock().expect("a lock");
        assert!(
            !frames.is_empty(),
            "silence from the microphone is still audio the far end must receive"
        );
        // The session refuses a non-monotonic sequence, so a regression here is a dropped call
        // rather than a cosmetic fault.
        for pair in frames.windows(2) {
            assert!(
                pair[1].sequence > pair[0].sequence,
                "outbound sequence must rise: {} then {}",
                pair[0].sequence,
                pair[1].sequence
            );
        }
        assert_eq!(frames[0].sequence, 1, "the first frame is sequence one");
        assert!(
            frames.iter().all(|frame| frame.bytes.len() == 320),
            "only whole frames are sent"
        );
    }

    #[test]
    fn a_profile_the_device_cannot_be_driven_at_is_refused_rather_than_resampled() {
        let wrong = MediaDescriptor {
            sample_format: "pcm_s16le".to_owned(),
            sample_rate_hz: 16_000,
            channels: 1,
            packet_time_ms: 20,
            frame_bytes: 640,
        };
        assert!(matches!(
            local_format(&wrong),
            Err(BindError::UnsupportedMedia)
        ));
        let admitted = MediaDescriptor::pcm_s16le_8khz_mono_20ms();
        let format = local_format(&admitted).expect("the admitted profile binds");
        assert_eq!(format, PcmFormat::mono(8_000));
    }

    /// Hear the duplex path on the real device, with no trunk and no call.
    ///
    /// A call needs a SIP peer; the audio path does not. This records from the microphone and plays
    /// it straight back through the speaker over the same `AudioDevice` a call uses, at the same
    /// 8 kHz mono profile, so a person can confirm both directions work before blaming telephony.
    ///
    /// It is `#[ignore]`d because it uses the microphone and makes noise. Run it deliberately:
    ///
    /// ```text
    /// cargo test --manifest-path crates/voice-local-audio/Cargo.toml \
    ///   -- --ignored --nocapture loopback
    /// ```
    #[test]
    #[ignore = "uses the real microphone and speaker"]
    fn loopback_is_audible_on_the_real_device() {
        let device = driver_audio::local_device(None);
        assert!(
            device.description().stack.is_some(),
            "this check needs a real sound stack; the null device would prove nothing"
        );
        let format = PcmFormat::mono(8_000);
        let mut capture = device.open_capture(format).expect("a microphone");
        let mut playback = device.open_playback(format).expect("a speaker");

        // Four seconds of 20 ms frames, echoed one frame behind.
        let mut frame = vec![0_u8; 320];
        for _ in 0..200 {
            if !fill(capture.as_mut(), &mut frame).expect("the microphone reads") {
                break;
            }
            playback.write(&frame).expect("the speaker accepts");
        }
        capture.stop().expect("the microphone releases");
        playback.finish().expect("the speaker drains");
    }

    #[test]
    fn a_partial_read_is_never_sent_as_a_frame() {
        /// Yields one byte at a time, which is what a real device is entitled to do.
        struct Dribble {
            remaining: usize,
        }
        impl AudioCapture for Dribble {
            fn read(&mut self, into: &mut [u8]) -> Result<usize, AudioDeviceError> {
                if self.remaining == 0 {
                    return Ok(0);
                }
                self.remaining -= 1;
                into[0] = 7;
                Ok(1)
            }
            fn stop(self: Box<Self>) -> Result<(), AudioDeviceError> {
                Ok(())
            }
        }

        let mut frame = vec![0_u8; 4];
        let mut full = Dribble { remaining: 4 };
        assert!(fill(&mut full, &mut frame).expect("a full frame reads"));
        assert_eq!(frame, vec![7, 7, 7, 7]);

        // Three bytes into a four-byte frame is not three quarters of a frame, it is no frame.
        let mut short = Dribble { remaining: 3 };
        let mut frame = vec![0_u8; 4];
        assert!(!fill(&mut short, &mut frame).expect("a short read is not an error"));
    }
}
