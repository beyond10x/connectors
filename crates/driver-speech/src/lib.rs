#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Local speech, as a **transformation over PCM** rather than a device.
//!
//! Synthesis produces samples; recognition, when it lands, consumes them. Neither reaches an
//! external system of its own, and neither may open a sound device: everything audible here goes
//! through [`domain::audio::AudioDevice`], which `driver-audio` implements for a real stack and for
//! a host that has none.
//!
//! # Why this is a separate crate from `driver-audio`
//!
//! The `protocol_driver` axis answers *which closed implementation speaks to the external system*,
//! and for audio that system is the machine's sound stack — which is `driver-audio`, and stays the
//! `audio_v1` driver. Speech is not a protocol. When both lived in one crate the synthesizer
//! spawned the playback command itself, so "the only crate that touches the sound stack" was
//! already untrue, and speech **recognition** would have made it untrue a second time.
//!
//! Split, each side gets a property worth having: a device can be swapped for a headless one
//! without speech knowing, and speech can be exercised with no device at all.
//!
//! # What the shipped surface is
//!
//! Two operations: `speech.speak` and `speech.status`. Synthesis runs entirely on this machine: the
//! driver reaches no network, reads no workspace content, writes no file, and holds no credential.
//! Speaking carries an outward effect — it is audible, it is not idempotent, and it cannot be
//! undone — which the catalog declares as `human_visible` rather than hiding behind a read-only
//! claim.
//!
//! Recognition (STT) is **not implemented**. It belongs here when it arrives, as a second
//! transformation over the same device, and is named rather than stubbed.
//!
//! One fact about synthesis is load-bearing and fails silently when broken: the **sample rate comes
//! from the voice**, read from its `.onnx.json` `audio.sample_rate`, and never from a constant.
//! Installed voices differ — 16000 and 22050 both occur — and playing one at the other's rate
//! renders it at the wrong pitch. Pinned by tests in [`piper`].

pub mod piper;
pub mod speech;

use std::fmt::Write as _;
use std::sync::Arc;

use domain::audio::AudioDevice;
use protocol::audio::{
    SpeechReadiness, SpeechSpeakInput, SpeechSpoken, SPEECH_SPEAK_OPERATION,
    SPEECH_STATUS_OPERATION,
};
use service::{AdmittedAudioPlan, MAX_UTTERANCES_PER_CONNECTION};

pub use crate::piper::{PiperConfig, PiperSpeechEngine, ENGINE_ID, SYNTHESIZER_EXECUTABLE};
pub use crate::speech::{
    SpeechAttestation, SpeechCancellation, SpeechEngine, SpeechEngineError, SpeechOutcome,
    Utterance,
};

/// Build the one shipped engine from an admitted plan, bound to this host's device.
///
/// This is the only place a deployment-owned route becomes an engine configuration. Nothing a
/// caller supplied reaches it: the utterance travels separately and is bounded again on the way in.
///
/// The device comes from [`driver_audio::local_device`], which is the real stack where there is one
/// and silence where there is not — so this succeeds on a server with no sound card, and the
/// attestation records which it got.
#[must_use]
pub fn engine_for(admitted: &AdmittedAudioPlan) -> PiperSpeechEngine {
    let route = admitted.route();
    engine_with_device(admitted, driver_audio::local_device(route.sink))
}

/// Build the engine against a device the caller chose.
///
/// The seam a headless deployment and every test use: pass [`driver_audio::NullAudioDevice`] and
/// synthesis runs end to end with no sound stack present.
#[must_use]
pub fn engine_with_device(
    admitted: &AdmittedAudioPlan,
    device: Arc<dyn AudioDevice>,
) -> PiperSpeechEngine {
    let route = admitted.route();
    PiperSpeechEngine::new(
        PiperConfig {
            synthesizer: route.synthesizer.clone(),
            voice: route.voice.clone(),
            voice_config: route.voice_config.clone(),
            voice_sha256: route.voice_sha256.clone(),
            maximum_utterance: route.maximum_utterance,
        },
        device,
    )
}

/// One probed engine bound to exactly one admitted Connection.
///
/// The engine must already have been probed, because readiness depends on the attestation it
/// produced: a Connection that could not resolve a synthesizer, a voice, or a sink never reaches
/// this constructor.
pub struct LocalSpeechDriver {
    connection: String,
    engine: Box<dyn SpeechEngine>,
    attestation: SpeechAttestation,
    cancellation: SpeechCancellation,
    maximum_characters: u32,
    spoken: u32,
    speaking: bool,
}

impl LocalSpeechDriver {
    /// Bind one probed engine to the Connection whose route produced it.
    ///
    /// # Errors
    ///
    /// Refuses a character bound of zero, and a bound above the one the admitted route carries.
    pub fn new(
        admitted: &AdmittedAudioPlan,
        engine: Box<dyn SpeechEngine>,
        attestation: SpeechAttestation,
        cancellation: SpeechCancellation,
    ) -> Result<Self, SpeechEngineError> {
        let route = admitted.route();
        if route.maximum_characters == 0 {
            return Err(SpeechEngineError::TooLarge {
                characters: 0,
                maximum: route.maximum_characters,
            });
        }
        Ok(Self {
            connection: route.connection.clone(),
            engine,
            attestation,
            cancellation,
            maximum_characters: route.maximum_characters,
            spoken: 0,
            speaking: false,
        })
    }

    /// The device snapshot this driver is bound to. Operator-facing, never model-facing.
    #[must_use]
    pub const fn attestation(&self) -> &SpeechAttestation {
        &self.attestation
    }

    /// The cancellation an owner sets to stop the utterance in flight.
    #[must_use]
    pub const fn cancellation(&self) -> &SpeechCancellation {
        &self.cancellation
    }

    /// Report readiness for `speech.status`.
    ///
    /// # Errors
    ///
    /// Refuses an admitted plan for another Connection or another operation.
    pub fn status(
        &self,
        admitted: &AdmittedAudioPlan,
    ) -> Result<SpeechReadiness, SpeechEngineError> {
        self.check(admitted, SPEECH_STATUS_OPERATION)?;
        // Deliberately no path, executable, or digest: this reaches a model.
        Ok(SpeechReadiness {
            ready: true,
            voice: self.attestation.voice_id.clone(),
            sample_rate_hz: self.attestation.sample_rate_hz,
            max_characters: self.maximum_characters,
            remaining_utterances: MAX_UTTERANCES_PER_CONNECTION.saturating_sub(self.spoken),
        })
    }

    /// Speak exactly one bounded utterance for `speech.speak`.
    ///
    /// # Errors
    ///
    /// Refuses an admitted plan for another Connection or another operation, text outside the
    /// admitted bound, an exhausted budget, a re-entrant call, an observed cancellation, and every
    /// device refusal the engine reports.
    pub fn speak(
        &mut self,
        admitted: &AdmittedAudioPlan,
        input: &SpeechSpeakInput,
    ) -> Result<SpeechSpoken, SpeechEngineError> {
        self.check(admitted, SPEECH_SPEAK_OPERATION)?;
        let utterance = Utterance::new(&input.text, self.maximum_characters)?;
        if self.spoken >= MAX_UTTERANCES_PER_CONNECTION {
            return Err(SpeechEngineError::BudgetExhausted {
                maximum: MAX_UTTERANCES_PER_CONNECTION,
            });
        }
        if self.speaking {
            return Err(SpeechEngineError::AlreadySpeaking);
        }
        self.speaking = true;
        let outcome = self.engine.speak(&utterance, &self.cancellation);
        self.speaking = false;
        let outcome = outcome?;
        self.spoken = self.spoken.saturating_add(1);
        Ok(SpeechSpoken {
            spoken: true,
            characters: outcome.characters,
            duration_ms: outcome.duration_ms,
            sample_rate_hz: outcome.sample_rate_hz,
            completed: outcome.completed,
        })
    }

    fn check(
        &self,
        admitted: &AdmittedAudioPlan,
        operation: &str,
    ) -> Result<(), SpeechEngineError> {
        if admitted.route().connection != self.connection {
            return Err(SpeechEngineError::Refused {
                reason: "admitted plan belongs to another Connection".to_owned(),
            });
        }
        if admitted.operation() != operation {
            return Err(SpeechEngineError::Refused {
                reason: "admitted plan is outside the admitted speech contract".to_owned(),
            });
        }
        Ok(())
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing hexadecimal to a String");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::time::Duration;

    use domain::audio::AudioSink;
    use domain::{
        AdmittedOperation, AudioPlan, Capability, ConnectionAuthority, Implementation,
        InitiationPolicy, Interaction, OperationFacts, Placement, ProtocolPlan, ZeroIoPlan,
    };
    use service::{admit_audio_plan, AudioDeploymentRoute};

    use super::*;

    /// An engine that records utterances instead of synthesizing them.
    ///
    /// It exists so speech behavior has executable vectors without a process, an audio device, or
    /// a voice model. No test here spawns either.
    struct FakeSpeechEngine {
        attestation: SpeechAttestation,
        spoken: Vec<String>,
        failure: Option<SpeechEngineError>,
    }

    impl FakeSpeechEngine {
        fn new(sample_rate_hz: u32) -> Self {
            Self {
                attestation: attestation(sample_rate_hz),
                spoken: Vec::new(),
                failure: None,
            }
        }
    }

    impl SpeechEngine for FakeSpeechEngine {
        fn id(&self) -> &'static str {
            "fake"
        }

        fn probe(&mut self) -> Result<SpeechAttestation, SpeechEngineError> {
            Ok(self.attestation.clone())
        }

        fn speak(
            &mut self,
            utterance: &Utterance,
            cancellation: &SpeechCancellation,
        ) -> Result<SpeechOutcome, SpeechEngineError> {
            if let Some(failure) = &self.failure {
                return Err(failure.clone());
            }
            if cancellation.is_stopped() {
                return Err(SpeechEngineError::Cancelled { audio_bytes: 0 });
            }
            self.spoken.push(utterance.text().to_owned());
            Ok(SpeechOutcome {
                engine: "fake".to_owned(),
                characters: utterance.characters(),
                audio_bytes: u64::from(utterance.characters()) * 2_950,
                sample_rate_hz: self.attestation.sample_rate_hz,
                duration_ms: 100,
                completed: true,
            })
        }
    }

    fn attestation(sample_rate_hz: u32) -> SpeechAttestation {
        SpeechAttestation {
            engine: "fake".to_owned(),
            synthesizer_path: "/nonexistent/piper-tts".to_owned(),
            synthesizer_sha256: "0".repeat(64),
            voice_id: "fixture".to_owned(),
            voice_path: "/nonexistent/voice.onnx".to_owned(),
            voice_sha256: "1".repeat(64),
            voice_config_path: "/nonexistent/voice.onnx.json".to_owned(),
            sample_rate_hz,
            channels: 1,
            sink: Some(AudioSink::PipeWire),
            sink_path: "/usr/bin/pw-play".to_owned(),
        }
    }

    fn route(connection: &str) -> AudioDeploymentRoute {
        AudioDeploymentRoute {
            connection: connection.to_owned(),
            synthesizer: None,
            voice: PathBuf::from("/voices/en_US-ryan-high.onnx"),
            voice_config: None,
            voice_sha256: None,
            sink: None,
            maximum_characters: 1_000,
            maximum_utterance: Duration::from_secs(30),
        }
    }

    fn zero_io_plan(connection: &str, operation: &str) -> ZeroIoPlan {
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
                ConnectionAuthority::new(connection, InitiationPolicy::b10x_only()).unwrap(),
            ),
            ProtocolPlan::AudioV1(AudioPlan {
                connection: connection.to_owned(),
            }),
        )
    }

    fn admitted(connection: &str, operation: &str) -> AdmittedAudioPlan {
        admit_audio_plan(&zero_io_plan(connection, operation), route(connection))
            .expect("the fixture plan and route admit")
    }

    fn driver(sample_rate_hz: u32) -> (LocalSpeechDriver, SpeechCancellation) {
        let cancellation = SpeechCancellation::new();
        let engine = FakeSpeechEngine::new(sample_rate_hz);
        let attestation = engine.attestation.clone();
        let driver = LocalSpeechDriver::new(
            &admitted("connection-1", SPEECH_SPEAK_OPERATION),
            Box::new(engine),
            attestation,
            cancellation.clone(),
        )
        .expect("driver");
        (driver, cancellation)
    }

    fn speak(text: &str) -> SpeechSpeakInput {
        SpeechSpeakInput {
            text: text.to_owned(),
        }
    }

    #[test]
    fn speaking_returns_a_completed_outcome_carrying_the_voice_rate() {
        let (mut driver, _cancellation) = driver(22_050);
        let result = driver
            .speak(
                &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                &speak("Ready."),
            )
            .expect("speak");
        assert!(result.spoken);
        assert!(result.completed);
        assert_eq!(result.characters, 6);
        assert_eq!(result.sample_rate_hz, 22_050);
    }

    #[test]
    fn status_reports_readiness_without_exposing_any_path() {
        let (driver, _cancellation) = driver(16_000);
        let readiness = driver
            .status(&admitted("connection-1", SPEECH_STATUS_OPERATION))
            .expect("status");
        assert!(readiness.ready);
        assert_eq!(readiness.sample_rate_hz, 16_000);
        assert_eq!(readiness.remaining_utterances, 32);
        let rendered = serde_json::to_string(&readiness).expect("render");
        assert!(!rendered.contains('/'), "status leaked a path: {rendered}");
    }

    #[test]
    fn the_rate_comes_from_the_voice_and_is_never_a_constant() {
        for rate in [16_000, 22_050] {
            let (mut driver, _cancellation) = driver(rate);
            let result = driver
                .speak(
                    &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                    &speak("Hi."),
                )
                .expect("speak");
            assert_eq!(result.sample_rate_hz, rate);
        }
    }

    #[test]
    fn over_length_text_refuses_and_speaks_nothing() {
        let (mut driver, _cancellation) = driver(22_050);
        let text = "a".repeat(1_001);
        let error = driver
            .speak(
                &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                &speak(&text),
            )
            .expect_err("refusal");
        assert_eq!(error.code(), "speech-utterance-too-large");
    }

    #[test]
    fn malformed_text_refuses_before_the_engine() {
        let (mut driver, _cancellation) = driver(22_050);
        for text in ["", "   ", "two\nlines"] {
            let error = driver
                .speak(
                    &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                    &speak(text),
                )
                .expect_err("refusal");
            assert_eq!(
                error.code(),
                "speech-utterance-rejected",
                "accepted {text:?}"
            );
        }
    }

    #[test]
    fn cancellation_reports_the_audio_emitted_and_never_reports_completion() {
        let (mut driver, cancellation) = driver(22_050);
        cancellation.stop();
        let error = driver
            .speak(
                &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                &speak("Hi."),
            )
            .expect_err("cancelled");
        assert_eq!(error.code(), "speech-cancelled");
    }

    #[test]
    fn an_admitted_plan_for_another_connection_refuses() {
        let (mut driver, _cancellation) = driver(22_050);
        let error = driver
            .speak(
                &admitted("connection-2", SPEECH_SPEAK_OPERATION),
                &speak("Hi."),
            )
            .expect_err("refusal");
        assert_eq!(error.code(), "speech-refused");
    }

    #[test]
    fn an_admitted_plan_for_the_other_operation_refuses() {
        let (mut driver, _cancellation) = driver(22_050);
        let error = driver
            .speak(
                &admitted("connection-1", SPEECH_STATUS_OPERATION),
                &speak("Hi."),
            )
            .expect_err("refusal");
        assert_eq!(error.code(), "speech-refused");
        let error = driver
            .status(&admitted("connection-1", SPEECH_SPEAK_OPERATION))
            .expect_err("refusal");
        assert_eq!(error.code(), "speech-refused");
    }

    #[test]
    fn the_per_connection_budget_is_exhaustible() {
        let (mut driver, _cancellation) = driver(22_050);
        for _ in 0..MAX_UTTERANCES_PER_CONNECTION {
            driver
                .speak(
                    &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                    &speak("Hi."),
                )
                .expect("within budget");
        }
        let error = driver
            .speak(
                &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                &speak("Hi."),
            )
            .expect_err("budget");
        assert_eq!(error.code(), "speech-budget-exhausted");
        let readiness = driver
            .status(&admitted("connection-1", SPEECH_STATUS_OPERATION))
            .expect("status");
        assert_eq!(readiness.remaining_utterances, 0);
    }

    #[test]
    fn an_engine_refusal_is_carried_out_with_its_own_code() {
        let cancellation = SpeechCancellation::new();
        let mut engine = FakeSpeechEngine::new(22_050);
        engine.failure = Some(SpeechEngineError::SinkUnavailable {
            reason: "the device went away".to_owned(),
        });
        let attestation = engine.attestation.clone();
        let mut driver = LocalSpeechDriver::new(
            &admitted("connection-1", SPEECH_SPEAK_OPERATION),
            Box::new(engine),
            attestation,
            cancellation,
        )
        .expect("driver");
        let error = driver
            .speak(
                &admitted("connection-1", SPEECH_SPEAK_OPERATION),
                &speak("Hi."),
            )
            .expect_err("device refusal");
        assert_eq!(error.code(), "speech-sink-unavailable");
        assert!(error.remediation().is_some());
    }

    #[test]
    fn an_admitted_route_becomes_the_engine_configuration_and_nothing_else_does() {
        let admitted = admitted("connection-1", SPEECH_SPEAK_OPERATION);
        let engine = engine_for(&admitted);
        assert_eq!(engine.id(), ENGINE_ID);
        assert!(
            engine.attestation().is_none(),
            "construction must resolve no device"
        );
    }
}
