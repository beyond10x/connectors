# Design 05: native SIP at the Connectors edge, RTVBP behind it

**Status:** accepted implementation plan; not implemented · **Date:** 2026-08-14

**Authority:**
[architecture ADR 0024](https://github.com/b10x/architecture/blob/main/adr/0024-native-voice-uses-sip-and-rtvbp-at-the-channel-edge.md) ·
[architecture ADR 0026](https://github.com/b10x/architecture/blob/main/adr/0026-native-voice-contracts-are-protocol-neutral.md) ·
[architecture RFC 0009](https://github.com/b10x/architecture/blob/main/rfcs/0009-native-sip-and-rtvbp-voice-boundary.md) ·
[development vectors](https://github.com/b10x/architecture/blob/main/specifications/draft/voice-session-v1/README.md)

This document turns the accepted cross-repository boundary into a Connectors implementation shape.
It deliberately creates no workspace member yet: the platform family is unstarted and S-024's
common zero-I/O plan does not exist. The B10x RTVBP binding is no longer an upstream catalog
prerequisite; it is implemented only after the generic `VoiceSession` semantics exist.

## 1. Boundary

Connectors owns the configured SIP Connection, trunk credentials, destination/listener aperture,
Grant admission, inbound tenant/channel binding, driver selection, placement, call lifecycle,
session-authority issuance, and audit. A built-in `sip_v1` driver terminates SIP/SDP and RTP/SRTP
using pinned [`codewandler/sipx`](https://github.com/codewandler/sipx) crates.

RTVBP is not the trunk protocol. It begins behind that SIP endpoint and carries typed call control
plus duplex media directly to an application-channel endpoint. It is one binding of the
Connectors-owned, protocol-neutral `VoiceSession` contract. Connectors ordinary invoke/event
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
│   │   └── src/voice.rs                  # TelephonySession + VoiceSession semantics and ports
│   ├── protocol/
│   │   └── src/voice.rs                  # released VoiceSession contract projection
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
│       ├── src/lib.rs                    # VoiceSession-to-RTVBP binding over TelephonySession
│       └── tests/{memory,websocket}.rs
├── contracts/
│   └── voice-session/v0alpha1/           # protocol-neutral semantics and conformance vectors
├── fixtures/
│   ├── sip-telephony-session/v1/         # sipx loopback and learned-peer characterization
│   └── rtvbp-voice-binding/v1/           # local mapping plus memory/WebSocket fixtures
└── docs/stories/
    ├── S-032-sip-driver-terminates-one-governed-call.md
    └── S-033-neutral-rtvbp-bridges-the-call-to-an-application-channel.md
```

The exact module files under the four platform crates may land only as part of their owning
milestone; the two new workspace members do not arrive before them. No `vendor/sipx`,
`vendor/rtvbp`, Git submodule, `voice` repository, substrate protocol module, dynamic plugin, or
out-of-process gateway artifact is added by this plan.

The owner contract directory never contains SIP or RTVBP cases. The two fixture directories prove
their respective adapter bindings against that contract; they are released as evidence alongside
the contract but remain distinct artifacts.

## 3. Dependency graph

```text
server ───────────────▶ service ─────────────▶ domain
  ├──▶ driver-sip ────────────────────────────▶ domain
  │       └──▶ exact sipx crates
  └──▶ rtvbp-voice-endpoint ──────────────────▶ domain
          └──▶ exact generic RTVBP Rust SDK

protocol ─▶ domain identities / bounded projections only
driver-sip ─X─▶ RTVBP
Agent/Substrate ─X─▶ sipx or RTVBP
```

`domain::voice` separates a protocol-neutral internal `TelephonySession` from the released
cross-repository `VoiceSession` semantics. `driver-sip` implements the first.
`rtvbp-voice-endpoint` maps the second onto the generic RTVBP runtime while consuming the first.
Only `server` may select and compose them after `service` returns a proof-bearing admitted plan. No
domain request, model, or caller names a Rust crate, RTVBP method, or upstream implementation.

Design 02's original literal socket-opener rule is refined because the pinned
[`sipx-transport` API](https://github.com/codewandler/sipx/blob/004ac534b8b222060ad2d2308763efe6e1dedc10/crates/sipx-transport/src/lib.rs)
owns sockets and performs `bind(Config)`. `server`/`service` remains the sole admission,
destination-policy, credential, and composition path. `driver-sip` is the one explicitly
network-classified driver and may call `sipx` bind only with a non-serializable, proof-bearing
`AdmittedSipPlan` returned by that path. Catalog data, a wire DTO, and a caller cannot construct it.

The admitted plan fixes local signaling/media listener apertures and the destination policy applied
to configured, DNS-resolved, SIP-learned, and SDP-learned targets before transmission. Because the
selected media runtime can learn a symmetric-RTP source internally, stable evidence must separately
prove that source admitted before it becomes an egress peer. Fence tests reject socket-capable
dependencies in all other drivers/platform crates and direct `sipx` binds anywhere else.

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

## 5. Generic contract and RTVBP binding

`VoiceSession` is the canonical semantic contract. It has opaque call/session/channel references,
explicitly untrusted participant context, bounded media negotiation and duplex frames, optional
channel signals, output interruption, observable loss, and typed termination. It has no SIP, RTP,
RTVBP, carrier, credential, IVR, recording, transcript, tool, or Agent-lifecycle type.

RTVBP provides reusable envelope/runtime/transport machinery. `rtvbp-voice-endpoint` owns the
voice-side mapping from `VoiceSession` to an exact B10x profile such as
`b10x.voice.v1`; the AI Agent Platform owns an independently implemented application-side
mapping. The binding manifest and fixtures ship with the Connectors owner bundle and both adapters
must pass the same generic semantic vectors. No B10x catalog needs to be compiled into the
upstream SDK.

The first implementation profile maps signed 16-bit little-endian PCM at 8 kHz, mono, 20 ms and
320 bytes per frame. This is one negotiated `VoiceSession` descriptor rather than the identity of
the generic contract. The upstream `L16/8000/1` label stays inside the binding. The frozen
`babelforce.v1` catalog includes application movement and session-variable semantics and remains
only in the downstream Babelforce adapter.

[Legacy RTVBP WebSocket negotiation](https://github.com/babelforce/rtvbp/blob/ee73c2f3ce13ffcfdd188ed2068ef79aea1b2fa8/docs/designs/multi-catalog.md)
maps an absent subprotocol to `rtvbp.v1`/`babelforce.v1`. The generic endpoint requires an explicit
exact local binding profile and refuses a headerless offer instead of inheriting that compatibility
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
| RTVBP Rust SDK | `sdk/rust/v0.1.0` | `ee73c2f3ce13ffcfdd188ed2068ef79aea1b2fa8` | final release; generic runtime baseline; local binding only |

Both declare Rust 1.88. This workspace currently declares 1.87, so one explicit workspace-wide
MSRV change and a green pinned-MSRV lane precede either dependency. Cargo versions are exact, the
lockfile records the resolved graph, and release evidence records licenses, source commits, local
binding identity, conformance results, and artifact digest. RTVBP's released crate asset is pinned
at SHA-256 `76b7a79069f725e7ae13d2ca9af5b47bf8198e83839c360185ff3cb368e95469`.
Its public generic request/event traits, handler registration, transport traits, and configurable
WebSocket subprotocols are the executable seam for the local binding. A mutable Git ref or
compatible range is refused. The selected RTVBP runtime has internal unbounded control/transport
queues; stable or exposed support requires bounded configuration/change, a bounded replacement of
those layers, or measured process containment. A stable Connectors voice claim also requires a
stable `sipx` API or a reviewed compatibility/upgrade exception.

## 8. Implementation sequence and exit evidence

1. Complete S-024 and the `domain`/`protocol`/`service`/`server` family.
2. Characterize the pinned `sipx` API, admitted-plan/destination-policy seam, feature closure,
   licenses, and MSRV; make the reviewed Rust 1.88 change.
3. Release the protocol-neutral `VoiceSession` contract and binding-neutral conformance vectors.
4. Implement S-032 against loopback SIP only: registration, inbound/outbound, DTMF, bounded media,
   hangup, cancellation, target refusal, credential non-disclosure, and task teardown.
5. Implement the local RTVBP binding in S-033 independently over memory and WebSocket with a fake
   application peer: semantic mapping, profile negotiation, issue/redeem/replay, bounds, loss,
   interruption, close, and generation drain.
6. Compose a model-free loopback call end to end and pass the architecture development vectors.
7. Only then characterize one explicitly authorized real PBX/trunk and satellite outward path.
8. Publish a signed Connectors owner bundle and prove a clean-room application-channel consumer
   before any stable, hosted, or authoritative-writer claim.

The Babelforce compatibility adapter is a downstream follow-on. It neither blocks the generic
loopback proof nor changes the ownership of `babelforce.v1`.
