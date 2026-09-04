---
format: aep.planning-md/1
id: story:sip-driver-terminates-one-governed-call
kind: story
status: active
title: The SIP driver terminates one governed call
refs:
- provider: legacy
  reference: S-032
relations:
- derived_from: epic:native-voice
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/driver-sip
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-032-sip-driver-terminates-one-governed-call.md:20`. **read**

- [x] S-024 and the platform-family ports exist; the driver implements the neutral
      `TelephonySession` port and only `server` selects it after admission.
- [x] The closed catalog vocabulary represents `sip_v1` without accepting arbitrary driver names;
      canonical-document and consumer round-trip tests preserve it exactly.
- [x] The repository-authored B10x native-capability source with registered provenance
      produces a reviewed provider declaration and generated canonical catalog member
      whose interaction shape is `session_establishment`, driver is `sip_v1`, implementation is
      `built_in`, and capability/risk/idempotency facts are exact. The first exposure is deliberately
      limited to the alias-only `sip.dial` member after the driver and development route proof.
- [x] The source, provenance, provider declaration, canonical document, lock row, pack, and web
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

## Context

Implement the closed built-in `sip_v1` driver with exact `sipx` dependencies and prove one bounded
SIP call without importing RTVBP or requiring a live carrier.

Source frontmatter: pillar Platform · areas [catalog, domain, service, server, driver-sip] · design `docs/design/05-native-sip-and-rtvbp.md`. **read**

Source `note:` field, quoted: “Source-grounded catalog and dev-cluster outbound proof exist; full conformance and stable network aperture remain”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-032-sip-driver-terminates-one-governed-call.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-032-sip-driver-terminates-one-governed-call.md`, which is not deleted and now names this artifact.

- First written 2026-08-14 · last touched 2026-08-16 · 7 revision(s)
- Legacy id `S-032`, recorded as the reference `legacy:S-032`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
