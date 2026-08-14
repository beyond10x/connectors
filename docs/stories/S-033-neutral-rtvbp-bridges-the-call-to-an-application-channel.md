---
id: S-033
title: "Neutral RTVBP bridges the call to an application channel"
pillar: Platform
status: backlog
priority:
design: docs/design/05-native-sip-and-rtvbp.md
epic: native-voice
areas: [domain, protocol, service, server, rtvbp-voice-endpoint]
note: "Blocked by S-027, the released generic VoiceSession contract, and the product/Agent neutral media contracts"
---

# Neutral RTVBP bridges the call to an application channel

## Goal

Implement the Connectors-owned RTVBP binding from generic `VoiceSession` semantics over
`TelephonySession` and prove a directly authorized, bounded session against a fake application peer
without a model or Babelforce product semantics.

## Acceptance

- [ ] RTVBP Rust SDK `sdk/rust/v0.1.0` resolves to
      `ee73c2f3ce13ffcfdd188ed2068ef79aea1b2fa8`, its released crate asset matches SHA-256
      `76b7a79069f725e7ae13d2ca9af5b47bf8198e83839c360185ff3cb368e95469`, and licenses are
      recorded; the B10x binding manifest and fixtures are locally owner-released with
      conformance evidence rather than compiled into the upstream SDK.
- [ ] The released `VoiceSession` contract contains no RTVBP type, and both endpoint adapters prove
      their method/event/media mapping against the same generic vectors.
- [ ] Generic code boots and passes its lifecycle with `babelforce.v1` absent; attempting that
      product profile or omitting explicit profile negotiation at the generic endpoint refuses by
      name rather than selecting RTVBP's legacy default.
- [ ] Memory and bounded WebSocket fixtures prove initialize, control/media channels, DTMF,
      duplex L16 mono audio, keepalive, close, cancellation, and whole-task termination.
- [ ] The proof-bound authority binds endpoint, tenant, actor, Connection, Grant, operation,
      profile, proof key, and lease; wrong audience, expiry, replay, and revocation refuse.
- [ ] Media/frame/queue/request bounds are explicit; overflow records observable bounded loss, and
      barge-in has one causal interruption/clear path.
- [ ] Characterization measures the SDK's internal control/transport queues. An outer bounded queue
      alone cannot support a stable/exposed bounded-memory claim.
- [ ] A private-satellite fixture connects outward directly, refuses an unreachable route as
      `session_unserved`, and proves federation never carries RTVBP or media bytes.
- [ ] One composed model-free loopback call crosses SIP → RTVBP → fake neutral application media,
      including hangup, cancellation, loss, interruption, and one-generation drain.
- [ ] A signed Connectors owner bundle and clean-room consumer proof precede any stable or hosted
      support claim.

## Progress

- Architecture and implementation placement are accepted, and the final upstream SDK release now
  exists. The generic owner contract, local RTVBP binding, and implementation prerequisites do not.
