//! Public input/output projection for local speech output.
//!
//! A caller supplies one bounded utterance and nothing else. No voice, sample rate, device,
//! synthesizer, executable, or filesystem path crosses this boundary in either direction: those are
//! deployment-owned facts resolved behind the closed `audio_v1` driver after admission.

use serde::{Deserialize, Serialize};

/// Canonical catalog id. Connector tool projection renders this as `speech.speak`.
pub const SPEECH_SPEAK_OPERATION: &str = "speech-speak";

/// Model/harness-facing operation reference derived from [`SPEECH_SPEAK_OPERATION`].
pub const SPEECH_SPEAK_TOOL_REF: &str = "speech.speak";

/// Canonical catalog id. Connector tool projection renders this as `speech.status`.
pub const SPEECH_STATUS_OPERATION: &str = "speech-status";

/// Model/harness-facing operation reference derived from [`SPEECH_STATUS_OPERATION`].
pub const SPEECH_STATUS_TOOL_REF: &str = "speech.status";

/// Stable Provider id for the platform-owned local speech capability.
pub const SPEECH_PROVIDER: &str = "b10x";

/// Permanent Provider authority for platform-owned Connector capabilities.
pub const SPEECH_PROVIDER_AUTHORITY: &str = "io.b10x";

/// The per-utterance character bound the catalog publishes.
///
/// Calibrated against a measured run: 20 characters produced 58,996 bytes of 22.05 kHz signed
/// 16-bit mono audio, so a thousand characters is roughly a minute of speech.
pub const DEFAULT_MAX_UTTERANCE_CHARACTERS: u32 = 1_000;

/// The hard ceiling a deployment may not raise the character bound above.
pub const MAX_UTTERANCE_CHARACTERS: u32 = 4_000;

/// A published default above the ceiling would advertise text the driver is required to refuse.
const _: () = assert!(DEFAULT_MAX_UTTERANCE_CHARACTERS <= MAX_UTTERANCE_CHARACTERS);

/// Caller input for one bounded speech request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechSpeakInput {
    /// Exactly one bounded, single-line utterance.
    pub text: String,
}

impl SpeechSpeakInput {
    /// Validate the catalog's closed utterance grammar.
    ///
    /// Over-length text is refused rather than truncated, because speaking a prefix would report
    /// that something was said which was not.
    ///
    /// # Errors
    ///
    /// Refuses empty or whitespace-only text, text carrying a NUL or other control character, and
    /// text longer than [`DEFAULT_MAX_UTTERANCE_CHARACTERS`].
    pub fn validate(&self) -> Result<u32, SpeechSpeakInputError> {
        if self.text.trim().is_empty() {
            return Err(SpeechSpeakInputError::Empty);
        }
        if self.text.chars().any(char::is_control) {
            return Err(SpeechSpeakInputError::ControlCharacter);
        }
        let characters =
            u32::try_from(self.text.chars().count()).map_err(|_| SpeechSpeakInputError::TooLong)?;
        if characters > DEFAULT_MAX_UTTERANCE_CHARACTERS {
            return Err(SpeechSpeakInputError::TooLong);
        }
        Ok(characters)
    }
}

/// Refusal before an utterance may reach the closed driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SpeechSpeakInputError {
    /// The utterance is empty or only whitespace.
    #[error("speech.speak text is empty")]
    Empty,
    /// The utterance carries a NUL or other control character.
    #[error("speech.speak text carries a control character")]
    ControlCharacter,
    /// The utterance is above the published character bound.
    #[error("speech.speak text is above the admitted character bound")]
    TooLong,
}

/// Successful speech result exposed to the invoking harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechSpoken {
    /// Always true; a refusal is an error, never a `false` here.
    pub spoken: bool,
    /// How many characters were actually synthesized.
    pub characters: u32,
    /// Wall-clock duration of the utterance.
    pub duration_ms: u64,
    /// The voice's own sample rate, which is never a constant.
    pub sample_rate_hz: u32,
    /// Whether audio ran to completion rather than being interrupted.
    pub completed: bool,
}

/// Readiness result exposed to the invoking harness.
///
/// Deliberately carries no path, executable, or digest: this reaches a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechReadiness {
    /// Whether every component required to speak resolved.
    pub ready: bool,
    /// Opaque voice identity, as the deployment's voice configuration names it.
    pub voice: String,
    /// The voice's own sample rate.
    pub sample_rate_hz: u32,
    /// The admitted per-utterance character bound.
    pub max_characters: u32,
    /// How much of the per-connection utterance budget remains.
    pub remaining_utterances: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str) -> SpeechSpeakInput {
        SpeechSpeakInput {
            text: text.to_owned(),
        }
    }

    #[test]
    fn bounded_single_line_text_is_admitted_and_counted_in_characters() {
        assert_eq!(input("Ready.").validate(), Ok(6));
        // Characters, not bytes: five graphemes that occupy seven bytes.
        assert_eq!(input("Grüße").validate(), Ok(5));
    }

    #[test]
    fn empty_control_bearing_and_over_length_text_refuse() {
        assert_eq!(input("").validate(), Err(SpeechSpeakInputError::Empty));
        assert_eq!(input("   ").validate(), Err(SpeechSpeakInputError::Empty));
        assert_eq!(
            input("hello\0there").validate(),
            Err(SpeechSpeakInputError::ControlCharacter)
        );
        assert_eq!(
            input("two\nlines").validate(),
            Err(SpeechSpeakInputError::ControlCharacter)
        );
        let over = "a".repeat(DEFAULT_MAX_UTTERANCE_CHARACTERS as usize + 1);
        assert_eq!(input(&over).validate(), Err(SpeechSpeakInputError::TooLong));
    }

    #[test]
    fn the_input_refuses_any_field_a_caller_invents() {
        let error = serde_json::from_str::<SpeechSpeakInput>(
            r#"{"text":"Ready.","voice":"/voices/en_US-ryan-high.onnx"}"#,
        )
        .expect_err("an unknown field is refused");
        assert!(error.to_string().contains("voice"), "{error}");
    }
}
