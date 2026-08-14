---
id: S-033
title: "Neutral RTVBP bridges the call to an application channel"
pillar: Platform
status: backlog
priority:
design: docs/design/05-native-sip-and-rtvbp.md
epic: native-voice
areas: [domain, protocol, service, server, rtvbp-voice-endpoint]
note: "Blocked by S-027, a released neutral RTVBP catalog, and the product/Agent neutral media contracts"
---

# Neutral RTVBP bridges the call to an application channel

## Goal

Implement the Connectors-owned RTVBP voice role over `TelephonySession` and prove a directly
authorized, bounded session against a fake application peer without a model or Babelforce product
semantics.

## Acceptance

- [ ] The exact final RTVBP Rust SDK identity and a released neutral B10x voice catalog are
      pinned with resolved commits, generated surfaces, licenses, and conformance evidence.
- [ ] Generic code boots and passes its lifecycle with `babelforce.v1` absent; attempting that
      product profile or omitting explicit profile negotiation at the generic endpoint refuses by
      name rather than selecting RTVBP's legacy default.
- [ ] Memory and bounded WebSocket fixtures prove initialize, control/media channels, DTMF,
      duplex L16 mono audio, keepalive, close, cancellation, and whole-task termination.
- [ ] The proof-bound authority binds endpoint, tenant, actor, Connection, Grant, operation,
      profile, proof key, and lease; wrong audience, expiry, replay, and revocation refuse.
- [ ] Media/frame/queue/request bounds are explicit; overflow records observable bounded loss, and
      barge-in has one causal interruption/clear path.
- [ ] A private-satellite fixture connects outward directly, refuses an unreachable route as
      `session_unserved`, and proves federation never carries RTVBP or media bytes.
- [ ] One composed model-free loopback call crosses SIP → RTVBP → fake neutral application media,
      including hangup, cancellation, loss, interruption, and one-generation drain.
- [ ] A signed Connectors owner bundle and clean-room consumer proof precede any stable or hosted
      support claim.

## Progress

- Architecture and implementation placement are accepted. The neutral upstream catalog and owner
  contracts do not yet exist.
