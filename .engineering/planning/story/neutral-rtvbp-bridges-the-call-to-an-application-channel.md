---
format: aep.planning-md/1
id: story:neutral-rtvbp-bridges-the-call-to-an-application-channel
kind: story
status: active
title: Neutral RTVBP bridges the call to an application channel
refs:
- provider: legacy
  reference: S-033
relations:
- derived_from: epic:native-voice
scope:
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: crates/rtvbp-voice-endpoint
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
- confidence: cited
  path: crates/voice-runtime
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-033-neutral-rtvbp-bridges-the-call-to-an-application-channel.md:21`. **read**

- [ ] RTVBP Rust SDK `sdk/rust/v0.1.0` resolves to
      `dc0a60f7425b4899885f372152028457791b1e72`, its released crate asset matches SHA-256
      `7d1d675e359016a5c8711bc0a29783ad9ce57a2f80f47ab5c77bc0152935ff9b`, and licenses are
      recorded; the B10x binding manifest and fixtures are locally owner-released with
      conformance evidence rather than compiled into the upstream SDK.
- [ ] The released `VoiceSession` contract contains no RTVBP type, and both endpoint adapters prove
      their method/event/media mapping against the same generic vectors.
- [x] Generic code boots and passes its lifecycle with `babelforce.v1` absent; attempting that
      product profile or omitting explicit profile negotiation at the generic endpoint refuses by
      name rather than selecting RTVBP's legacy default.
- [ ] Memory and bounded WebSocket fixtures prove initialize, control/media channels, DTMF,
      duplex L16 mono audio, keepalive, close, cancellation, and whole-task termination.
- [x] The proof-bound authority binds endpoint, tenant, actor, Connection, Grant, operation,
      profile, proof key, and lease; wrong audience, expiry, replay, and revocation refuse.
- [x] Media/frame/queue/request bounds are explicit; overflow records observable bounded loss, and
      barge-in has one causal interruption/clear path.
- [ ] Characterization measures the SDK's internal control/transport queues. An outer bounded queue
      alone cannot support a stable/exposed bounded-memory claim.
- [ ] A private-satellite fixture connects outward directly, refuses an unreachable route as
      `session_unserved`, and proves federation never carries RTVBP or media bytes.
- [ ] One composed model-free loopback call crosses SIP → RTVBP → fake neutral application media,
      including hangup, cancellation, loss, interruption, and one-generation drain.
- [ ] A signed Connectors owner bundle and clean-room consumer proof precede any stable or hosted
      support claim.
- [ ] The `RUSTSEC-2025-0141` maintenance warning is closed by replacing RTVBP's
      `webrtc 0.14.0` → `dtls 0.13.0` → `bincode 1.3.3` closure or by pinning and reviewing a
      maintained fork before stable or exposed support.

## Context

Implement the Connectors-owned RTVBP binding from generic `VoiceSession` semantics over
`TelephonySession` and prove a directly authorized, bounded session against a fake application peer
without a model or Babelforce product semantics.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server, rtvbp-voice-endpoint, voice-runtime] · design `docs/design/05-native-sip-and-rtvbp.md`. **read**

Source `note:` field, quoted: “Supervised authenticated SIP/RTVBP composition exists; complete lifecycle, placement, and release evidence remain”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-033-neutral-rtvbp-bridges-the-call-to-an-application-channel.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-033-neutral-rtvbp-bridges-the-call-to-an-application-channel.md`, which is not deleted and now names this artifact.

- First written 2026-08-14 · last touched 2026-08-15 · 8 revision(s)
- Legacy id `S-033`, recorded as the reference `legacy:S-033`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
