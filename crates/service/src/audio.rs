//! Application-layer admission of the exact device facts a local-audio driver may consume.
//!
//! The same shape as SIP admission, for the same reason: the crate that is allowed to spawn a
//! synthesizer and open a sound device receives non-serializable evidence it cannot mint itself,
//! and every device fact in that evidence is deployment-owned. A caller supplies one bounded
//! utterance; it never supplies — and cannot name — a voice, a sample rate, a sink, an executable,
//! or a filesystem path.

use std::path::{Path, PathBuf};
use std::time::Duration;

use domain::audio::AudioSink;
use domain::{DriverId, Interaction, ProtocolPlan, ZeroIoPlan};
use protocol::audio::{
    SpeechSpeakInput, MAX_UTTERANCE_CHARACTERS, SPEECH_SPEAK_OPERATION, SPEECH_STATUS_OPERATION,
};

/// Maximum wall-clock time one admitted utterance may occupy the device.
pub const MAX_UTTERANCE: Duration = Duration::from_secs(120);

/// How many utterances one admitted Connection may speak.
pub const MAX_UTTERANCES_PER_CONNECTION: u32 = 32;

/// Deployment-selected device route. No request or model field can construct any part of this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeploymentRoute {
    /// The Connection this route belongs to.
    pub connection: String,
    /// An explicit absolute synthesizer path. `None` leaves discovery to the driver, which admits
    /// exactly one executable name and never a bare guess.
    pub synthesizer: Option<PathBuf>,
    /// The absolute voice-model path. There is no default: a voice is always chosen explicitly.
    pub voice: PathBuf,
    /// The absolute voice configuration. `None` means the model path plus `.json`.
    pub voice_config: Option<PathBuf>,
    /// An optional digest pin. The digest is always computed and recorded; a pin additionally
    /// refuses on mismatch.
    pub voice_sha256: Option<String>,
    /// An explicit sink family. `None` probes the candidates in order, once.
    pub sink: Option<AudioSink>,
    /// The admitted per-utterance character bound.
    pub maximum_characters: u32,
    /// The admitted wall-clock bound on one utterance.
    pub maximum_utterance: Duration,
}

/// Failure before the device-capable crate receives a plan.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AudioAdmissionError {
    /// The plan is not an admitted local-audio unary operation.
    #[error("operation is not an admitted local-audio operation")]
    WrongOperation,
    /// Organization, principal, or grant is missing from the admitted plan.
    #[error("admitted local-audio identity is incomplete")]
    InvalidIdentity,
    /// The deployment route belongs to a different Connection.
    #[error("deployment audio route belongs to another Connection")]
    ConnectionMismatch,
    /// A device path is relative, so what it resolves to would depend on ambient state.
    #[error("audio route path is not absolute")]
    RelativePath,
    /// The character bound is zero or above the published ceiling.
    #[error("audio route character bound is outside the published ceiling")]
    InvalidCharacterBound,
    /// The utterance deadline is zero or above the published ceiling.
    #[error("audio route has an invalid finite deadline")]
    InvalidDeadline,
    /// A pinned voice digest is not 64 lowercase hexadecimal characters.
    #[error("audio route voice digest pin is not a SHA-256 hexadecimal digest")]
    InvalidVoiceDigest,
    /// The caller's utterance failed the published grammar or the admitted bound.
    #[error("speech.speak text was refused: {0}")]
    UtteranceRefused(&'static str),
}

/// Check every deployment-owned field before it can reach the device-capable crate.
///
/// # Errors
///
/// Returns the exact refusal naming the field at fault.
pub fn validate_audio_deployment_route(
    route: &AudioDeploymentRoute,
) -> Result<(), AudioAdmissionError> {
    if !route.voice.is_absolute() {
        return Err(AudioAdmissionError::RelativePath);
    }
    for optional in [route.synthesizer.as_deref(), route.voice_config.as_deref()] {
        if optional.is_some_and(|path: &Path| !path.is_absolute()) {
            return Err(AudioAdmissionError::RelativePath);
        }
    }
    if route.maximum_characters == 0 || route.maximum_characters > MAX_UTTERANCE_CHARACTERS {
        return Err(AudioAdmissionError::InvalidCharacterBound);
    }
    if route.maximum_utterance.is_zero() || route.maximum_utterance > MAX_UTTERANCE {
        return Err(AudioAdmissionError::InvalidDeadline);
    }
    if let Some(digest) = &route.voice_sha256 {
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AudioAdmissionError::InvalidVoiceDigest);
        }
    }
    Ok(())
}

/// Join grant admission and deployment-only device selection into one device-opening proof.
///
/// # Errors
///
/// Refuses a plan for another driver, another lifecycle, another Connection, an incomplete
/// identity, or a route that fails [`validate_audio_deployment_route`].
pub fn admit_audio_plan(
    plan: &ZeroIoPlan,
    route: AudioDeploymentRoute,
) -> Result<AdmittedAudioPlan, AudioAdmissionError> {
    let ProtocolPlan::AudioV1(audio) = plan.protocol() else {
        return Err(AudioAdmissionError::WrongOperation);
    };
    if plan.protocol().driver() != DriverId::AudioV1
        || plan.facts().interaction != Interaction::Unary
    {
        return Err(AudioAdmissionError::WrongOperation);
    }
    if !matches!(
        plan.facts().operation.as_str(),
        SPEECH_SPEAK_OPERATION | SPEECH_STATUS_OPERATION
    ) {
        return Err(AudioAdmissionError::WrongOperation);
    }
    if plan.admission().organization().is_empty()
        || plan.admission().principal().is_empty()
        || plan.admission().grant().is_empty()
    {
        return Err(AudioAdmissionError::InvalidIdentity);
    }
    if audio.connection != route.connection || plan.admission().connection() != route.connection {
        return Err(AudioAdmissionError::ConnectionMismatch);
    }
    validate_audio_deployment_route(&route)?;
    Ok(AdmittedAudioPlan {
        provider: plan.facts().provider.clone(),
        operation: plan.facts().operation.clone(),
        organization: plan.admission().organization().to_owned(),
        principal: plan.admission().principal().to_owned(),
        grant: plan.admission().grant().to_owned(),
        route,
        _proof: AdmissionProof,
    })
}

/// Admit one bounded utterance for `speech.speak` only.
///
/// The caller's text is checked twice on purpose: once against the grammar the catalog publishes,
/// and once against the bound this deployment actually admitted, which may be lower.
///
/// # Errors
///
/// Refuses any operation other than `speech-speak`, and any text the published grammar or the
/// admitted bound rejects.
pub fn admit_speech_speak(
    plan: &ZeroIoPlan,
    input: &SpeechSpeakInput,
    route: AudioDeploymentRoute,
) -> Result<(AdmittedAudioPlan, u32), AudioAdmissionError> {
    if plan.facts().operation != SPEECH_SPEAK_OPERATION {
        return Err(AudioAdmissionError::WrongOperation);
    }
    let characters = input.validate().map_err(|error| match error {
        protocol::audio::SpeechSpeakInputError::Empty => {
            AudioAdmissionError::UtteranceRefused("text is empty")
        }
        protocol::audio::SpeechSpeakInputError::ControlCharacter => {
            AudioAdmissionError::UtteranceRefused("text carries a control character")
        }
        protocol::audio::SpeechSpeakInputError::TooLong => {
            AudioAdmissionError::UtteranceRefused("text is above the published character bound")
        }
    })?;
    let admitted = admit_audio_plan(plan, route)?;
    if characters > admitted.route.maximum_characters {
        return Err(AudioAdmissionError::UtteranceRefused(
            "text is above this deployment's admitted character bound",
        ));
    }
    Ok((admitted, characters))
}

/// Non-serializable evidence handed only to the `driver-speech` crate, which turns it into an
/// engine bound to whichever `driver-audio` device the deployment provided.
pub struct AdmittedAudioPlan {
    provider: String,
    operation: String,
    organization: String,
    principal: String,
    grant: String,
    route: AudioDeploymentRoute,
    _proof: AdmissionProof,
}

struct AdmissionProof;

impl AdmittedAudioPlan {
    /// The Provider this device operation belongs to.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The exact admitted operation id.
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// The admitted organization.
    #[must_use]
    pub fn organization(&self) -> &str {
        &self.organization
    }

    /// The admitted principal.
    #[must_use]
    pub fn principal(&self) -> &str {
        &self.principal
    }

    /// The admitted Connector Grant.
    #[must_use]
    pub fn grant(&self) -> &str {
        &self.grant
    }

    /// The deployment-selected device route.
    #[must_use]
    pub fn route(&self) -> &AudioDeploymentRoute {
        &self.route
    }
}

impl std::fmt::Debug for AdmittedAudioPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedAudioPlan")
            .field("provider", &self.provider)
            .field("operation", &self.operation)
            .field("organization", &self.organization)
            .field("principal", &self.principal)
            .field("grant", &self.grant)
            .field("route", &self.route)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use domain::{
        AdmittedOperation, AudioPlan, Capability, ConnectionAuthority, Implementation,
        InitiationPolicy, OperationFacts, Placement,
    };

    use super::*;

    fn route() -> AudioDeploymentRoute {
        AudioDeploymentRoute {
            connection: "connection-1".to_owned(),
            synthesizer: None,
            voice: PathBuf::from("/voices/en_US-ryan-high.onnx"),
            voice_config: None,
            voice_sha256: None,
            sink: None,
            maximum_characters: 1_000,
            maximum_utterance: Duration::from_secs(30),
        }
    }

    fn plan(operation: &str) -> ZeroIoPlan {
        ZeroIoPlan::new(
            OperationFacts {
                provider: "b10x".to_owned(),
                operation: operation.to_owned(),
                service: "default".to_owned(),
                interaction: Interaction::Unary,
                placement: Placement::FederatedSatellite,
                implementation: Implementation::BuiltIn,
                required_capabilities: BTreeSet::from([Capability::Device]),
                permission_subjects: vec!["device:local-audio-output".to_owned()],
            },
            AdmittedOperation::for_local_owner(
                "b10x",
                operation,
                "org",
                "principal-1",
                "grant-1",
                ConnectionAuthority::new("connection-1", InitiationPolicy::platform_only())
                    .unwrap(),
            ),
            ProtocolPlan::AudioV1(AudioPlan {
                connection: "connection-1".to_owned(),
            }),
        )
    }

    #[test]
    fn a_speech_plan_and_its_deployment_route_admit_together() {
        let admitted = admit_audio_plan(&plan(SPEECH_SPEAK_OPERATION), route()).expect("admitted");
        assert_eq!(admitted.provider(), "b10x");
        assert_eq!(admitted.operation(), SPEECH_SPEAK_OPERATION);
        assert_eq!(admitted.route().maximum_characters, 1_000);
    }

    #[test]
    fn a_plan_for_another_driver_or_operation_is_refused() {
        let foreign = ZeroIoPlan::new(
            plan(SPEECH_SPEAK_OPERATION).facts().clone(),
            AdmittedOperation::for_local_owner(
                "b10x",
                SPEECH_SPEAK_OPERATION,
                "org",
                "principal-1",
                "grant-1",
                ConnectionAuthority::new("connection-1", InitiationPolicy::platform_only())
                    .unwrap(),
            ),
            ProtocolPlan::SipV1(domain::SipPlan {
                connection: "connection-1".to_owned(),
            }),
        );
        assert_eq!(
            admit_audio_plan(&foreign, route()).expect_err("a SIP plan is not an audio plan"),
            AudioAdmissionError::WrongOperation
        );
        assert_eq!(
            admit_audio_plan(&plan("speech-shout"), route())
                .expect_err("an unknown operation is refused"),
            AudioAdmissionError::WrongOperation
        );
    }

    #[test]
    fn a_route_for_another_connection_is_refused() {
        let mut elsewhere = route();
        elsewhere.connection = "connection-2".to_owned();
        assert_eq!(
            admit_audio_plan(&plan(SPEECH_SPEAK_OPERATION), elsewhere)
                .expect_err("another Connection's route is refused"),
            AudioAdmissionError::ConnectionMismatch
        );
    }

    #[test]
    fn relative_paths_absent_bounds_and_bad_digests_never_reach_the_device() {
        let mut relative = route();
        relative.voice = PathBuf::from("voices/en_US-ryan-high.onnx");
        assert_eq!(
            validate_audio_deployment_route(&relative),
            Err(AudioAdmissionError::RelativePath)
        );

        let mut relative_synthesizer = route();
        relative_synthesizer.synthesizer = Some(PathBuf::from("bin/piper-tts"));
        assert_eq!(
            validate_audio_deployment_route(&relative_synthesizer),
            Err(AudioAdmissionError::RelativePath)
        );

        for characters in [0, MAX_UTTERANCE_CHARACTERS + 1] {
            let mut bound = route();
            bound.maximum_characters = characters;
            assert_eq!(
                validate_audio_deployment_route(&bound),
                Err(AudioAdmissionError::InvalidCharacterBound)
            );
        }

        for deadline in [Duration::ZERO, MAX_UTTERANCE + Duration::from_secs(1)] {
            let mut bound = route();
            bound.maximum_utterance = deadline;
            assert_eq!(
                validate_audio_deployment_route(&bound),
                Err(AudioAdmissionError::InvalidDeadline)
            );
        }

        let mut pinned = route();
        pinned.voice_sha256 = Some("not-a-digest".to_owned());
        assert_eq!(
            validate_audio_deployment_route(&pinned),
            Err(AudioAdmissionError::InvalidVoiceDigest)
        );
        pinned.voice_sha256 = Some("a".repeat(64));
        assert_eq!(validate_audio_deployment_route(&pinned), Ok(()));
    }

    #[test]
    fn the_caller_text_is_bounded_by_the_deployment_and_not_only_by_the_catalog() {
        let mut narrow = route();
        narrow.maximum_characters = 4;
        let error = admit_speech_speak(
            &plan(SPEECH_SPEAK_OPERATION),
            &SpeechSpeakInput {
                text: "Ready.".to_owned(),
            },
            narrow,
        )
        .map(|(_, characters)| characters)
        .expect_err("the deployment bound is lower than the catalog's");
        assert!(matches!(error, AudioAdmissionError::UtteranceRefused(_)));

        let (_, characters) = admit_speech_speak(
            &plan(SPEECH_SPEAK_OPERATION),
            &SpeechSpeakInput {
                text: "Ready.".to_owned(),
            },
            route(),
        )
        .expect("bounded text admits");
        assert_eq!(characters, 6);
    }

    #[test]
    fn status_never_admits_an_utterance() {
        assert_eq!(
            admit_speech_speak(
                &plan(SPEECH_STATUS_OPERATION),
                &SpeechSpeakInput {
                    text: "Ready.".to_owned(),
                },
                route(),
            )
            .map(|(_, characters)| characters)
            .expect_err("status carries no utterance"),
            AudioAdmissionError::WrongOperation
        );
    }

    #[test]
    fn admitted_evidence_never_prints_as_a_serializable_route_secret() {
        let admitted = admit_audio_plan(&plan(SPEECH_STATUS_OPERATION), route()).expect("admitted");
        let rendered = format!("{admitted:?}");
        assert!(rendered.contains("AdmittedAudioPlan"));
        assert!(rendered.contains(".."), "evidence must stay non-exhaustive");
    }
}
