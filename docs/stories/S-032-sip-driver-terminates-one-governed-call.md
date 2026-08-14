---
id: S-032
title: "The SIP driver terminates one governed call"
pillar: Platform
status: in-progress
priority:
design: docs/design/05-native-sip-and-rtvbp.md
epic: native-voice
areas: [catalog, domain, service, server, driver-sip]
note: "Development loopback driver is real; catalog provider, full conformance, and stable network aperture remain"
---

# The SIP driver terminates one governed call

## Goal

Implement the closed built-in `sip_v1` driver with exact `sipx` dependencies and prove one bounded
SIP call without importing RTVBP or requiring a live carrier.

## Acceptance

- [x] S-024 and the platform-family ports exist; the driver implements the neutral
      `TelephonySession` port and only `server` selects it after admission.
- [x] The closed catalog vocabulary represents `sip_v1` without accepting arbitrary driver names;
      canonical-document and consumer round-trip tests preserve it exactly.
- [ ] At least one authoritative or repository-authored carrier/PBX source with registered
      provenance produces a reviewed provider declaration and generated canonical catalog member
      whose interaction shape is `session_establishment`, driver is `sip_v1`, implementation is
      `built_in`, initial model exposure is false, and capability/risk/idempotency facts are exact.
- [ ] The source, provenance, provider declaration, canonical document, lock row, pack, and web
      projection land atomically; the effective catalog exposes the member only where the selected
      deployment can actually dispatch `sip_v1`.
- [x] `driver-sip` is the only named network-capable driver and calls `sipx` bind only from a
      non-serializable, proof-bearing `AdmittedSipPlan`; fence tests reject every other path.
- [ ] Configured, DNS-resolved, SIP-learned, and SDP-learned targets plus local listener/media ports
      are checked against the admitted destination/bind apertures before transmission.
- [ ] A symmetric-RTP source cannot become an egress peer before media-peer admission. Loopback may
      characterize the pinned behavior, but stable/exposed support waits for an enforceable hook,
      supported disable switch, or independently measured network aperture.
- [x] The workspace explicitly moves to Rust 1.88 with its MSRV lane green, and `sipx`
      `v1.0.0-rc.23` resolves to `004ac534b8b222060ad2d2308763efe6e1dedc10` in release evidence.
- [ ] Loopback fixtures cover registration, inbound and outbound dialogs, DTMF, bounded duplex
      audio, hangup, cancellation, authentication failure, reconnect, and whole-task termination.
- [ ] Driver, implementation, credential, placement, target aperture, tenant, and profile are
      server-selected; caller attempts to widen any one refuse before secret access.
- [ ] SIP credentials and SRTP material are structurally absent from protocol DTOs, events, audit,
      logs, fixtures, and the `TelephonySession` port.
- [x] The crate implements no proxy, registrar service, PBX, arbitrary SIP URI proxy, TURN, video,
      IVR, RTVBP, or Agent behavior.
- [x] Stable support remains blocked while the selected `sipx` API is prerelease unless an explicit
      compatibility and upgrade exception is accepted with conformance evidence.

## Progress

- The exact sipx-backed outbound driver now consumes only a proof-bearing, non-serializable server
  plan. Admission rejects non-loopback deployments and aperture widening before the driver can bind.
- A real UDP SIP/RTP loopback test normalizes G.711 to the neutral 8 kHz/20 ms PCM profile, crosses
  RTVBP to a fake application in both directions, and observes whole-task teardown without a model,
  external PBX, or credentials.
- CI builds/tests/lints the isolated exact lock, and a dependency fence keeps sipx out of the
  canonical compiler closure. No provider member is advertised yet: the authoritative source,
  generated artifacts, remaining conformance matrix, and stable learned-peer enforcement are open.
  A catalog invariant fails if any provider advertises `sip_v1` before that atomic change lands.
