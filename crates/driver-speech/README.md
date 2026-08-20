# driver-speech

Local speech, as a **transformation over PCM** rather than a device. Synthesis produces samples;
recognition, when it lands, consumes them.

Neither reaches an external system of its own, and neither may open a sound device: everything
audible here goes through `domain::audio::AudioDevice`, which `driver-audio` implements for a real
stack and for a host that has none. This crate resolves no sink, spawns no playback command, and
cannot tell whether the device it writes to has a speaker behind it.

## Why this is not `driver-audio`

The `protocol_driver` axis answers *which closed implementation speaks to the external system*, and
for audio that system is the machine's sound stack — which is `driver-audio`, and stays the
`audio_v1` driver. Speech is not a protocol, so it gets no axis word.

The two were one crate until the split, and the arrangement was already untrue: the synthesizer
spawned the playback command itself, so the crate that claimed to be "the only code touching the
sound stack" was not the one doing it. Speech **recognition** would have broken it a second time.

Split, each side gets a property worth having:

- a device can be swapped for a headless one without speech knowing;
- speech can be exercised with **no device at all** — every test here runs against
  `NullAudioDevice`, so synthesis tests exercise the shipping path on a machine with no sound stack,
  which was impossible before.

## The shipped surface

Two operations, declared in `providers/b10x.toml`: `speech.speak` and `speech.status`. A caller
supplies exactly one bounded `text` string; the synthesizer, voice, sample rate, device and bounds
are deployment-owned facts on the Connection, resolved after Grant admission. The vendor name appears
only in `src/piper.rs` and in the recorded engine identity — no `domain`, `protocol`, or `service`
type names a synthesizer, a voice, or an executable.

`engine_for` binds this host's device; `engine_with_device` takes one the caller chose, which is the
seam a headless deployment and every test use.

**Recognition (STT) is not implemented.** It belongs here when it arrives, as a second transformation
over the same device. It is named rather than stubbed.

## Two silent failure modes this crate exists to prevent

1. **The sample rate comes from the voice**, read from its `.onnx.json` `audio.sample_rate`, never
   from a constant. Installed voices differ — 16000 and 22050 both occur — and playing one at the
   other's rate renders it at the wrong pitch.
   `piper::tests::voice_configuration_supplies_the_rate_rather_than_a_constant` pins it.
2. **Discovery admits exactly one synthesizer executable name**, `piper-tts`. The bare name `piper`
   is deliberately not a candidate: on Arch it is an unrelated gaming-mouse configuration tool. A
   synthesizer at that name is reachable only through an explicit absolute path on the Connection's
   route.

The synthesizer performs its own grapheme-to-phoneme conversion using a library it bundles, which
happens to be espeak-ng. That is an internal stage of the neural pipeline: no espeak voice produces
audio here, this repository executes no espeak binary, and observing that library name in a recorded
attestation is not evidence of a fallback path.

## Evidence on a host with no sound card

`SpeechAttestation.sink` is optional. A headless deployment records `None` rather than a fabricated
family — synthesis still ran and still produced the byte count beside it, and the evidence says where
the audio went.

## Tests

No test spawns a synthesizer, opens a device, or requires a voice model. Two `#[ignore]`d live checks
exercise the real local stack and are run deliberately:

```bash
B10X_AUDIO_VOICE=/absolute/path/voice.onnx \
  cargo test --manifest-path crates/driver-speech/Cargo.toml -- --ignored --nocapture
```

The crate is an intentionally nested workspace with its own lock, like `driver-audio` and
`driver-sip`: process-capable code stays out of the deterministic catalogue-compiler workspace.
