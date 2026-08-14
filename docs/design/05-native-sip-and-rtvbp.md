# Design 05: native SIP at the Connectors edge, RTVBP behind it

**Status:** accepted implementation plan; not implemented · **Date:** 2026-08-14

**Authority:**
[architecture ADR 0024](https://github.com/b10x/architecture/blob/main/adr/0024-native-voice-uses-sip-and-rtvbp-at-the-channel-edge.md) ·
[architecture RFC 0009](https://github.com/b10x/architecture/blob/main/rfcs/0009-native-sip-and-rtvbp-voice-boundary.md) ·
[development vectors](https://github.com/b10x/architecture/blob/main/specifications/draft/voice-session-v1/README.md)

This document turns the accepted cross-repository boundary into a Connectors implementation shape.
It deliberately creates no workspace member yet: the platform family is unstarted, S-024's common
zero-I/O plan does not exist, and the generic RTVBP catalog is not released.

## 1. Boundary

Connectors owns the configured SIP Connection, trunk credentials, destination/listener aperture,
Grant admission, inbound tenant/channel binding, driver selection, placement, call lifecycle,
session-authority issuance, and audit. A built-in `sip_v1` driver terminates SIP/SDP and RTP/SRTP
using pinned [`codewandler/sipx`](https://github.com/codewandler/sipx) crates.

RTVBP is not the trunk protocol. It begins behind that SIP endpoint and carries typed call control
plus duplex media directly to an application-channel endpoint. Connectors ordinary invoke/event
delivery and satellite federation do not carry those continuous bytes.

The endpoint is a SIP user agent, not a proxy, registrar service, PBX, arbitrary dial proxy, TURN
relay, browser media platform, or IVR engine. Caller ID and asserted SIP identities are untrusted
channel context. The configured Connectors deployment/service identity is the actor that requests
product work.

## 2. Exact repository delta

The eventual Connectors additions are:

```text
connectors/
├── crates/
│   ├── domain/
│   │   └── src/voice.rs                  # neutral call/session ids, plans, bounds and ports
│   ├── protocol/
│   │   └── src/voice.rs                  # establishment DTOs; never media frames
│   ├── service/
│   │   └── src/voice.rs                  # admission, routing, authority and lifecycle use cases
│   ├── server/
│   │   └── src/voice/
│   │       ├── mod.rs                    # sole composition point
│   │       ├── authority.rs              # proof-bound issue/redeem adapter
│   │       └── supervisor.rs             # call task ownership and generation drain
│   ├── driver-sip/
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs                    # sipx-backed TelephonySession implementation
│   │   └── tests/loopback.rs             # no external PBX or credentials
│   └── rtvbp-voice-endpoint/
│       ├── Cargo.toml
│       ├── src/lib.rs                    # neutral RTVBP voice role over TelephonySession
│       └── tests/{memory,websocket}.rs
├── fixtures/
│   └── voice-session-v1/                 # pinned owner vectors once released
└── docs/stories/
    ├── S-032-sip-driver-terminates-one-governed-call.md
    └── S-033-neutral-rtvbp-bridges-the-call-to-an-application-channel.md
```

The exact module files under the four platform crates may land only as part of their owning
milestone; the two new workspace members do not arrive before them. No `vendor/sipx`,
`vendor/rtvbp`, Git submodule, `voice` repository, substrate protocol module, dynamic plugin, or
out-of-process gateway artifact is added by this plan.

## 3. Dependency graph

```text
server ───────────────▶ service ─────────────▶ domain
  ├──▶ driver-sip ────────────────────────────▶ domain
  │       └──▶ exact sipx crates
  └──▶ rtvbp-voice-endpoint ──────────────────▶ domain
          └──▶ exact neutral RTVBP Rust SDK

protocol ─▶ domain identities / bounded projections only
driver-sip ─X─▶ RTVBP
Agent/Substrate ─X─▶ sipx or RTVBP
```

`domain::voice` names a protocol-neutral `TelephonySession` port. `driver-sip` implements it.
`rtvbp-voice-endpoint` consumes it and implements the neutral RTVBP voice role. Only `server` may
select and compose both after `service` returns a proof-bearing admitted plan. No catalog document,
wire request, model, or caller names a Rust crate or upstream implementation.

Design 02's original literal socket-opener rule is refined because the pinned
[`sipx-transport` API](https://github.com/codewandler/sipx/blob/004ac534b8b222060ad2d2308763efe6e1dedc10/crates/sipx-transport/src/lib.rs)
owns sockets and performs `bind(Config)`. `server`/`service` remains the sole admission,
destination-policy, credential, and composition path. `driver-sip` is the one explicitly
network-classified driver and may call `sipx` bind only with a non-serializable, proof-bearing
`AdmittedSipPlan` returned by that path. Catalog data, a wire DTO, and a caller cannot construct it.

The admitted plan fixes local signaling/media listener apertures and the destination policy applied
to configured, DNS-resolved, SIP-learned, and SDP/RTP-learned targets before transmission. Fence
tests reject socket-capable dependencies in all other drivers/platform crates and direct `sipx`
binds anywhere else. If `sipx` later exposes injected pre-bound sockets, this adapter may narrow
without changing `TelephonySession`.

## 4. Catalog and admission

The first declaration uses:

- interaction shape `session_establishment`;
- protocol driver `sip_v1`;
- implementation form `built_in`;
- deployment-selected local or satellite placement;
- explicit public/private network, listener, port range, and secret capability facts.

An outbound operation declares bounded target forms and is non-idempotent unless a narrower member
proves otherwise. The caller cannot provide a driver, artifact, credential location, network class,
placement, arbitrary SIP URI, tenant, or profile. Inbound dialogs match exactly one configured
Connection plus tenant/application-channel binding; zero or multiple matches refuse before product
work. SIP credentials and SRTP material never enter protocol DTOs, RTVBP frames, events, audit,
fixtures, or Agent input.

## 5. RTVBP profiles

RTVBP provides reusable envelope/runtime/transport machinery, but a catalog owns payload meaning:

- Generic voice requires a released neutral catalog such as `b10x.voice.v1`, generated by
  the upstream RTVBP pipeline. Its vocabulary is limited to neutral call/session/media lifecycle.
- The frozen `babelforce.v1` catalog includes application movement and session-variable semantics.
  It is implemented only by the downstream Babelforce distribution adapter, never by
  `rtvbp-voice-endpoint` or another generic crate.

The neutral catalog release is a hard prerequisite, not a locally copied schema. Its generated SDK
identity and conformance fixtures are pinned into the Connectors owner bundle. The initial transport
proof uses in-memory peers and bounded WebSocket control/media with L16 mono audio; WebRTC is a
later additive browser/NAT journey.

The initial catalog is limited to `session.initialize`, `session.terminate`, `call.hangup`,
`audio.buffer.clear`, and `ping`; events `session.updated`, `call.hangup`, `dtmf`,
`audio.speech.started`, and bounded `audio.info`; plus the existing binary media channels.
Initialization has closed session/call/channel references, explicitly untrusted remote-party
context, and bounded format offers. It has no free-form metadata/session variables, tenant
authority, credentials, IVR node, recording command, Agent tool event, or transcript projection.

The hand-authored upstream source belongs in
`spec/crates/rtvbp-spec-b10x-voice-v1/src/catalog.rs`; its manifest and Rust/Go catalog modules
are generator output. The exact profile/subprotocol name is accepted and emitted upstream rather
than aliased locally.

[Legacy RTVBP WebSocket negotiation](https://github.com/babelforce/rtvbp/blob/ee73c2f3ce13ffcfdd188ed2068ef79aea1b2fa8/docs/designs/multi-catalog.md)
maps an absent subprotocol to `rtvbp.v1`/`babelforce.v1`. The generic endpoint requires an explicit
exact neutral profile and refuses a headerless offer instead of inheriting that compatibility
default. The SDK's generated `demo.v1` profile is a multi-catalog test and is not an application
contract.

## 6. Authority, placement, and lifecycle

The 60-second, proof-bound, single-redemption authority from architecture ADR 0016 binds the exact
endpoint, tenant, actor, Connection, Grant, operation, channel kind, RTVBP profile, proof key, and
session lease. Authority expiry controls establishment; hangup, cancellation, revocation, lease,
and bounded drain control an established session.

A public gateway may dial or accept the independently reachable application endpoint according to
the admitted plan. A satellite beside a private PBX initiates RTVBP outward. SaaS never dials into
the private network, and the federation control stream never relays RTVBP or audio. No permitted
direct route produces `session_unserved`.

Each call task belongs to one gateway/application generation until hangup or the bounded drain
deadline. New calls move to a ready successor; a live SIP dialog and its media are never migrated.
Media/frame/queue/request limits are finite. Overflow records typed loss/degradation instead of
blocking without bound. Application-side speech detection owns a barge-in request; the voice
endpoint clears bounded playback, while Agent steering/cancellation remains a separate fact.

## 7. Dependency baseline and release gates

The reviewed initial identities are:

| Dependency | Exact identity | Resolved commit | Gate |
|---|---|---|---|
| `codewandler/sipx` | `v1.0.0-rc.23` | `004ac534b8b222060ad2d2308763efe6e1dedc10` | development characterization only while prerelease |
| RTVBP Rust SDK | `sdk/rust/v0.1.0` | `ee73c2f3ce13ffcfdd188ed2068ef79aea1b2fa8` | runtime baseline; neutral catalog still required |

Both declare Rust 1.88. This workspace currently declares 1.87, so one explicit workspace-wide
MSRV change and a green pinned-MSRV lane precede either dependency. Cargo versions are exact, the
lockfile records the resolved graph, and release evidence records licenses, source commits,
generated catalog identity, conformance results, and artifact digest. A mutable Git ref or compatible
range is refused. A stable Connectors voice claim additionally requires a stable `sipx` API or a
reviewed compatibility/upgrade exception.

## 8. Implementation sequence and exit evidence

1. Complete S-024 and the `domain`/`protocol`/`service`/`server` family.
2. Characterize the pinned `sipx` API, admitted-plan/destination-policy seam, feature closure,
   licenses, and MSRV; make the reviewed Rust 1.88 change.
3. Release and pin the neutral RTVBP catalog and generated Rust surface upstream.
4. Implement S-032 against loopback SIP only: registration, inbound/outbound, DTMF, bounded media,
   hangup, cancellation, target refusal, credential non-disclosure, and task teardown.
5. Implement S-033 independently over memory and WebSocket with a fake application peer: profile
   negotiation, issue/redeem/replay, bounds, loss, interruption, close, and generation drain.
6. Compose a model-free loopback call end to end and pass the architecture development vectors.
7. Only then characterize one explicitly authorized real PBX/trunk and satellite outward path.
8. Publish a signed Connectors owner bundle and prove a clean-room application-channel consumer
   before any stable, hosted, or authoritative-writer claim.

The Babelforce compatibility adapter is a downstream follow-on. It neither blocks the generic
loopback proof nor changes the ownership of `babelforce.v1`.
