//! The real device: this machine's sound stack, driven as a subprocess in both directions.
//!
//! One family is resolved once, at construction, and **both directions come from that same
//! family**. Mixing them — capturing through PipeWire while playing through ALSA — would work often
//! enough to ship and then fail on the machine where the two disagree about which card is default.
//!
//! Every stream here is a pipe to a process. That is why the port is synchronous: `write` blocking
//! is not a deficiency, it *is* the backpressure that keeps a producer at real time.

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use domain::audio::{
    AudioCapture, AudioDevice, AudioDeviceDescription, AudioDeviceError, AudioPlayback, AudioSink,
    PcmFormat,
};

use crate::device::{
    capture_arguments, capture_executable, discover_executable, resolve_sink, sink_arguments,
    sink_executable,
};

/// This machine's sound stack.
pub struct ProcessAudioDevice {
    stack: AudioSink,
    playback_path: PathBuf,
}

impl ProcessAudioDevice {
    /// Resolve the stack family this machine actually has.
    ///
    /// Eager on purpose: a device that cannot name what it is cannot be recorded as evidence, and a
    /// deployment with no stack is supposed to bind [`crate::null::NullAudioDevice`] instead of
    /// holding a device that fails at every use.
    ///
    /// # Errors
    ///
    /// [`AudioDeviceError::Unavailable`] when no candidate stack is on `PATH`.
    pub fn resolve(explicit: Option<AudioSink>) -> Result<Self, AudioDeviceError> {
        let (stack, playback_path) =
            resolve_sink(explicit).ok_or_else(|| AudioDeviceError::Unavailable {
                reason: explicit.map_or_else(
                    || "no PipeWire, PulseAudio or ALSA playback command is on PATH".to_owned(),
                    |sink| format!("`{}` is not on PATH", sink_executable(sink)),
                ),
            })?;
        Ok(Self {
            stack,
            playback_path,
        })
    }

    /// Mono is the only profile the stack arguments encode, so anything else is refused by name
    /// rather than played at the wrong channel count.
    fn admit(format: PcmFormat) -> Result<(), AudioDeviceError> {
        if format.channels != 1 {
            return Err(AudioDeviceError::Unavailable {
                reason: format!(
                    "the local stack profile is mono; {} channels were requested",
                    format.channels
                ),
            });
        }
        Ok(())
    }
}

impl AudioDevice for ProcessAudioDevice {
    fn description(&self) -> AudioDeviceDescription {
        AudioDeviceDescription {
            stack: Some(self.stack),
            path: self.playback_path.display().to_string(),
        }
    }

    fn open_playback(&self, format: PcmFormat) -> Result<Box<dyn AudioPlayback>, AudioDeviceError> {
        Self::admit(format)?;
        let mut child = Command::new(&self.playback_path)
            .args(sink_arguments(self.stack, format.sample_rate_hz))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| AudioDeviceError::Unavailable {
                reason: format!("`{}`: {error}", self.playback_path.display()),
            })?;
        let stdin = child.stdin.take().ok_or_else(|| AudioDeviceError::Failed {
            reason: "the playback command's stdin was not captured".to_owned(),
        })?;
        Ok(Box::new(ProcessPlayback {
            child,
            stdin: Some(stdin),
        }))
    }

    fn open_capture(&self, format: PcmFormat) -> Result<Box<dyn AudioCapture>, AudioDeviceError> {
        Self::admit(format)?;
        // Pinned to the resolved family rather than probed again: a machine with a speaker and no
        // microphone must refuse here, not silently record through a different stack.
        let name = capture_executable(self.stack);
        let path = discover_executable(name).ok_or_else(|| AudioDeviceError::Unavailable {
            reason: format!("`{name}` is not on PATH"),
        })?;
        let mut child = Command::new(&path)
            .args(capture_arguments(self.stack, format.sample_rate_hz))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| AudioDeviceError::Unavailable {
                reason: format!("`{}`: {error}", path.display()),
            })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AudioDeviceError::Failed {
                reason: "the capture command's stdout was not captured".to_owned(),
            })?;
        Ok(Box::new(ProcessCapture {
            child,
            stdout: Some(stdout),
        }))
    }
}

/// PCM on its way into a playback command's stdin.
struct ProcessPlayback {
    child: Child,
    /// Dropped by `finish` to signal end-of-stream, which is the only way the command knows to
    /// drain and exit.
    stdin: Option<ChildStdin>,
}

impl AudioPlayback for ProcessPlayback {
    fn write(&mut self, pcm: &[u8]) -> Result<(), AudioDeviceError> {
        let stdin = self.stdin.as_mut().ok_or(AudioDeviceError::Closed)?;
        stdin.write_all(pcm).map_err(|error| {
            if error.kind() == std::io::ErrorKind::BrokenPipe {
                // The command exited underneath us. That is the stream ending, not a fault: a
                // caller that keeps writing gets a closed stream rather than a fabricated failure.
                AudioDeviceError::Closed
            } else {
                AudioDeviceError::Failed {
                    reason: format!("writing to the playback command failed: {error}"),
                }
            }
        })
    }

    fn finish(mut self: Box<Self>) -> Result<(), AudioDeviceError> {
        // Closing stdin is what makes the command drain and exit; waiting before it is closed
        // deadlocks.
        drop(self.stdin.take());
        let status = self
            .child
            .wait()
            .map_err(|error| AudioDeviceError::Failed {
                reason: format!("waiting for the playback command failed: {error}"),
            })?;
        if status.success() {
            return Ok(());
        }
        Err(AudioDeviceError::Failed {
            reason: "the playback command exited with a failure status".to_owned(),
        })
    }
}

/// PCM arriving from a capture command's stdout.
struct ProcessCapture {
    child: Child,
    stdout: Option<ChildStdout>,
}

impl AudioCapture for ProcessCapture {
    fn read(&mut self, into: &mut [u8]) -> Result<usize, AudioDeviceError> {
        let Some(stdout) = self.stdout.as_mut() else {
            return Ok(0);
        };
        stdout.read(into).map_err(|error| AudioDeviceError::Failed {
            reason: format!("reading from the capture command failed: {error}"),
        })
    }

    fn stop(mut self: Box<Self>) -> Result<(), AudioDeviceError> {
        // A recorder never ends on its own — it is killed. Dropping the pipe first would leave it
        // running until it filled a buffer nobody drains.
        drop(self.stdout.take());
        let _ = self.child.kill();
        self.child
            .wait()
            .map(|_| ())
            .map_err(|error| AudioDeviceError::Failed {
                reason: format!("waiting for the capture command failed: {error}"),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_with_no_named_stack_refuses_by_name() {
        let error = ProcessAudioDevice::resolve(Some(AudioSink::Alsa))
            .err()
            .filter(|_| crate::device::discover_executable("aplay").is_none());
        if let Some(error) = error {
            assert!(matches!(error, AudioDeviceError::Unavailable { .. }));
        }
    }

    #[test]
    fn a_multi_channel_request_is_refused_rather_than_played_wrong() {
        // The stack arguments hardcode `--channels=1`. Accepting stereo here would play it at half
        // speed with the channels interleaved into each other, and exit zero.
        let error = ProcessAudioDevice::admit(PcmFormat {
            sample_rate_hz: 8_000,
            channels: 2,
        })
        .expect_err("stereo is not the local profile");
        assert!(
            error.to_string().contains("mono"),
            "the refusal must name the profile: {error}"
        );
        ProcessAudioDevice::admit(PcmFormat::mono(8_000)).expect("mono is the profile");
    }

    #[test]
    fn the_description_names_the_resolved_stack_and_its_exact_path() {
        let Ok(device) = ProcessAudioDevice::resolve(None) else {
            return; // No stack on this machine; the null device is what such a host binds.
        };
        let description = device.description();
        assert!(
            description.stack.is_some(),
            "a resolved process device knows its family"
        );
        assert!(
            !description.path.is_empty(),
            "evidence records the exact executable"
        );
    }
}
