//! The one shipped [`SpeechEngine`]: a local neural synthesizer driven as a subprocess.
//!
//! This is the *speech* implementation. It produces PCM and hands it to a
//! [`domain::audio::AudioDevice`] it was given. It resolves no sink and spawns no playback command:
//! `driver-audio` is the only code that knows a sound stack exists, and this crate cannot tell
//! whether the device it writes to has a speaker behind it.
//!
//! The synthesizer product is named here, in its configuration values, and in the recorded engine
//! identity. It is named in no `domain`, `protocol`, or `service` type. A network-backed engine, if
//! one is ever proposed, arrives as a second implementation of the same port and never as a
//! fallback from this one.
//!
//! The synthesizer performs its own grapheme-to-phoneme conversion using a library it bundles,
//! which happens to be espeak-ng. That is an internal stage of the neural pipeline: no espeak
//! voice produces audio here, this repository executes no espeak binary, and observing that
//! library name in a recorded attestation is not evidence of a fallback path.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use domain::audio::{AudioDevice, AudioPlayback, PcmFormat};
use driver_audio::{discover_executable, executable_at};
use sha2::{Digest, Sha256};

use crate::speech::{
    SpeechAttestation, SpeechCancellation, SpeechEngine, SpeechEngineError, SpeechOutcome,
    Utterance,
};

/// The engine identity recorded in evidence.
pub const ENGINE_ID: &str = "local-subprocess:piper";

/// The only executable name auto-discovered on `PATH`.
///
/// The bare name `piper` is deliberately not a candidate: on Arch it belongs to an unrelated
/// gaming-mouse configuration tool, and auto-selecting it would execute the wrong program. A
/// synthesizer at that name is reachable only through an explicit absolute path.
pub const SYNTHESIZER_EXECUTABLE: &str = "piper-tts";

const PUMP_CHUNK_BYTES: usize = 8 * 1024;
const DIGEST_CHUNK_BYTES: usize = 1024 * 1024;
const MIN_SAMPLE_RATE_HZ: u32 = 8_000;
const MAX_SAMPLE_RATE_HZ: u32 = 48_000;

/// Everything the deployment selected, before any component has been resolved.
///
/// This is built from an admitted route by [`crate::engine_for`]; it is never built from caller
/// input, and nothing here is nameable across the operation boundary.
#[derive(Debug, Clone)]
pub struct PiperConfig {
    /// An explicit absolute synthesizer path. When absent, `piper-tts` is discovered on `PATH`.
    pub synthesizer: Option<PathBuf>,
    /// The absolute voice model path. There is no default: a voice is always chosen explicitly.
    pub voice: PathBuf,
    /// The voice configuration. When absent, the model path plus `.json` is used.
    pub voice_config: Option<PathBuf>,
    /// An optional digest pin. The digest is always computed and recorded; a pin additionally
    /// refuses on mismatch.
    pub voice_sha256: Option<String>,
    /// The wall-clock bound on one utterance.
    pub maximum_utterance: Duration,
}

/// A local neural speech synthesizer driven over stdin/stdout.
///
/// It produces PCM and hands it to a device it was given. It resolves no sink, spawns no playback
/// command, and cannot tell whether the device it writes to has a speaker behind it.
pub struct PiperSpeechEngine {
    config: PiperConfig,
    device: Arc<dyn AudioDevice>,
    attestation: Option<SpeechAttestation>,
}

impl PiperSpeechEngine {
    /// Bind one deployment-selected configuration to the device it will play through.
    ///
    /// Nothing is resolved until `probe`.
    #[must_use]
    pub fn new(config: PiperConfig, device: Arc<dyn AudioDevice>) -> Self {
        Self {
            config,
            device,
            attestation: None,
        }
    }

    /// The attestation from the most recent successful probe, if any.
    #[must_use]
    pub const fn attestation(&self) -> Option<&SpeechAttestation> {
        self.attestation.as_ref()
    }

    fn resolve_synthesizer(&self) -> Result<PathBuf, SpeechEngineError> {
        if let Some(explicit) = &self.config.synthesizer {
            return executable_at(explicit).ok_or_else(|| {
                SpeechEngineError::SynthesizerUnavailable {
                    path: explicit.display().to_string(),
                    reason: "not an executable regular file".to_owned(),
                }
            });
        }
        discover_executable(SYNTHESIZER_EXECUTABLE).ok_or_else(|| {
            SpeechEngineError::SynthesizerUnavailable {
                path: SYNTHESIZER_EXECUTABLE.to_owned(),
                reason: "not found in any absolute PATH directory".to_owned(),
            }
        })
    }

    fn voice_config_path(&self) -> PathBuf {
        self.config.voice_config.clone().unwrap_or_else(|| {
            let mut value = self.config.voice.clone().into_os_string();
            value.push(".json");
            PathBuf::from(value)
        })
    }
}

impl SpeechEngine for PiperSpeechEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    fn probe(&mut self) -> Result<SpeechAttestation, SpeechEngineError> {
        let synthesizer = self.resolve_synthesizer()?;
        let voice = &self.config.voice;
        if !readable_file(voice) {
            return Err(SpeechEngineError::VoiceUnavailable {
                path: voice.display().to_string(),
                reason: "not a readable regular file".to_owned(),
            });
        }
        let voice_config = self.voice_config_path();
        let (sample_rate_hz, voice_id) = read_voice_configuration(&voice_config)?;
        let voice_sha256 =
            file_digest(voice).map_err(|reason| SpeechEngineError::VoiceUnavailable {
                path: voice.display().to_string(),
                reason,
            })?;
        if let Some(expected) = &self.config.voice_sha256 {
            if !expected.eq_ignore_ascii_case(&voice_sha256) {
                return Err(SpeechEngineError::VoiceDigestMismatch {
                    expected: expected.clone(),
                    actual: voice_sha256,
                });
            }
        }
        let synthesizer_sha256 = file_digest(&synthesizer).map_err(|reason| {
            SpeechEngineError::SynthesizerUnavailable {
                path: synthesizer.display().to_string(),
                reason,
            }
        })?;
        // The device is asked what it is rather than probed for: on a host with no sound stack this
        // records an absent one and synthesis still runs, which is the headless case working rather
        // than a refusal.
        let device = self.device.description();
        let attestation = SpeechAttestation {
            engine: ENGINE_ID.to_owned(),
            synthesizer_path: synthesizer.display().to_string(),
            synthesizer_sha256,
            voice_id,
            voice_path: voice.display().to_string(),
            voice_sha256,
            voice_config_path: voice_config.display().to_string(),
            sample_rate_hz,
            channels: 1,
            sink: device.stack,
            sink_path: device.path,
        };
        self.attestation = Some(attestation.clone());
        Ok(attestation)
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        cancellation: &SpeechCancellation,
    ) -> Result<SpeechOutcome, SpeechEngineError> {
        // A device can disappear between preparation and use, so every utterance re-resolves.
        let attestation = self.probe()?;
        let started = Instant::now();

        let mut synthesizer = Command::new(&attestation.synthesizer_path)
            .arg("-q")
            .arg("-m")
            .arg(&attestation.voice_path)
            .arg("-c")
            .arg(&attestation.voice_config_path)
            .arg("--output_raw")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| SpeechEngineError::SynthesizerUnavailable {
                path: attestation.synthesizer_path.clone(),
                reason: error.to_string(),
            })?;

        // The voice's own rate, never a constant — see `read_voice_configuration`.
        let mut playback = match self
            .device
            .open_playback(PcmFormat::mono(attestation.sample_rate_hz))
        {
            Ok(playback) => playback,
            Err(error) => {
                terminate(&mut synthesizer);
                return Err(SpeechEngineError::SinkUnavailable {
                    reason: error.to_string(),
                });
            }
        };

        let write_result = synthesizer.stdin.take().map_or_else(
            || Err("synthesizer stdin was not captured".to_owned()),
            |stdin| {
                let mut stdin = BufWriter::new(stdin);
                stdin
                    .write_all(utterance.text().as_bytes())
                    .and_then(|()| stdin.write_all(b"\n"))
                    .and_then(|()| stdin.flush())
                    .map_err(|error| error.to_string())
            },
        );
        if let Err(reason) = write_result {
            terminate(&mut synthesizer);
            // Dropped rather than drained: the tail must not play after a refusal.
            drop(playback);
            return Err(SpeechEngineError::Refused { reason });
        }

        let outcome = pump(
            &mut synthesizer,
            playback.as_mut(),
            cancellation,
            started,
            self.config.maximum_utterance,
        );
        let audio_bytes = match outcome {
            Ok(bytes) => bytes,
            Err(error) => {
                terminate(&mut synthesizer);
                drop(playback);
                return Err(error);
            }
        };

        // Draining first lets the device finish the tail; the synthesizer is already done.
        let synthesizer_status = synthesizer.wait().ok();
        let drained = playback.finish();
        if synthesizer_status.is_some_and(|status| !status.success()) {
            return Err(SpeechEngineError::Refused {
                reason: "synthesizer exited with a failure status".to_owned(),
            });
        }
        drained.map_err(|error| SpeechEngineError::SinkUnavailable {
            reason: error.to_string(),
        })?;

        Ok(SpeechOutcome {
            engine: ENGINE_ID.to_owned(),
            characters: utterance.characters(),
            audio_bytes,
            sample_rate_hz: attestation.sample_rate_hz,
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            completed: true,
        })
    }
}

/// Moves audio from the synthesizer into the device while remaining cancellable.
///
/// Letting the synthesizer write to a device pipe directly would be simpler, but then neither the
/// byte count nor a cancellation could be observed until the utterance had already been played.
///
/// Cancellation and the wall-clock bound are observed between chunks, so the worst-case latency is
/// one blocking read: sub-second once audio is flowing, and about as long as model loading takes
/// before the first chunk arrives.
fn pump(
    synthesizer: &mut Child,
    destination: &mut dyn AudioPlayback,
    cancellation: &SpeechCancellation,
    started: Instant,
    maximum: Duration,
) -> Result<u64, SpeechEngineError> {
    let mut source = synthesizer
        .stdout
        .take()
        .ok_or_else(|| SpeechEngineError::Refused {
            reason: "synthesizer stdout was not captured".to_owned(),
        })?;
    let mut buffer = vec![0_u8; PUMP_CHUNK_BYTES];
    let mut audio_bytes = 0_u64;
    loop {
        if cancellation.is_stopped() {
            return Err(SpeechEngineError::Cancelled { audio_bytes });
        }
        if started.elapsed() >= maximum {
            return Err(SpeechEngineError::TimedOut {
                audio_bytes,
                maximum_ms: u64::try_from(maximum.as_millis()).unwrap_or(u64::MAX),
            });
        }
        let read = source
            .read(&mut buffer)
            .map_err(|error| SpeechEngineError::Refused {
                reason: format!("reading synthesized audio failed: {error}"),
            })?;
        if read == 0 {
            break;
        }
        destination
            .write(&buffer[..read])
            .map_err(|error| SpeechEngineError::Refused {
                reason: format!("writing to the audio device failed: {error}"),
            })?;
        audio_bytes = audio_bytes.saturating_add(read as u64);
    }
    Ok(audio_bytes)
}

/// Stops production first, then playback, so buffered audio is discarded rather than played out.
fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Reads the voice's own sample rate and identity.
///
/// The rate is never a constant: shipped voices differ, and reproducing a 16 kHz voice at 22.05 kHz
/// renders it at the wrong pitch.
fn read_voice_configuration(path: &Path) -> Result<(u32, String), SpeechEngineError> {
    let invalid = |reason: String| SpeechEngineError::VoiceConfigurationInvalid {
        path: path.display().to_string(),
        reason,
    };
    let bytes = fs::read(path).map_err(|error| invalid(error.to_string()))?;
    let document: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
    let sample_rate = document
        .get("audio")
        .and_then(|audio| audio.get("sample_rate"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| invalid("`audio.sample_rate` is absent or not a number".to_owned()))?;
    let sample_rate =
        u32::try_from(sample_rate).map_err(|_| invalid(format!("sample rate {sample_rate} Hz")))?;
    if !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate) {
        return Err(invalid(format!(
            "sample rate {sample_rate} Hz is outside {MIN_SAMPLE_RATE_HZ}..={MAX_SAMPLE_RATE_HZ}"
        )));
    }
    let voice_id = document
        .get("dataset")
        .and_then(serde_json::Value::as_str)
        .map_or_else(
            || {
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("voice")
                    .to_owned()
            },
            str::to_owned,
        );
    Ok((sample_rate, voice_id))
}

/// Streams a file through SHA-256 so a 120 MB voice never enters memory whole.
fn file_digest(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; DIGEST_CHUNK_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(crate::hex(&hasher.finalize()))
}

fn readable_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The device every test here runs against: no sound stack, no subprocess, no sound.
    ///
    /// Before the split this was impossible — the engine resolved `pw-play` itself, so synthesis on
    /// a host without one exercised a different path than the one that ships.
    fn headless() -> Arc<dyn AudioDevice> {
        Arc::new(driver_audio::NullAudioDevice::new())
    }

    fn write(directory: &Path, name: &str, body: &str) -> PathBuf {
        let path = directory.join(name);
        let mut file = File::create(&path).expect("fixture file");
        file.write_all(body.as_bytes()).expect("fixture body");
        path
    }

    #[test]
    fn voice_configuration_supplies_the_rate_rather_than_a_constant() {
        let directory = tempfile::tempdir().expect("temp dir");
        let low = write(
            directory.path(),
            "low.json",
            r#"{"audio":{"sample_rate":16000},"dataset":"alan"}"#,
        );
        let high = write(
            directory.path(),
            "high.json",
            r#"{"audio":{"sample_rate":22050},"dataset":"ryan"}"#,
        );
        assert_eq!(
            read_voice_configuration(&low).expect("low voice"),
            (16_000, "alan".to_owned())
        );
        assert_eq!(
            read_voice_configuration(&high).expect("high voice"),
            (22_050, "ryan".to_owned())
        );
    }

    #[test]
    fn absent_zero_and_absurd_sample_rates_refuse() {
        let directory = tempfile::tempdir().expect("temp dir");
        for (name, body) in [
            ("absent.json", r#"{"audio":{}}"#),
            ("zero.json", r#"{"audio":{"sample_rate":0}}"#),
            ("absurd.json", r#"{"audio":{"sample_rate":1000000}}"#),
            ("garbage.json", "not json"),
        ] {
            let path = write(directory.path(), name, body);
            assert!(
                matches!(
                    read_voice_configuration(&path),
                    Err(SpeechEngineError::VoiceConfigurationInvalid { .. })
                ),
                "{name} was accepted"
            );
        }
    }

    #[test]
    fn a_missing_voice_configuration_refuses_by_path() {
        let error = read_voice_configuration(Path::new("/nonexistent/voice.onnx.json"))
            .expect_err("absent configuration");
        match error {
            SpeechEngineError::VoiceConfigurationInvalid { path, .. } => {
                assert_eq!(path, "/nonexistent/voice.onnx.json");
            }
            other => panic!("unexpected refusal: {other}"),
        }
    }

    #[test]
    fn the_bare_gaming_mouse_name_is_never_a_discovery_candidate() {
        assert_eq!(SYNTHESIZER_EXECUTABLE, "piper-tts");
        assert_ne!(SYNTHESIZER_EXECUTABLE, "piper");
    }

    #[test]
    fn an_absent_synthesizer_refuses_with_its_path_and_remediation() {
        let engine = PiperSpeechEngine::new(
            PiperConfig {
                synthesizer: Some(PathBuf::from("/nonexistent/piper-tts")),
                voice: PathBuf::from("/nonexistent/voice.onnx"),
                voice_config: None,
                voice_sha256: None,
                maximum_utterance: Duration::from_secs(1),
            },
            headless(),
        );
        let error = engine
            .resolve_synthesizer()
            .expect_err("absent synthesizer");
        match &error {
            SpeechEngineError::SynthesizerUnavailable { path, .. } => {
                assert_eq!(path, "/nonexistent/piper-tts");
            }
            other => panic!("unexpected refusal: {other}"),
        }
        assert!(error
            .remediation()
            .is_some_and(|value| value.contains(SYNTHESIZER_EXECUTABLE)));
    }

    #[test]
    fn voice_configuration_defaults_to_the_model_path_plus_json() {
        let engine = PiperSpeechEngine::new(
            PiperConfig {
                synthesizer: None,
                voice: PathBuf::from("/voices/en_US-ryan-high.onnx"),
                voice_config: None,
                voice_sha256: None,
                maximum_utterance: Duration::from_secs(1),
            },
            headless(),
        );
        assert_eq!(
            engine.voice_config_path(),
            PathBuf::from("/voices/en_US-ryan-high.onnx.json")
        );
    }

    #[test]
    fn file_digest_streams_and_matches_a_known_value() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = write(directory.path(), "body", "abc");
        assert_eq!(
            file_digest(&path).expect("digest"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}

/// Live checks against the real local engine.
///
/// These are `#[ignore]`d: they need an installed synthesizer, a voice model, and an audio device,
/// none of which the gate may assume. Run them deliberately:
///
/// ```text
/// B10X_AUDIO_VOICE=/absolute/path/voice.onnx \
///   cargo test --manifest-path crates/driver-speech/Cargo.toml -- --ignored --nocapture
/// ```
#[cfg(test)]
mod live {
    use super::*;

    /// The real stack, because being audible is the whole point of this suite.
    fn speaker() -> Arc<dyn AudioDevice> {
        driver_audio::local_device(None)
    }

    fn configured() -> Option<PiperConfig> {
        let voice = std::env::var_os("B10X_AUDIO_VOICE").map(PathBuf::from)?;
        Some(PiperConfig {
            synthesizer: None,
            voice,
            voice_config: None,
            voice_sha256: None,
            maximum_utterance: Duration::from_secs(60),
        })
    }

    #[test]
    #[ignore = "requires an installed synthesizer and voice model"]
    fn probing_resolves_every_component_without_emitting_audio() {
        let Some(config) = configured() else {
            panic!("set B10X_AUDIO_VOICE to an absolute voice path");
        };
        let mut engine = PiperSpeechEngine::new(config, speaker());
        let attestation = engine.probe().expect("probe");
        assert_eq!(attestation.engine, ENGINE_ID);
        assert_eq!(attestation.channels, 1);
        assert!((MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&attestation.sample_rate_hz));
        assert_eq!(attestation.voice_sha256.len(), 64);
        assert_eq!(attestation.synthesizer_sha256.len(), 64);
        println!(
            "engine={} voice={} rate={} sink={:?} at {}",
            attestation.engine,
            attestation.voice_id,
            attestation.sample_rate_hz,
            attestation.sink,
            attestation.sink_path
        );
    }

    #[test]
    #[ignore = "plays audible speech on the local device"]
    fn speaks_one_audible_utterance() {
        let Some(config) = configured() else {
            panic!("set B10X_AUDIO_VOICE to an absolute voice path");
        };
        let mut engine = PiperSpeechEngine::new(config, speaker());
        engine.probe().expect("probe");
        let utterance = Utterance::new("b10x speech is online.", 1_000).expect("utterance");
        let outcome = engine
            .speak(&utterance, &SpeechCancellation::new())
            .expect("speak");
        assert!(outcome.completed);
        assert!(outcome.audio_bytes > 0, "no audio was produced");
        println!(
            "spoke {} characters as {} bytes at {} Hz in {} ms",
            outcome.characters, outcome.audio_bytes, outcome.sample_rate_hz, outcome.duration_ms
        );
    }
}
