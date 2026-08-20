//! The neutral speech-synthesis port — the *speech* half of this driver, not the device half.
//!
//! Speech is a transformation that produces PCM; [`crate::device`] is what carries PCM to the local
//! sound stack. Keeping them apart is what lets `audio_v1` stay the driver word: a later speech
//! **recognition** port would be a second transformation over the same device, and a notification
//! tone would be no transformation at all.
//!
//! One implementation owns exactly one synthesis path. It never selects another engine, voice,
//! output device, or destination as a fallback, and it never retries a failed component through a
//! different one. No type here names a synthesizer product.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use domain::audio::AudioSink;
use serde::{Deserialize, Serialize};

/// The exact components one engine resolved, retained as the operator-facing device snapshot.
///
/// Replacing the synthesizer or the voice changes this value. It carries no credential, and it is
/// never surfaced to a model: the model-facing projection is `protocol::audio::SpeechReadiness`,
/// which carries no path, executable, or digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechAttestation {
    /// The engine identity recorded in evidence.
    pub engine: String,
    /// The resolved synthesizer executable.
    pub synthesizer_path: String,
    /// The synthesizer's content digest.
    pub synthesizer_sha256: String,
    /// The voice's own identity, as its configuration names it.
    pub voice_id: String,
    /// The resolved voice-model path.
    pub voice_path: String,
    /// The voice model's content digest.
    pub voice_sha256: String,
    /// The resolved voice configuration path.
    pub voice_config_path: String,
    /// The voice's own sample rate. Never a constant.
    pub sample_rate_hz: u32,
    /// Channel count. The first profile is mono.
    pub channels: u8,
    /// The device's stack family, absent on a host that has no sound stack at all.
    ///
    /// A headless deployment records `None` here rather than a fabricated family: synthesis still
    /// ran and still produced the byte count below, and the evidence says where it went.
    pub sink: Option<AudioSink>,
    /// The exact executable the device was found as — deliberately not canonicalized — or the
    /// device's own name where there is no executable.
    pub sink_path: String,
}

/// One bounded utterance that has already passed every admitted bound.
///
/// It exists so an engine cannot be handed unvalidated text: construction is the only way to
/// obtain one, and construction enforces the bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Utterance {
    text: String,
    characters: u32,
}

impl Utterance {
    /// Accepts exactly one bounded, single-line utterance.
    ///
    /// # Errors
    ///
    /// Refuses empty text, text carrying a NUL or control character, and text longer than
    /// `max_characters`. Over-length text is refused rather than truncated, because speaking a
    /// prefix would report that something was said which was not.
    pub fn new(text: &str, max_characters: u32) -> Result<Self, SpeechEngineError> {
        let characters = text.chars().count();
        if text.trim().is_empty() {
            return Err(SpeechEngineError::UtteranceRejected {
                reason: "utterance is empty".to_owned(),
            });
        }
        if text
            .chars()
            .any(|value| value == '\0' || value.is_control())
        {
            return Err(SpeechEngineError::UtteranceRejected {
                reason: "utterance carries a control character".to_owned(),
            });
        }
        let characters = u32::try_from(characters).map_err(|_| SpeechEngineError::TooLarge {
            characters: u32::MAX,
            maximum: max_characters,
        })?;
        if characters > max_characters {
            return Err(SpeechEngineError::TooLarge {
                characters,
                maximum: max_characters,
            });
        }
        Ok(Self {
            text: text.to_owned(),
            characters,
        })
    }

    /// The exact admitted text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// How many characters — not bytes — the utterance carries.
    #[must_use]
    pub const fn characters(&self) -> u32 {
        self.characters
    }
}

/// What one completed or interrupted utterance actually produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechOutcome {
    /// The engine that produced the audio.
    pub engine: String,
    /// How many characters were synthesized.
    pub characters: u32,
    /// How many bytes of PCM reached the sink.
    pub audio_bytes: u64,
    /// The voice's own sample rate.
    pub sample_rate_hz: u32,
    /// Wall-clock duration.
    pub duration_ms: u64,
    /// Whether audio ran to completion.
    pub completed: bool,
}

/// A cancellation an engine observes while it holds the device.
///
/// The owner sets it; the engine polls it. Nothing drains a control channel while a synchronous
/// utterance is in flight, so this flag is the only path by which a cancellation reaches the audio
/// pipeline. It is one-way: a stopped utterance never resumes.
#[derive(Debug, Clone, Default)]
pub struct SpeechCancellation(Arc<AtomicBool>);

impl SpeechCancellation {
    /// A fresh, unstopped cancellation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stop the utterance in flight, and every later one sharing this flag.
    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether a stop has been observed.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Every way local speech refuses.
///
/// Each variant names the exact component or bound at fault, because the operator's next action
/// differs for a missing synthesizer, a missing voice, and a missing audio device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SpeechEngineError {
    /// The synthesizer executable is absent or not executable.
    #[error("speech synthesizer is unavailable at `{path}`: {reason}")]
    SynthesizerUnavailable {
        /// The path that was tried.
        path: String,
        /// Why it was refused.
        reason: String,
    },
    /// The voice model is absent or unreadable.
    #[error("speech voice model is unavailable at `{path}`: {reason}")]
    VoiceUnavailable {
        /// The path that was tried.
        path: String,
        /// Why it was refused.
        reason: String,
    },
    /// The voice configuration is missing, malformed, or carries no usable rate.
    #[error("speech voice configuration `{path}` is unreadable or invalid: {reason}")]
    VoiceConfigurationInvalid {
        /// The path that was tried.
        path: String,
        /// Why it was refused.
        reason: String,
    },
    /// The voice model's digest does not match the deployment's pin.
    #[error("speech voice digest is `{actual}`, expected the pinned `{expected}`")]
    VoiceDigestMismatch {
        /// The pinned digest.
        expected: String,
        /// The digest actually measured.
        actual: String,
    },
    /// No local audio sink is present.
    #[error("no local audio sink is available: {reason}")]
    SinkUnavailable {
        /// Why no sink was admitted.
        reason: String,
    },
    /// The utterance is above the admitted character bound.
    #[error("utterance is {characters} characters, above the admitted maximum of {maximum}")]
    TooLarge {
        /// The measured character count.
        characters: u32,
        /// The admitted maximum.
        maximum: u32,
    },
    /// The utterance failed the admitted grammar.
    #[error("utterance was rejected: {reason}")]
    UtteranceRejected {
        /// Why it was refused.
        reason: String,
    },
    /// The device is already speaking.
    #[error("this connection is already speaking")]
    AlreadySpeaking,
    /// The per-connection utterance budget is exhausted.
    #[error("the per-connection utterance budget of {maximum} is exhausted")]
    BudgetExhausted {
        /// The admitted budget.
        maximum: u32,
    },
    /// The owner cancelled while audio was flowing.
    #[error("speech stopped after {audio_bytes} bytes because the operation was cancelled")]
    Cancelled {
        /// How much audio had already been emitted.
        audio_bytes: u64,
    },
    /// The utterance exceeded its wall-clock bound.
    #[error("speech exceeded its {maximum_ms} ms bound after {audio_bytes} bytes")]
    TimedOut {
        /// How much audio had already been emitted.
        audio_bytes: u64,
        /// The admitted bound.
        maximum_ms: u64,
    },
    /// Any other refusal from the engine itself.
    #[error("speech engine refused: {reason}")]
    Refused {
        /// Why it was refused.
        reason: String,
    },
}

impl SpeechEngineError {
    /// The stable machine-readable code carried into an operation refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SynthesizerUnavailable { .. } => "speech-synthesizer-unavailable",
            Self::VoiceUnavailable { .. } => "speech-voice-unavailable",
            Self::VoiceConfigurationInvalid { .. } => "speech-voice-configuration-invalid",
            Self::VoiceDigestMismatch { .. } => "speech-voice-digest-mismatch",
            Self::SinkUnavailable { .. } => "speech-sink-unavailable",
            Self::TooLarge { .. } => "speech-utterance-too-large",
            Self::UtteranceRejected { .. } => "speech-utterance-rejected",
            Self::AlreadySpeaking => "speech-already-speaking",
            Self::BudgetExhausted { .. } => "speech-budget-exhausted",
            Self::Cancelled { .. } => "speech-cancelled",
            Self::TimedOut { .. } => "speech-timed-out",
            Self::Refused { .. } => "speech-refused",
        }
    }

    /// The operator-facing action that would supply the missing component.
    ///
    /// It is returned to the operator, never to a model, and it therefore may name a package, a
    /// configuration field, or a path.
    #[must_use]
    pub fn remediation(&self) -> Option<String> {
        match self {
            Self::SynthesizerUnavailable { .. } => Some(
                "install a local neural speech synthesizer (Arch/AUR package `piper-tts-bin` \
                 provides `piper-tts`) or configure an absolute `synthesizer` path on this \
                 Connection's audio route"
                    .to_owned(),
            ),
            Self::VoiceUnavailable { .. } => Some(
                "install a voice model and configure an absolute `voice` path on this Connection's \
                 audio route"
                    .to_owned(),
            ),
            Self::VoiceConfigurationInvalid { .. } => Some(
                "the voice needs its companion `.onnx.json`; configure an absolute `voice_config` \
                 path on this Connection's audio route"
                    .to_owned(),
            ),
            Self::SinkUnavailable { .. } => {
                Some("install one of `pw-play`, `paplay`, or `aplay`".to_owned())
            }
            _ => None,
        }
    }
}

/// One bounded local speech-synthesis engine.
pub trait SpeechEngine: Send {
    /// The exact engine identity retained in evidence.
    fn id(&self) -> &'static str;

    /// Re-checks that every required component is present right now.
    ///
    /// # Errors
    ///
    /// Returns a refusal naming exactly which component is missing. Absence of a component is
    /// never reported as a degraded success.
    fn probe(&mut self) -> Result<SpeechAttestation, SpeechEngineError>;

    /// Synthesizes and plays exactly one bounded utterance, returning after audio has stopped.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal for a missing component, an exceeded bound, a non-zero engine exit,
    /// or an observed cancellation. A cancelled utterance reports the audio already emitted rather
    /// than claiming completion.
    fn speak(
        &mut self,
        utterance: &Utterance,
        cancellation: &SpeechCancellation,
    ) -> Result<SpeechOutcome, SpeechEngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_length_text_is_refused_and_never_truncated() {
        let text = "a".repeat(11);
        let error = Utterance::new(&text, 10).unwrap_err();
        assert_eq!(
            error,
            SpeechEngineError::TooLarge {
                characters: 11,
                maximum: 10
            }
        );
        assert_eq!(error.code(), "speech-utterance-too-large");
    }

    #[test]
    fn empty_and_control_bearing_text_refuse_before_any_engine() {
        assert!(matches!(
            Utterance::new("   ", 10).unwrap_err(),
            SpeechEngineError::UtteranceRejected { .. }
        ));
        assert!(matches!(
            Utterance::new("hello\0there", 64).unwrap_err(),
            SpeechEngineError::UtteranceRejected { .. }
        ));
    }

    #[test]
    fn utterance_counts_characters_not_bytes() {
        let utterance = Utterance::new("Grüße", 5).expect("five characters fit");
        assert_eq!(utterance.characters(), 5);
        assert_eq!(utterance.text(), "Grüße");
    }

    #[test]
    fn cancellation_is_one_way() {
        let cancellation = SpeechCancellation::new();
        assert!(!cancellation.is_stopped());
        cancellation.stop();
        assert!(cancellation.is_stopped());
        let clone = cancellation.clone();
        assert!(clone.is_stopped());
    }
}
