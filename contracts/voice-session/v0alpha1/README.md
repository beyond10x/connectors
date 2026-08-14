# VoiceSession v0alpha1

`b10x.voice-session.v0alpha1` is the Connectors-owned, protocol-neutral semantic contract for
one admitted voice session with one negotiated duplex audio flow.

The contract exposes opaque call/session/channel references, explicitly untrusted participant
context, bounded audio, optional channel signals, output interruption, observable loss, and typed
termination. SIP, SDP, RTP, RTVBP, carriers, credentials, IVR, recordings, transcripts, tools, and
Agent lifecycle types are forbidden from this directory.

`voice-session.schema.json` is the strict self-describing `operation + payload` projection.
`vectors.json` is the executable state-transition suite consumed by the `protocol` crate. A binding
maps these neutral operation names onto its own methods/events; binding-specific names and transport
cases live under `fixtures/`, not here.

State transitions are serialized by the endpoint event loop. `created → ready → closing → closed`
is monotonic. Media and signals before `ready` refuse. The first terminal action accepted by that
serialized loop wins; later terminal actions are idempotent observations and cannot replace its
reason. Input overflow drops the oldest frame and emits bounded loss. Output overload never
silently drops synthesized speech: it degrades or terminates as `media_overload`.
