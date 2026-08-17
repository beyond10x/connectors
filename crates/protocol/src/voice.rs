//! Protocol-neutral `VoiceSession v0alpha1` projection.

use domain::voice::{
    ChannelSignal, ContextTrust, MediaDescriptor, ParticipantContext, TerminationReason, VoiceRef,
    VoiceSessionDescriptor,
};
use serde::{Deserialize, Serialize};

/// Contract identity carried by the owner bundle.
pub const CONTRACT: &str = "b10x.voice-session.v0alpha1";

/// Self-describing protocol-neutral message. Bindings map `operation` onto their own method/event
/// vocabulary; they do not make that vocabulary part of the owner contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", content = "payload", rename_all = "snake_case")]
pub enum VoiceMessage {
    /// Establish one admitted neutral session.
    Initialize(Initialize),
    /// Confirm the selected contract and transition to ready.
    Ready(Ready),
    /// Interrupt buffered output for one explicit cause.
    InterruptOutput(InterruptOutput),
    /// Confirm an idempotent control.
    Acknowledged(Acknowledged),
    /// Request typed session closure.
    Close(Close),
    /// Report observable bounded input loss.
    MediaLoss(MediaLoss),
    /// Deliver an optional neutral channel signal.
    Signal(Signal),
    /// Report the single terminal fact.
    Terminated(Terminated),
}

/// One bounded session initialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Initialize {
    /// Opaque call reference.
    pub call: VoiceRef,
    /// Opaque session reference.
    pub session: VoiceRef,
    /// Opaque application-channel reference.
    pub channel: VoiceRef,
    /// Explicitly untrusted remote participant context.
    pub participant: ParticipantContext,
    /// Negotiated media descriptor.
    pub media: MediaDescriptor,
}

impl From<&VoiceSessionDescriptor> for Initialize {
    fn from(value: &VoiceSessionDescriptor) -> Self {
        Self {
            call: value.call.clone(),
            session: value.session.clone(),
            channel: value.channel.clone(),
            participant: value.participant.clone(),
            media: value.media.clone(),
        }
    }
}

/// Successful initialization response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ready {
    /// Contract identity accepted by the peer.
    pub contract: String,
}

/// Request to clear bounded output without implying Agent cancellation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterruptOutput {
    /// Causal application-side reference used for audit correlation.
    pub cause: VoiceRef,
}

/// Acknowledgement for idempotent controls.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acknowledged {}

/// Request to close a live session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Close {
    /// Typed terminal reason.
    pub reason: TerminationReason,
}

/// Observable media loss. Counts are facts, never silently repaired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaLoss {
    /// First lost sequence number.
    pub first_sequence: u64,
    /// Number of frames lost.
    pub frames: u32,
}

/// Optional channel signal event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signal {
    /// Neutral channel signal.
    pub signal: ChannelSignal,
}

/// Exactly one terminal fact emitted by an endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terminated {
    /// Typed reason selected by the serialized terminal event loop.
    pub reason: TerminationReason,
}

/// Closed outcome vocabulary used by executable semantic vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Operation or transition is admitted.
    Accepted,
    /// Operation continues with observable bounded loss.
    Degraded,
    /// Operation or transition is refused.
    Refused,
}

/// One executable semantic vector from the owner bundle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vector {
    /// Stable scenario identity.
    pub case: String,
    /// Initial state token.
    pub from: String,
    /// Input semantic action.
    pub action: String,
    /// Resulting state token.
    pub to: String,
    /// Expected outcome class.
    pub outcome: Outcome,
    /// Stable result/refusal code.
    pub code: String,
}

/// Embedded owner vectors used by both endpoint implementations.
pub fn vectors() -> Result<Vec<Vector>, serde_json::Error> {
    serde_json::from_str(include_str!(
        "../../../contracts/voice-session/v0alpha1/vectors.json"
    ))
}

/// Build the first profile's neutral initialization descriptor for tests and fixtures.
pub fn fixture_initialize() -> Initialize {
    Initialize {
        call: VoiceRef::new("call-1").expect("static fixture reference is valid"),
        session: VoiceRef::new("session-1").expect("static fixture reference is valid"),
        channel: VoiceRef::new("channel-1").expect("static fixture reference is valid"),
        participant: ParticipantContext {
            reference: VoiceRef::new("participant-1").expect("static fixture reference is valid"),
            trust: ContextTrust::Untrusted,
            display: Some("Synthetic caller".to_owned()),
        },
        media: MediaDescriptor::pcm_s16le_8khz_mono_20ms(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn owner_vectors_are_closed_and_unique() {
        let vectors = vectors().expect("embedded vectors parse");
        assert!(!vectors.is_empty());
        let cases = vectors
            .iter()
            .map(|vector| vector.case.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(cases.len(), vectors.len());
        assert!(vectors
            .iter()
            .any(|vector| vector.case == "bounded-input-loss"));
        assert!(vectors
            .iter()
            .any(|vector| vector.case == "terminal-is-once"));
    }

    #[test]
    fn fixture_context_cannot_claim_trust() {
        let json = serde_json::to_value(VoiceMessage::Initialize(fixture_initialize()))
            .expect("serializes");
        assert_eq!(json["operation"], "initialize");
        assert_eq!(json["payload"]["participant"]["trust"], "untrusted");
        assert!(!json.to_string().contains("rtvbp"));
        assert!(!json.to_string().contains("sip"));
    }
}
