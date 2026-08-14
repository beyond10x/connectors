---
id: S-033
title: "Neutral RTVBP bridges the call to an application channel"
pillar: Platform
status: in-progress
priority:
design: docs/design/05-native-sip-and-rtvbp.md
epic: native-voice
areas: [domain, protocol, service, server, rtvbp-voice-endpoint, voice-runtime]
note: "Supervised authenticated SIP/RTVBP composition exists; complete lifecycle, placement, and release evidence remain"
---

# Neutral RTVBP bridges the call to an application channel

## Goal

Implement the Connectors-owned RTVBP binding from generic `VoiceSession` semantics over
`TelephonySession` and prove a directly authorized, bounded session against a fake application peer
without a model or Babelforce product semantics.

## Acceptance

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

## Progress

- The exact final RTVBP Rust SDK release is pinned. The alpha generic owner bundle and separate
  `b10x.voice.v1` binding bundle are hash-frozen in this repository.
- The voice-side adapter uses only generic upstream envelope/transport APIs. Its memory transport
  test performs exact profile negotiation, serving-side authority redemption, initialization, and
  duplex 320-byte media over a fake `TelephonySession` without `babelforce.v1`.
- Both repositories now own finite WebSocket transports instead of relying on RTVBP's unbounded
  WebSocket queues. The application adapter independently redeems exact WSS/profile/DPoP authority
  before media and proves bounded input/loss, output overload, interruption, close, lease, and
  generation-drain behavior.
- The Connectors WebSocket transport retains both pump handles, aborts and boundedly joins them on
  close/timeout, and aborts them when the last owner is dropped. Stalled-writer regressions prove
  both its own close deadline and cancellation by an outer deadline release the stream.
- Voice-side application-to-call overflow is reported directly by the read pump, selects typed
  `media_overload`, and stops admitting later wire frames. A regression proves a later application
  close cannot hide that earlier terminal fact; the supervisor has no polling window.
- Connectors now has one nested `voice-runtime` leaf. Server admission joins the exact SIP and
  application routes before I/O; admitted tenant identity comes only from grant evidence, while the
  runtime alone owns credential custody, ephemeral proof material, authority timing, liveness/lease
  supervision, and first-wins termination.
- The runtime exposes a one-shot `sip.dial` establishment observer. It returns the catalog response
  only after both the SIP dialog and authenticated RTVBP application binding are ready; any earlier
  terminal outcome becomes a typed closed refusal while the supervisor continues owning the live
  session after the receipt.
- A real sipx UDP/RTP loopback crosses the neutral `TelephonySession` port, a serving-side-redeemed
  WebSocket authority, exact RTVBP initialization, duplex PCM, application close acknowledgement,
  terminal event, and observed SIP/transport teardown. The broader composed cases
  (cancellation, loss, interruption, generation drain), satellite/unserved path, complete shared
  vectors, signed owner release, and clean-room release proof remain open; support is not stable or
  hosted.
