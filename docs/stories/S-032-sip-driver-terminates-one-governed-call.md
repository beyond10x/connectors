---
id: S-032
title: "The SIP driver terminates one governed call"
pillar: Platform
status: in-progress
priority:
design: docs/design/05-native-sip-and-rtvbp.md
epic: native-voice
areas: [catalog, domain, service, server, driver-sip]
note: "Source-grounded catalog and dev-cluster outbound proof exist; full conformance and stable network aperture remain"
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

## Progress

- The exact sipx-backed outbound driver now consumes only a proof-bearing, non-serializable server
  plan. Admission rejects non-loopback deployments, aperture widening, incomplete admitted
  identity, and dial deadlines outside `1..=30` seconds before the driver can bind. Organization is
  structurally copied from grant evidence; no application route can supply a competing tenant.
- A real UDP SIP/RTP loopback test normalizes G.711 to the neutral 8 kHz/20 ms PCM profile, crosses
  RTVBP to a fake application in both directions, and observes whole-task teardown without a model,
  external PBX, or credentials. A separate ringing-peer test proves runtime cancellation reaches
  sipx's cancellation-safe dial and the peer observes SIP CANCEL before the driver returns.
- `TelephonySession::wait_terminated` exposes the driver's first typed terminal cause to the
  supervisor. Media/signal EOF is no longer reclassified as remote hangup; a sipx timeout remains
  `transport_lost` through the neutral port. Session teardown joins the sipx owner task through
  endpoint shutdown, while dropping an abandoned session aborts that owner.
- The B10x source pin, provider declaration, canonical document, lock row, pack, and web
  projection now land together. One catalog invariant permits exactly that reviewed `sip-dial`
  member under `b10x` / `io.b10x`, requires its non-HTTP facts, and proves Asterisk's
  ARI Provider contains no native SIP member or inherited HTTP endpoint/host.
- `sip.dial` takes only a Connection-owned symbolic alias. Planning independently requires
  B10x initiation authority and a Grant; server admission resolves the alias to an exact
  deployment route and refuses caller-supplied URIs, hosts, ports, placement, credentials, or
  aperture widening before the driver.
- The operator-authorized development mode completed a real TCP SIP call and RTP echo against the
  configured dev-cluster Asterisk peer. The proof is intentionally weaker than stable support: the learned peer is
  validated before the session is returned, but the selected sipx media runtime may already have
  transmitted symmetric RTP internally. The complete lifecycle matrix and enforceable pre-send
  learned-peer gate remain open.
- CI builds/tests/lints the isolated exact lock, and a dependency fence keeps sipx out of the
  canonical compiler closure.
