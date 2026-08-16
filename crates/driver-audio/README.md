# driver-audio

The closed built-in `audio_v1` protocol driver: the only Connectors crate allowed to reach the local
sound stack and to spawn the processes that feed it. It consumes a non-serializable
`service::AdmittedAudioPlan` and never mints one.

## Why the driver is `audio_v1` and the operations are `speech.*`

The `protocol_driver` axis answers *which closed implementation speaks to the external system*. The
external system is the machine's sound stack — PipeWire, PulseAudio or ALSA — which is `src/device.rs`.
Speech synthesis is a **transformation that produces PCM**, not a protocol: it is `src/speech.rs`
(the neutral port) plus `src/piper.rs` (the one shipped implementation).

| Later capability | New driver? | Where it lands |
|---|---|---|
| speech recognition (STT) | no | a second transformation over the same device |
| microphone capture | no | the read direction of `device` |
| a notification tone | no | no transformation at all — straight to `device` |
| device enumeration and selection | no | `device`, surfaced as operations |

Naming the driver `speech_v1` would have forced a second driver, or a lie, for the last three.
**Device enumeration and selection are reserved, not implemented**: when they arrive, a caller may
still select only an opaque Connection-owned alias — the discipline `sip.dial`'s `target` uses —
because the five-axis model forbids a caller selecting a destination.

## The shipped surface

Two operations, declared in `providers/b10x.toml`: `speech.speak` and `speech.status`. A caller
supplies exactly one bounded `text` string; the synthesizer, voice, sample rate, sink and bounds are
deployment-owned facts on the Connection, resolved after Grant admission. The vendor name appears
only in `src/piper.rs` and in the recorded engine identity — no `domain`, `protocol`, or `service`
type names a synthesizer, a voice, or an executable.

## Two silent failure modes this crate exists to prevent

1. **A resolved sink path is never canonicalized.** `pw-play` and `paplay` are multi-call binaries
   that select their behavior from `argv[0]`; running the `pw-cat`/`pacat` symlink target instead
   exits zero and produces no audio.
   `device::tests::a_multi_call_sink_symlink_is_executed_under_its_own_name` pins it.
2. **The sample rate comes from the voice**, read from its `.onnx.json` `audio.sample_rate`, never
   from a constant. Installed voices differ — 16000 and 22050 both occur — and playing one at the
   other's rate renders it at the wrong pitch.
   `piper::tests::voice_configuration_supplies_the_rate_rather_than_a_constant` pins it.

Discovery admits exactly one synthesizer executable name, `piper-tts`. The bare name `piper` is
deliberately not a candidate: on Arch it is an unrelated gaming-mouse configuration tool. A
synthesizer at that name is reachable only through an explicit absolute path on the Connection's
route.

## Tests

No test spawns a process, opens a device, or requires a voice model. Device behavior is covered
through a fake engine implementing the same port. Two `#[ignore]`d live checks exercise the real
local stack and are run deliberately:

```bash
B10X_AUDIO_VOICE=/absolute/path/voice.onnx \
  cargo test --manifest-path crates/driver-audio/Cargo.toml -- --ignored --nocapture
```

The crate is an intentionally nested workspace with its own lock, like `driver-sip`: device- and
process-capable code stays out of the deterministic catalogue-compiler workspace.
