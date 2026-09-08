# media/v1alpha1 (MediaSession)

- **Status:** proposed, not implemented.
- **Family:** media. Rides on [sessions](../../sessions/v1alpha1/semantics.md) as a stream contract. Independent bindings: SIP/RTP, RTVBP, local audio device, WebRTC (target).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `media/v1alpha1` |
| Profiles (track formats) | `pcm-s16le-8k-mono-20ms` |
| Capabilities | `dtmf`, `interrupt`; reserved `hold`, `transfer` |
| Vocabulary | track descriptor, frame, sequence, timestamp, ready, loss, overload, terminal |

`MediaSession` adds to a session: negotiated track descriptions (encoding, sample format and rate, channels, timing, bounds), duplex streams with sequence and timestamp semantics, explicit readiness, interruption, loss, overload and termination behavior, and capability negotiation (`docs/design.md:628-633`). SIP/RTP, WebRTC and RTVBP expose the same media semantics through independent implementations; their protocol code must not depend on one another (`docs/design.md:30`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| VoiceSession v0alpha1: opaque call/session/channel refs, explicitly untrusted participant context, bounded audio, optional channel signals, output interruption, observable loss, typed termination; SIP, SDP, RTP, RTVBP, carriers, credentials, IVR, recordings, transcripts, tools, Agent lifecycle forbidden from the contract | `../connectors/contracts/voice-session/v0alpha1/README.md:3-9` | preserve; refs and lifecycle move to `sessions`; media semantics stay here |
| Media and signals before `ready` refuse; input overflow drops the oldest frame and emits bounded loss; output overload never silently drops synthesized speech: it degrades or terminates as `media_overload` | same, lines 17-20 | preserve |
| Profile `pcm_s16le`, 8 kHz, mono, 20 ms, 320 bytes, asserted at bind time; no transcoding at the seam; a second profile arrives as a refusal, not an assumption | `../connectors/crates/voice-local-audio/README.md:18-26` | preserve as the first named profile; a profile fact, not a contract identity (`docs/design.md:639`) |
| DTMF sent as RFC 4733 telephone-events on the negotiated media session; refusal error when the session declines | `../connectors/crates/driver-sip/src/lib.rs:359-381` | preserve as capability `dtmf`; RFC 4733 is the SIP binding's realization, not the contract's |
| Application-side speech detection owns barge-in; the voice endpoint clears bounded playback; Agent steering is a separate fact | `../connectors/docs/design/05-native-sip-and-rtvbp.md:313-318` | preserve as capability `interrupt` |
| RTVBP binding `b10x.voice.v1`; upstream label `L16/8000/1` stays inside the binding; headerless offer refused, explicit profile required; `babelforce.v1` stays downstream | old design 05, lines 271-295 | preserve; binding facts live with the RTVBP adapter |
| Bridge: forwards compatible semantic media and controls; joint termination policy; imports neither SIP nor RTVBP | `docs/design.md:653` | preserve; bridge is a composition component conforming to this contract twice |

## 3. Types

Track descriptor (negotiated per stream at session establishment):

```json
{
  "track": "audio",
  "direction": "duplex",
  "profile": "pcm-s16le-8k-mono-20ms",
  "format": { "encoding": "pcm_s16le", "sample_rate_hz": 8000, "channels": 1, "frame_ms": 20, "frame_bytes": 320, "endianness": "little" },
  "bounds": { "input_queue_frames": 50, "output_queue_frames": 50 },
  "capabilities": ["dtmf", "interrupt"]
}
```

Frame (data plane, binary; the JSON here is the semantic view):

```json
{ "track": "audio", "seq": 1234, "timestamp_samples": 987200, "bytes": 320, "payload": "<opaque>" }
```

Control messages (session events/requests scoped to a track):

| Message | Direction | Meaning |
|---|---|---|
| `track_ready` | endpoint → peer | frames may flow |
| `loss { direction, frames_dropped, seq_from, seq_to }` | endpoint → peer | bounded loss report |
| `overload { direction }` | endpoint → peer | output queue saturated; followed by degrade or terminal `media_overload` |
| `signal { kind: "dtmf", digit, duration_ms }` | either | capability `dtmf` only |
| `interrupt { track }` | application → endpoint | capability `interrupt`: clear bounded output playback |
| `stream_end { track, direction }` | either | half-close of one direction |

Terminal reasons added to `sessions`: `media_overload`, `media_incompatible`.

## 4. Rules

- Negotiation: a stream is established only when both participants accept the same profile and capability set; no implicit transcoding; a mismatch is `media_incompatible` before `ready` (`docs/design.md:637`).
- Readiness gating: frames or signals before `track_ready` are refused, never buffered into a not-yet-admitted session.
- Sequence and timestamp: `seq` increments by one per frame per direction; `timestamp_samples` increments by `frame_ms × sample_rate / 1000` per frame; a gap in `seq` is loss and must be reported by the receiver with `loss`.
- Loss policy: input direction (from the remote party toward the application) drops the oldest frame on overflow and emits `loss`; output direction (application → remote) never silently drops: on overflow the endpoint emits `overload`, then either degrades (declared per binding) or terminates `media_overload`.
- Data/control isolation: frames travel on a bounded data path scheduled separately from control; a saturated stream must not delay `close`, `revoke`, or `ping` beyond the session's control bound (`docs/design.md:637`).
- Capabilities are explicit: an endpoint without `dtmf` refuses a `signal` with `Unsupported`; it never accepts and discards (`docs/design.md:635`). `hold`/`transfer` are not promised by this version.
- Interrupt: clears the endpoint's bounded output queue and reports the number of frames cleared; it does not cancel anything at the application (`docs/design.md:315` in old design 05, steering is separate).
- Termination: first accepted terminal wins (sessions rule); `stream_end` in one direction leaves the other usable only while the session's live data authority remains valid. Revocation and any session terminal stop both directions under [sessions §4.1](../../sessions/v1alpha1/semantics.md#41-traffic-cutoff-and-teardown-f06): at most 2 s from host revocation, no grace after lease expiry, no queued-data drain, and at most 5 s to local teardown/accounting. Queued input, output, DTMF, playback and bridge forwarding are discarded at cutoff with local accounting. This declared termination discard does not change the live-session overflow rule. A remote close acknowledgement is not required, and bytes already beyond the last controlled boundary cannot be recalled.
- Bridge: a bridge holds two MediaSessions, checks profile and capability compatibility before forwarding, forwards frames and signals, and applies a declared joint termination policy (first profile: either side's terminal closes both, propagating the originating reason unless the other side already accepted an earlier terminal). It imports no protocol crate. Both legs enforce their own live leases; terminal propagation cannot extend either lease or add a per-hop cutoff allowance. An unresponsive leg cannot retain queued audio after its cutoff.
- Participant context (caller number, display name) is untrusted data on the session; it never selects a tenant or destination (`docs/design.md:671`).

## 5. Limits (first-profile defaults, from the old profile where stated)

| Bound | Value | Source |
|---|---|---|
| Frame | 320 bytes, 20 ms | old profile |
| Input queue | 50 frames (1 s) | new, to be measured |
| Output queue | 50 frames | new, to be measured |
| Control latency under load | within the session's ping bound | sessions |
| Signal rate | bounded (default 20 per second) | new |
| Revocation cutoff / terminal teardown | 2 s / 5 s maximum; zero queued frames after cutoff | normative sessions §4.1; applies to direct, relay and local device paths |

## 6. Conformance scenarios (`docs/design.md:984`, `1002`)

- Two fake endpoints negotiate `pcm-s16le-8k-mono-20ms`; 1,000 frames each way; `seq`/`timestamp_samples` monotonic; zero loss reports.
- Frame before `track_ready` → refused.
- Input flood of 200 frames with the peer stalled → oldest dropped, `loss` reports sum to the drop count.
- Output flood → `overload`, then `media_overload` terminal within the bound; no frame silently dropped (fixture counts).
- `signal dtmf` to an endpoint without `dtmf` → `Unsupported`; with `dtmf` → delivered once.
- `interrupt` during playback → output queue cleared; count reported; input unaffected.
- Data flood with a peer ignoring `close` → controlled traffic stops within the session cutoff bound; local teardown/accounting completes within 5 s without waiting for an acknowledgement.
- Bridge between a fake SIP-like endpoint and a fake RTVBP-like endpoint → frames pass unchanged; terminal on one side closes the other with the same reason unless that side already recorded its own terminal; substituting either endpoint leaves the bridge and application unchanged (`docs/design.md:1015`).
- Profile mismatch (one side offers 16 kHz) → `media_incompatible`, no `ready`.

## 7. Compatibility

- The old VoiceSession vectors are characterization input for readiness, loss, overload and terminal rules; naming differs (`media_overload` preserved).
- Bindings publish their own fixtures under their adapter (RTVBP `b10x.voice.v1`, SIP/RTP), not here (old README rule preserved).

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Track, frame, control types; bounded data path abstraction with separate scheduling | `crates/connectors-contracts` (media module; a separate crate only if dependency boundaries demand, `docs/design.md:626`) |
| Fake conforming endpoints for the harness | `crates/connectors-conformance` |
| Bridge component | composition crate, imports contracts only |
| WebSocket binary data binding for frames | host `duplex_transport` (sessions) |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `Session.streams[]` track descriptors | values on `Session` |
| Frames | not modeled |
| Media profile registry | value list in the adapter specification (`provides.profiles`) |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Queue depths | 50 frames each way |
| Degrade versus terminate on output overload | binding-declared; SIP binding terminates (old behavior), local audio may degrade |
| `hold`/`transfer` | reserved, not specified |
| WebRTC | interoperability target; no claim until an implementation passes this suite (`docs/design.md:947`) |
