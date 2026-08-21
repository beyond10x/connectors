//! Protocol-neutral voice semantics and the internal telephony port.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const MAX_REFERENCE_BYTES: usize = 128;
pub const MAX_SIGNAL_DIGITS: usize = 32;
pub const MAX_AUDIO_FRAME_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VoiceError {
    #[error("voice reference is empty or exceeds {MAX_REFERENCE_BYTES} bytes")]
    InvalidReference,
    #[error("media descriptor is unsupported")]
    UnsupportedMedia,
    #[error("audio frame exceeds its negotiated or absolute bound")]
    FrameTooLarge,
    #[error("channel signal is invalid")]
    InvalidSignal,
    #[error("this telephony binding cannot send channel signals")]
    SignalUnsupported,
    #[error("voice session is already terminated")]
    Terminated,
    #[error("telephony endpoint failed: {0}")]
    Endpoint(String),
}

/// Opaque reference with no protocol or tenant meaning.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VoiceRef(String);

impl VoiceRef {
    pub fn new(value: impl Into<String>) -> Result<Self, VoiceError> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_REFERENCE_BYTES {
            return Err(VoiceError::InvalidReference);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Trust classification is explicit; remote channel context never becomes authority by inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTrust {
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticipantContext {
    pub reference: VoiceRef,
    pub trust: ContextTrust,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// Neutral fixed-width PCM descriptor for the first profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaDescriptor {
    pub sample_format: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub packet_time_ms: u16,
    pub frame_bytes: usize,
}

impl MediaDescriptor {
    pub fn pcm_s16le_8khz_mono_20ms() -> Self {
        Self {
            sample_format: "pcm_s16le".to_owned(),
            sample_rate_hz: 8_000,
            channels: 1,
            packet_time_ms: 20,
            frame_bytes: 320,
        }
    }

    pub fn validate(&self) -> Result<(), VoiceError> {
        if self == &Self::pcm_s16le_8khz_mono_20ms() {
            Ok(())
        } else {
            Err(VoiceError::UnsupportedMedia)
        }
    }

    pub fn packet_time(&self) -> Duration {
        Duration::from_millis(u64::from(self.packet_time_ms))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFrame {
    pub sequence: u64,
    pub bytes: Vec<u8>,
}

impl AudioFrame {
    pub fn new(sequence: u64, bytes: Vec<u8>, media: &MediaDescriptor) -> Result<Self, VoiceError> {
        if bytes.len() != media.frame_bytes || bytes.len() > MAX_AUDIO_FRAME_BYTES {
            return Err(VoiceError::FrameTooLarge);
        }
        Ok(Self { sequence, bytes })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChannelSignal {
    Dtmf { digits: String },
}

impl ChannelSignal {
    pub fn validate(&self) -> Result<(), VoiceError> {
        match self {
            Self::Dtmf { digits }
                if !digits.is_empty()
                    && digits.len() <= MAX_SIGNAL_DIGITS
                    && digits
                        .bytes()
                        .all(|byte| matches!(byte, b'0'..=b'9' | b'*' | b'#' | b'A'..=b'D')) =>
            {
                Ok(())
            }
            Self::Dtmf { .. } => Err(VoiceError::InvalidSignal),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    Completed,
    Cancelled,
    RemoteHangup,
    AuthorityRevoked,
    LeaseExpired,
    MediaOverload,
    TransportLost,
    ProtocolError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceSessionDescriptor {
    pub call: VoiceRef,
    pub session: VoiceRef,
    pub channel: VoiceRef,
    pub participant: ParticipantContext,
    pub media: MediaDescriptor,
}

/// Connectors-internal admitted call/media port. No SIP, RTP, RTVBP, carrier, or product type
/// crosses it.
#[async_trait]
pub trait TelephonySession: Send + Sync {
    fn descriptor(&self) -> &VoiceSessionDescriptor;
    async fn read_input(&self) -> Result<Option<AudioFrame>, VoiceError>;
    async fn write_output(&self, frame: AudioFrame) -> Result<(), VoiceError>;
    async fn next_signal(&self) -> Result<Option<ChannelSignal>, VoiceError>;

    /// Send a signal to the far end.
    ///
    /// The write direction of [`TelephonySession::next_signal`], and the reason a call can be
    /// acted on after it is up: a keypress is not audio, so it cannot travel through
    /// [`TelephonySession::write_output`], and an IVR that asks for a digit cannot be answered
    /// without this.
    ///
    /// Defaulted to a refusal so a binding that cannot signal says so by name rather than by
    /// silently accepting a keypress the far end never receives. A caller that gets
    /// [`VoiceError::SignalUnsupported`] learns the binding's limit; one that got `Ok(())` would
    /// learn nothing and wait for a response that is not coming.
    ///
    /// # Errors
    ///
    /// [`VoiceError::SignalUnsupported`] when this binding cannot send signals,
    /// [`VoiceError::InvalidSignal`] for a signal outside its grammar, and
    /// [`VoiceError::Terminated`] once the session has ended.
    async fn send_signal(&self, signal: ChannelSignal) -> Result<(), VoiceError> {
        let _ = signal;
        Err(VoiceError::SignalUnsupported)
    }
    /// Wait for the driver's first terminal fact without guessing from media or signal EOF.
    async fn wait_terminated(&self) -> Result<TerminationReason, VoiceError>;
    async fn interrupt_output(&self) -> Result<(), VoiceError>;
    async fn terminate(&self, reason: TerminationReason) -> Result<(), VoiceError>;
}
