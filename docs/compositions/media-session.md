# Media composition

**Status:** proposed; no protocol runtime or rollout introduced here.

Native contracts and configuration belong to [SIP](../../adapters/sip/design.md)
and [RTVBP](../../adapters/rtvbp/design.md). Shared session/media own lifecycle and
negotiation. This document owns pairing, bridge/local-device composition and
integration scenarios, not either native protocol.

## 1. Scope and placement

Two independent adapters implement the same two contracts, [sessions](../../contracts/sessions/v1alpha1/semantics.md) and [media](../../contracts/media/v1alpha1/semantics.md): SIP terminates SIP/SDP and RTP toward a trunk or PBX; RTVBP carries the session and media to an application endpoint over its own transport. A bridge joins two media sessions and imports neither protocol crate (`docs/design.md:643-654`). A local audio device binding plays and captures on this machine's sound stack (`crates/voice-local-audio`, `driver-audio`). Placement: the SIP adapter sits where the trunk or PBX is reachable (a satellite beside a private PBX in the old design, `docs/design/05:306-309`); RTVBP connects outward to the application endpoint; SaaS never dials into the private network.

Rebuild means: outbound dial parity with `sip-dial`, the narrow PCM profile, DTMF and barge-in, the RTVBP `b10x.voice.v1` binding, and the bridge, on the new contracts; inbound offers to one configured destination; tenant assignment stays deferred.


## Composition ownership

A contract-only bridge joins two conforming MediaSession endpoints and imports
neither protocol stack. Local audio plays/captures via an explicit workstation
binding; speech operations remain outside this document. The predecessor
voice-runtime sequence becomes host supervision plus composition: resolve
operation-scoped credentials, establish a native port, issue authority, connect
the admitted application endpoint, supervise control/signals/duplex traffic,
keepalive/lease and termination. Emit exactly one ready receipt only after both
sides and all declared streams are ready. A nested workspace is not the default.

SIP's exact dial-effect outcome is owned by [its binding](../../adapters/sip/contracts/dial/v1alpha1/semantics.md).
Composition preserves known effect even if later media/application readiness
fails; it does not choose or weaken the native effect proof.

Both endpoints, bridge and local audio obey shared sessions cutoff/teardown:
2 s to controlled cutoff after authoritative revocation, no post-cutoff queue
drain, 5 s to local teardown/accounting after terminal acceptance. A call-duration
ceiling or 60 s establishment authority cannot extend the <=2 s live data lease.
Device playback/transmit buffers need a proven cutoff; closing one transport is
insufficient. Remote shutdown uncertainty cannot extend local ownership.

Local device evidence is ../connectors/crates/voice-local-audio, driver-audio
(audio_v1, PipeWire/PulseAudio/ALSA, NullAudioDevice) at 81459ac4. Treat an eventual
device implementation as its own binding with native sources/fixtures, not shared
protocol vocabulary. Asterisk ARI remains a separate optional HTTP adapter.

## Routing, specification and deferred scope

No resource discovery route is selected. Federation relays neither RTVBP nor
audio; media follows negotiated paths and live dialogs never migrate.
Native adapter-kind and codec obligations live with each adapter. Tenant
assignment/provisioning, hold/transfer, WebRTC and wider codec scope remain
deferred under the shared/media design.

## 10. Evidence required

- Fixture (`docs/design.md:1002`): two fake conforming endpoints before any protocol stack: establishment, duplex traffic, DTMF, interruption, overload, cancellation, terminal races; then SIP and RTVBP implementations through the contract-only bridge; substituting either endpoint leaves bridge and application unchanged.
- Live (separately authorized, not in default tests): TCP SIP plus RTP echo against a dev PBX as the old `sip_dial_characterize` example did (`crates/driver-sip/README.md:15-19`).
- Decoupling: the SIP crate has no RTVBP dependency and vice versa; the client and host build without either (`docs/design.md:1010-1015`).
- Timed cutoff fixtures: fill input/output/device queues, ignore close on either leg, partition direct control immediately after renewal, deliver a renewal late, suspend/resume an endpoint, and revoke during establishment. Measure last controlled emission/delivery and local resource release/accounting against the 2 s / 5 s ceilings. Compile-only ESS evidence does not satisfy these runtime obligations.
