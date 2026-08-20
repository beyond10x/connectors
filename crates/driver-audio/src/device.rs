//! The device half of `audio_v1`: reaching the local sound stack, and nothing else.
//!
//! This module knows about PipeWire, PulseAudio and ALSA. It knows nothing about speech: no voice,
//! no synthesizer, no transcript. Everything here would be equally correct for playing a
//! notification tone or, later, for capturing a microphone — which is the reason the driver axis
//! word is `audio_v1` and not `speech_v1`.

use std::fs;
use std::path::{Path, PathBuf};

use domain::audio::AudioSink;

/// The executable name a sink family is played through.
///
/// The names belong to the device stacks themselves, not to any vendor this repository integrates.
#[must_use]
pub const fn sink_executable(sink: AudioSink) -> &'static str {
    match sink {
        AudioSink::PipeWire => "pw-play",
        AudioSink::PulseAudio => "paplay",
        AudioSink::Alsa => "aplay",
    }
}

/// The arguments that play signed 16-bit little-endian mono audio at `sample_rate_hz`.
///
/// Every sink is told the rate explicitly because the rate belongs to whatever produced the audio,
/// not to the device.
#[must_use]
pub fn sink_arguments(sink: AudioSink, sample_rate_hz: u32) -> Vec<String> {
    match sink {
        AudioSink::PipeWire => vec![
            "--raw".to_owned(),
            format!("--rate={sample_rate_hz}"),
            "--channels=1".to_owned(),
            "--format=s16".to_owned(),
            "-".to_owned(),
        ],
        AudioSink::PulseAudio => vec![
            "--raw".to_owned(),
            "--format=s16le".to_owned(),
            format!("--rate={sample_rate_hz}"),
            "--channels=1".to_owned(),
        ],
        AudioSink::Alsa => vec![
            "-q".to_owned(),
            "-t".to_owned(),
            "raw".to_owned(),
            "-f".to_owned(),
            "S16_LE".to_owned(),
            "-r".to_owned(),
            sample_rate_hz.to_string(),
            "-c".to_owned(),
            "1".to_owned(),
            "-".to_owned(),
        ],
    }
}

/// The executable a stack family is **captured** through.
///
/// The read direction of the same three stacks. Each family ships its recorder under its own name,
/// so this is a lookup rather than a mode flag on [`sink_executable`].
#[must_use]
pub const fn capture_executable(sink: AudioSink) -> &'static str {
    match sink {
        AudioSink::PipeWire => "pw-record",
        AudioSink::PulseAudio => "parecord",
        AudioSink::Alsa => "arecord",
    }
}

/// The arguments that capture signed 16-bit little-endian mono audio at `sample_rate_hz` to stdout.
///
/// Every recorder is told the rate explicitly, for the same reason playback is: the rate belongs to
/// whatever will consume the audio, not to the device. `--raw` / `-t raw` is load-bearing on all
/// three — `parecord` and `arecord` both default to writing a **WAV header**, and a 44-byte header
/// interpreted as samples is a click followed by permanently offset frame boundaries.
#[must_use]
pub fn capture_arguments(sink: AudioSink, sample_rate_hz: u32) -> Vec<String> {
    match sink {
        AudioSink::PipeWire => vec![
            "--raw".to_owned(),
            format!("--rate={sample_rate_hz}"),
            "--channels=1".to_owned(),
            "--format=s16".to_owned(),
            "-".to_owned(),
        ],
        AudioSink::PulseAudio => vec![
            "--raw".to_owned(),
            "--format=s16le".to_owned(),
            format!("--rate={sample_rate_hz}"),
            "--channels=1".to_owned(),
        ],
        AudioSink::Alsa => vec![
            "-q".to_owned(),
            "-t".to_owned(),
            "raw".to_owned(),
            "-f".to_owned(),
            "S16_LE".to_owned(),
            "-r".to_owned(),
            sample_rate_hz.to_string(),
            "-c".to_owned(),
            "1".to_owned(),
            "-".to_owned(),
        ],
    }
}

/// Resolve the first capture family present on this machine, in the fixed candidate order.
///
/// Separate from [`resolve_sink`] rather than derived from it: a machine can have a working speaker
/// and no microphone, and resolving one direction must not claim the other.
#[must_use]
pub fn resolve_capture(explicit: Option<AudioSink>) -> Option<(AudioSink, PathBuf)> {
    if let Some(sink) = explicit {
        return discover_executable(capture_executable(sink)).map(|path| (sink, path));
    }
    AudioSink::candidates()
        .into_iter()
        .find_map(|sink| discover_executable(capture_executable(sink)).map(|path| (sink, path)))
}

/// Resolve the first sink family present on this machine, in the fixed candidate order.
///
/// Returns the family **and the exact path it was found at**. A caller must execute that path
/// verbatim; see [`executable_at`] for why.
#[must_use]
pub fn resolve_sink(explicit: Option<AudioSink>) -> Option<(AudioSink, PathBuf)> {
    if let Some(sink) = explicit {
        return discover_executable(sink_executable(sink)).map(|path| (sink, path));
    }
    AudioSink::candidates()
        .into_iter()
        .find_map(|sink| discover_executable(sink_executable(sink)).map(|path| (sink, path)))
}

/// Resolves one candidate name against absolute `PATH` directories only.
///
/// No shell, no `which`, no relative directory, and an executable bit is required.
#[must_use]
pub fn discover_executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|directory| directory.is_absolute())
        .find_map(|directory| executable_at(&directory.join(name)))
}

/// Accepts one executable regular file at exactly the path given.
///
/// `metadata` follows symlinks, so a packaged `/usr/bin/x -> /opt/.../x` is accepted. The link is
/// deliberately **not** resolved to its target: `pw-play` and `paplay` are multi-call binaries that
/// select their mode from the name they were invoked as, so executing the canonical
/// `pw-cat`/`pacat` target instead would silently change what the program does — and exit zero
/// while producing no audio.
#[must_use]
pub fn executable_at(path: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return None;
    }
    Some(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sink_is_told_the_rate_explicitly() {
        for sink in AudioSink::candidates() {
            let arguments = sink_arguments(sink, 16_000).join(" ");
            assert!(
                arguments.contains("16000"),
                "{sink:?} dropped the sample rate: {arguments}"
            );
        }
    }

    #[test]
    fn every_recorder_is_told_the_rate_explicitly() {
        for sink in AudioSink::candidates() {
            let arguments = capture_arguments(sink, 8_000).join(" ");
            assert!(
                arguments.contains("8000"),
                "{sink:?} dropped the capture rate: {arguments}"
            );
        }
    }

    #[test]
    fn every_recorder_is_forced_out_of_its_container_format() {
        // `parecord` and `arecord` both default to a WAV header. Forty-four bytes of header read as
        // samples is a click, and then every frame boundary after it is wrong — audible as noise,
        // never as a failure.
        for sink in AudioSink::candidates() {
            let arguments = capture_arguments(sink, 8_000);
            assert!(
                arguments.iter().any(|argument| argument == "--raw")
                    || arguments.windows(2).any(|pair| pair == ["-t", "raw"]),
                "{sink:?} would emit a container header: {arguments:?}"
            );
        }
    }

    #[test]
    fn capture_and_playback_are_different_executables_per_stack() {
        // The read direction is a separate binary per family, not a flag: resolving one and
        // assuming the other is what would make a machine with a speaker claim a microphone.
        for sink in AudioSink::candidates() {
            assert_ne!(capture_executable(sink), sink_executable(sink));
        }
    }

    #[test]
    fn every_sink_executable_is_distinct() {
        let mut seen = Vec::new();
        for sink in AudioSink::candidates() {
            let executable = sink_executable(sink);
            assert!(!seen.contains(&executable), "{executable} appears twice");
            seen.push(executable);
        }
    }

    #[test]
    fn a_multi_call_sink_symlink_is_executed_under_its_own_name() {
        // Executing the canonical target would turn `pw-play` into `pw-cat`, which reads a
        // different default mode and produces no audio.
        let directory = tempfile::tempdir().expect("temp dir");
        let target = directory.path().join("pw-cat");
        std::fs::write(&target, "#!/bin/sh\n").expect("target");
        let mut permissions = std::fs::metadata(&target).expect("meta").permissions();
        {
            use std::os::unix::fs::PermissionsExt as _;
            permissions.set_mode(0o755);
        }
        std::fs::set_permissions(&target, permissions).expect("chmod");
        let link = directory.path().join("pw-play");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        assert_eq!(executable_at(&link), Some(link.clone()));
    }

    #[test]
    fn a_non_executable_file_is_never_a_candidate() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("pw-play");
        std::fs::write(&path, "#!/bin/sh\n").expect("file");
        assert_eq!(executable_at(&path), None);
        assert_eq!(executable_at(directory.path()), None);
    }
}
