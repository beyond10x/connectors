# driver-audio

The closed built-in `audio_v1` protocol driver: **this machine's sound device, in both directions,
and nothing else**. The only Connectors crate allowed to reach the local sound stack and to spawn the
processes that feed it.

It knows about PipeWire, PulseAudio and ALSA. It knows nothing about speech — no voice, no
synthesizer, no transcript, no recognizer. Those live in `driver-speech`, which reaches a device only
through `domain::audio::AudioDevice`.

## Why the driver is `audio_v1`, and why speech is a different crate

The `protocol_driver` axis answers *which closed implementation speaks to the external system*. The
external system is the machine's sound stack, which is this crate. Speech synthesis and recognition
are **transformations that produce or consume PCM**: they reach no external system of their own, so
they are not drivers and do not get an axis word.

| Capability | Lives in |
|---|---|
| playback to a speaker | here, `src/process.rs` |
| capture from a microphone | here, `src/process.rs` |
| a host with neither | here, `src/null.rs` |
| speech synthesis (TTS) | `driver-speech` |
| speech recognition (STT) | `driver-speech` |
| carrying call media to a device | a binding over `AudioDevice` |

They were one crate until the split. That arrangement was already untrue in one direction — the
synthesizer spawned `pw-play` itself, so "the only crate that touches the sound stack" was not the
crate that claimed it — and speech **recognition** would have made it untrue a second time.

**Device enumeration and selection are reserved, not implemented**: when they arrive, a caller may
still select only an opaque Connection-owned alias — the discipline `sip.dial`'s `target` uses —
because the five-axis model forbids a caller selecting a destination.

## The port is the deployment seam

`ProcessAudioDevice` is what a workstation binds: real output, real input, resolved from the stack
this machine actually has. `NullAudioDevice` is what a server binds: the same contract, paced to real
time, with no sound.

**Nothing above the port branches on which one is bound.** That is the point — a headless deployment
is a configuration, not a disabled code path. `local_device()` picks one and records which in
`AudioDevice::description()`, so evidence says where the audio went; a host with no stack reports an
absent one rather than naming a family it does not have.

The null device is **paced**, not a no-op. Playback and capture are the only backpressure in the
audio path. Remove the wait and a server does not "run without audio" — it burns a core spinning, and
every timing-dependent behaviour above it stops matching the workstation it was developed on.

The port is **synchronous**: every implementation is a pipe to a subprocess or a memory buffer, and
both block. An async caller runs it on a blocking task rather than making a whole synthesis path
async for the sake of a file descriptor.

## Three silent failure modes this crate exists to prevent

1. **A resolved path is never canonicalized.** `pw-play` and `paplay` are multi-call binaries that
   select their behavior from `argv[0]`; running the `pw-cat`/`pacat` symlink target instead exits
   zero and produces no audio.
   `device::tests::a_multi_call_sink_symlink_is_executed_under_its_own_name` pins it.
2. **Recorders are forced out of their container format.** `parecord` and `arecord` both default to
   writing a WAV header. Read as samples, those 44 bytes are a click followed by permanently
   misaligned frames — noise, never an error.
   `device::tests::every_recorder_is_forced_out_of_its_container_format` pins it.
3. **Capture is resolved separately from playback, and pinned to the same family.** A machine can
   have a working speaker and no microphone. Deriving one direction from the other would make such a
   host claim a capability it does not have.
   `device::tests::capture_and_playback_are_different_executables_per_stack` pins it.

## Tests

No test opens a real device or requires one to be present. `NullAudioDevice` is covered directly;
`ProcessAudioDevice` tests skip rather than fail on a host with no stack, because absence is a valid
deployment rather than a broken one.

The crate is an intentionally nested workspace with its own lock, like `driver-sip`: device- and
process-capable code stays out of the deterministic catalogue-compiler workspace.
