---
id: S-032
title: "The SIP driver terminates one governed call"
pillar: Platform
status: backlog
priority:
design: docs/design/05-native-sip-and-rtvbp.md
epic: native-voice
areas: [catalog, domain, service, server, driver-sip]
note: "Blocked by S-024 and the platform family; prove the admitted SIP network plan and Rust 1.88 before scaffolding"
---

# The SIP driver terminates one governed call

## Goal

Implement the closed built-in `sip_v1` driver with exact `sipx` dependencies and prove one bounded
SIP call without importing RTVBP or requiring a live carrier.

## Acceptance

- [ ] S-024 and the platform-family ports exist; the driver implements the neutral
      `TelephonySession` port and only `server` selects it after admission.
- [ ] `driver-sip` is the only named network-capable driver and calls `sipx` bind only from a
      non-serializable, proof-bearing `AdmittedSipPlan`; fence tests reject every other path.
- [ ] Configured, DNS-resolved, SIP-learned, and SDP/RTP-learned targets plus local listener/media
      ports are checked against the admitted destination/bind apertures before transmission.
- [ ] The workspace explicitly moves to Rust 1.88 with its MSRV lane green, and `sipx`
      `v1.0.0-rc.23` resolves to `004ac534b8b222060ad2d2308763efe6e1dedc10` in release evidence.
- [ ] Loopback fixtures cover registration, inbound and outbound dialogs, DTMF, bounded duplex
      audio, hangup, cancellation, authentication failure, reconnect, and whole-task termination.
- [ ] Driver, implementation, credential, placement, target aperture, tenant, and profile are
      server-selected; caller attempts to widen any one refuse before secret access.
- [ ] SIP credentials and SRTP material are structurally absent from protocol DTOs, events, audit,
      logs, fixtures, and the `TelephonySession` port.
- [ ] The crate implements no proxy, registrar service, PBX, arbitrary SIP URI proxy, TURN, video,
      IVR, RTVBP, or Agent behavior.
- [ ] Stable support remains blocked while the selected `sipx` API is prerelease unless an explicit
      compatibility and upgrade exception is accepted with conformance evidence.

## Progress

- Architecture and implementation placement are accepted; prerequisites are not started.
