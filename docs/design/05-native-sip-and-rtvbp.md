# Design 05: native SIP at the Connectors edge, RTVBP behind it

**Status:** personal-local alpha operation serving and development SIP/RTVBP path implemented; stable serving gated · **Date:** 2026-08-14

**Authority:**
[architecture ADR 0024](https://github.com/b10x/architecture/blob/main/adr/0024-native-voice-uses-sip-and-rtvbp-at-the-channel-edge.md) ·
[architecture ADR 0026](https://github.com/b10x/architecture/blob/main/adr/0026-native-voice-contracts-are-protocol-neutral.md) ·
[architecture RFC 0009](https://github.com/b10x/architecture/blob/main/rfcs/0009-native-sip-and-rtvbp-voice-boundary.md) ·
[development vectors](https://github.com/b10x/architecture/blob/main/specifications/draft/voice-session-v1/README.md)

This document turns the accepted cross-repository boundary into a Connectors implementation shape.
The platform-family plan/dispatch seam, alpha `VoiceSession` owner bundle, one-shot authority, and
memory RTVBP endpoint now exist. The exact `sipx` driver, proof-bearing admission, locally
bounded WebSocket endpoints, application adapter, and a supervised runtime leaf now also exist.
That leaf resolves the generated B10x `sip-dial` member, proves authenticated model-free duplex
composition and one terminal result over real loopback SIP/RTP plus RTVBP WebSocket, and returns an
operation receipt at establishment. A separate operator-authorized characterization has established
TCP SIP and an RTP echo against a configured dev-cluster Asterisk peer. Stable serving, the remaining lifecycle
and placement matrix, and a signed release remain gated work.

**2026-08-16 Provider-ownership correction.** Native `sip.dial` is a B10x capability under
the permanent Provider identity `b10x` / `io.b10x`. Asterisk never owned this member:
its `asterisk` / `org.asterisk.ari` Provider remains the vendor ARI surface, while Asterisk is one
possible Connection-owned SIP peer alongside carriers, SBCs, and other PBXs. This correction
supersedes the earlier Asterisk-provider wording without changing the stable `sip.dial` tool ref.

The released-alpha `ConnectorOperation` bundle and owner-only local daemon now project this member
as generic `search`/`describe`/`invoke` plus session status/termination/reconciliation. The product
binary consumes the one supervised runtime leaf; no SIP or RTVBP type crosses the client contract.
Its initial configuration and audit journal are deliberately personal/development-only and do not
claim M2 hosted posture or M3's general Connection/Grant stores.

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

The implemented foundation and remaining planned additions are:

```text
connectors/
├── crates/
│   ├── domain/
│   │   ├── src/connection.rs             # protocol-neutral initiation authority
│   │   ├── src/plan.rs                   # admitted, driver-discriminated zero-I/O plans
│   │   └── src/voice.rs                  # TelephonySession + neutral voice semantics and ports
│   ├── protocol/
│   │   ├── src/operation.rs              # generic owner-facing operation/session protocol
│   │   ├── src/sip.rs                    # sip.dial alias input + establishment receipt
│   │   └── src/voice.rs                  # alpha VoiceSession contract projection and vectors
│   ├── service/
│   │   └── src/planning.rs               # pure catalog/admission/capability planning
│   ├── server/
│   │   ├── src/dispatch.rs               # sole closed-driver policy composition
│   │   ├── src/authority.rs              # proof-bound issue/present/redeem adapter
│   │   ├── src/sip.rs                    # exact alias/transport/aperture admission proof
│   │   ├── src/voice.rs                  # SIP + application route admission proof
│   │   └── src/local.rs                  # owner-credentialed bounded Unix-socket daemon
│   ├── driver-sip/                       # isolated sipx dependency/runtime workspace
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs                    # sipx-backed TelephonySession implementation
│   │   ├── examples/sip_dial_characterize.rs # exact operator-authorized PBX proof
│   │   └── Cargo.lock                    # exact prerelease source resolution
│   ├── rtvbp-voice-endpoint/
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs                    # VoiceSession-to-RTVBP binding over TelephonySession
│   │   ├── src/connect.rs                 # proof-bound client upgrade
│   │   ├── src/bounded_ws.rs              # finite local semantic transport
│   │   └── Cargo.lock                    # exact final RTVBP SDK resolution
│   ├── voice-runtime/                    # sole supervised adapter-composition leaf
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs                    # custody, authority, pumps, lease, first-wins terminal
│   │   ├── tests/supervised.rs           # real SIP/RTP + authenticated RTVBP WebSocket journey
│   │   └── Cargo.lock                    # joined, independently gated runtime closure
│   └── connectors-cli/                   # isolated product binary/runtime consumer
│       ├── src/main.rs                   # connectors serve + safe personal state root
│       ├── src/sip_backend.rs            # catalog/authority/approval/session projection
│       ├── src/runtime.rs                # VoiceRuntime launcher + exact TLS application route
│       ├── examples/asterisk-dev.example.toml
│       └── Cargo.lock                    # complete product dependency closure
├── specs/
│   ├── b10x.provenance.toml         # repository-owned native capability source pin
│   └── asterisk/
│       ├── samples/pjsip.conf.sample     # pinned first-party SIP configuration source
│       ├── samples/rtp.conf.sample       # pinned first-party RTP configuration source
│       └── provenance.toml               # optional Asterisk peer interoperability evidence
├── providers/
│   ├── b10x.toml                    # exposed native sip-dial member
│   └── asterisk.toml                     # vendor-owned ARI surface only
├── catalog/
│   ├── b10x.catalog.json            # generated native capability document
│   └── asterisk.catalog.json             # generated ARI provider document
├── contracts/
│   ├── connector-operation/v0alpha1/     # generic clean-room operation/session contract
│   └── voice-session/v0alpha1/           # protocol-neutral semantics and conformance vectors
├── fixtures/
│   ├── sip-telephony-session/v1/         # sipx loopback and learned-peer characterization
│   └── rtvbp-voice-binding/v1/           # local mapping plus memory/WebSocket fixtures
└── docs/stories/
    ├── S-032-sip-driver-terminates-one-governed-call.md
    └── S-033-neutral-rtvbp-bridges-the-call-to-an-application-channel.md
```

The native capability Provider is B10x, with permanent authority `io.b10x`. Asterisk's
separate `org.asterisk.ari` authority remains attached only to its ARI surface. Its pinned
first-party SIP and RTP samples provide interoperability evidence for the configured development
peer, not ownership or provenance for `sip.dial`. No `vendor/sipx`,
`vendor/rtvbp`, Git submodule, `voice` repository, substrate protocol module, dynamic plugin, or
out-of-process gateway artifact is added by this plan.

The owner contract directory never contains SIP or RTVBP cases. The two fixture directories prove
their respective adapter bindings against that contract; they are released as evidence alongside
the contract but remain distinct artifacts.

## 3. Dependency graph

The arrows below describe production dependencies. The supervised leaf is the only production
crate that joins both adapters; its loopback fixture proves the complete model-free path.

```text
server ───────────────▶ service ─────────────▶ domain
  │
  └── admitted voice proof ─▶ voice-runtime
                                ├──▶ driver-sip ─────────▶ domain
                                │       └──▶ exact sipx crates
                                └──▶ rtvbp-voice-endpoint ▶ domain
                                        └──▶ exact generic RTVBP Rust SDK

protocol ─▶ domain identities / bounded projections only
driver-sip ─X─▶ RTVBP
Agent/Substrate ─X─▶ sipx or RTVBP

connectors binary ─▶ server local operation port
                  └▶ voice-runtime (never the two adapters directly)
```

`domain::voice` separates a protocol-neutral internal `TelephonySession` from the released
cross-repository `VoiceSession` semantics. `driver-sip` implements the first.
`rtvbp-voice-endpoint` maps the second onto the generic RTVBP runtime while consuming the first.
`server` alone joins Grant admission with deployment-selected SIP and application routes into a
non-serializable `AdmittedVoicePlan`. `voice-runtime` alone consumes that proof and composes the two
adapters. No domain request, model, or caller names a Rust crate, RTVBP method, endpoint, or upstream
implementation.

Tenant identity is carried inside the private `AdmittedOperation` grant evidence and projected into
`AdmittedSipPlan`. `VoiceApplicationRoute` deliberately has no organization/tenant field: the
runtime can only issue application authority for the organization admitted with the principal and
Grant, so a deployment route cannot accidentally or maliciously substitute a second tenant.

`rtvbp-voice-endpoint` is a nested Cargo workspace rather than a member of the deterministic
catalog/compiler workspace. The final SDK enables `serde_json/preserve_order`; Cargo feature
unification would otherwise change OpenAPI object traversal and canonical artifact bytes. CI builds,
tests, lints, formats, and checks the nested lock independently. This is dependency-closure
isolation, not a process/plugin boundary or separate runtime trust decision.

`driver-sip` is independently nested for the analogous network/dependency fence: its large socket
and media closure never enters the deterministic catalog compiler lock. CI treats both nested
workspaces as required components. A source fence keeps the production `sipx_transport::bind`
inside `driver-sip`; the supervised runtime test is the one explicit loopback peer fixture.

`voice-runtime` is a third nested workspace because it deliberately joins those two isolated
closures without feature-unifying either into the catalog compiler. It owns operation-scoped
credential resolution, ephemeral proof material, authority issuance, the deployment-owned
DNS/TCP/proxy/TLS connector port, authenticated upgrade, initialization, media/control/signal and
keepalive pumps, lease expiry, first-wins termination, teardown, and payload-free observation. It
owns no SIP, RTVBP, or product semantics.

`connectors-cli` is a fourth nested workspace and the product composition leaf. It reads the
canonical B10x member, verifies the exact personal owner snapshot, Connection initiation,
Grant reference, description lease, external approval reference and configured alias before
constructing an admitted voice plan. It depends on `voice-runtime`, not directly on either adapter.
The owner-only Unix socket is in `server`; the binary owns deployment config, authority signing-key
custody, exact TCP/TLS application routing, live session handles and a payload-free audit journal.
No-config personal mode binds safely but advertises no operations. A held owner-only state lock
prevents a second process from unlinking the live daemon socket, and shutdown revokes and boundedly
joins backend-owned sessions before removing that socket.

The same owner signal reaches credential lookup, SIP establishment, application connection, and
the established supervisor. SIP admission bounds an invitation to `1..=30` seconds, and the driver
passes cancellation to sipx's `dial_until`: an outstanding INVITE is withdrawn with CANCEL before
the driver returns, including sipx's crossed-success ACK/BYE cleanup. The neutral telephony port
also exposes the driver's first typed terminal fact; media or signal EOF never guesses
`remote_hangup` and therefore cannot erase `transport_lost` or another exact cause.

An established sipx call task is likewise owned by its `TelephonySession`: termination waits for
the driver task through endpoint shutdown, and dropping the last session aborts a remaining owner.
The typed terminal observation may arrive before that join, but runtime teardown does not report
clean completion until the socket-owning task has ended.

Design 02's original literal socket-opener rule is refined because the pinned
[`sipx-transport` API](https://github.com/codewandler/sipx/blob/004ac534b8b222060ad2d2308763efe6e1dedc10/crates/sipx-transport/src/lib.rs)
owns sockets and performs `bind(Config)`. `server`/`service` remains the sole admission and
destination-policy path; `voice-runtime` is the sole credential-custody and adapter-composition
path. `driver-sip` is the one explicitly
network-classified driver and may call `sipx` bind only with a non-serializable, proof-bearing
`AdmittedSipPlan` returned by that path. Catalog data, a wire DTO, and a caller cannot construct it.

The admitted plan fixes local signaling/media listener apertures and the destination policy applied
to configured, DNS-resolved, SIP-learned, and SDP-learned targets before transmission. Because the
selected media runtime can learn a symmetric-RTP source internally, stable evidence must separately
prove that source admitted before it becomes an egress peer. Fence tests reject socket-capable
dependencies in all other production drivers/platform crates and direct production `sipx` binds
anywhere else.

The application WebSocket transport owns both pump task handles. Normal close gives its finite
control queue one bounded deadline and joins completed pumps; timeout or a straggling terminal pump
is aborted and boundedly joined before `close()` returns. Dropping the last transport handle also
aborts them, including when an outer timeout cancels `close()`, so a stalled `AsyncWrite` cannot
retain the split stream. Application-to-call media
saturation is the binding's `output_overload` case: the read pump records loss, selects
`media_overload`, stops reading later wire frames, and wakes the supervisor immediately. A later
application close consequently cannot replace an earlier overload merely because a polling
interval elapsed.

## 4. Catalog and admission

`sip_v1` is a closed catalog `protocol_driver` value. It is not itself a provider. The
repository-owned B10x Provider declares the capability; a carrier, Asterisk, SBC, or other
PBX is a Connection-owned peer and does not become the operation's Provider merely because it
speaks SIP. RTVBP and `VoiceSession` are not separate providers or callable operations; they are
the post-admission binding and semantic contract behind the SIP member.

The repository-authored B10x declaration uses:

- interaction shape `session_establishment`;
- protocol driver `sip_v1`;
- implementation form `built_in`;
- deployment-selected local or satellite placement;
- explicit public/private network, listener, port range, and secret capability facts.

Its outbound call-establishment Operation has the stable catalog id `sip-dial`, projected to the
harness as `sip.dial`, with `expose = true`, write/network and external-send effects, reviewed high
risk, and non-idempotent replay behavior. Its only caller field is `target`: a 1..=64 byte opaque
alias that the selected Connection maps to the exact SIP identity, clear transport, signaling
address, From identity, media address aperture, and optional credentials. A URI, host, port,
credential location, placement, network class, tenant, or protocol cannot cross that caller
boundary. Its
inbound call surface is a declared provider channel/session binding tied to the same Connection and
tenant/application route; it is not an ambient listener outside the catalog. `SOURCES.toml`, the
provider source/provenance record, `providers/<voice-provider>.toml`, the generated canonical
document, `connectors.lock`, the packed catalog, and the web projection land atomically.

The `sip_v1` vocabulary may land before the runtime so documents and consumers can represent the
accepted closed driver. A provider member must not land until S-024's plan/dispatch seam and S-032's
driver make that member callable: the catalog never advertises an implementation that would only
refuse as missing. Conversely, S-032 cannot complete—and no native SIP support can be claimed—until
at least one such canonical provider member is served and discoverable through the effective
catalog.

The Connection separately declares whether `b10x`, `provider`, or both may initiate. A
B10x/harness call must pass both that Connection gate and the operation Grant; neither is a
substitute for the other. Inbound dialogs match exactly one configured
Connection plus tenant/application-channel binding; zero or multiple matches refuse before product
work. SIP credentials and SRTP material never enter protocol DTOs, RTVBP frames, events, audit,
fixtures, or Agent input.

## 5. Generic contract and RTVBP binding

`VoiceSession` is the canonical semantic contract. It has opaque call/session/channel references,
explicitly untrusted participant context, bounded media negotiation and duplex frames, optional
channel signals, output interruption, observable loss, and typed termination. It has no SIP, RTP,
RTVBP, carrier, credential, IVR, recording, transcript, tool, or Agent-lifecycle type.

RTVBP provides reusable envelope/runtime/transport machinery. `rtvbp-voice-endpoint` owns the
voice-side mapping from `VoiceSession` to the exact B10x profile
`b10x.voice.v1`; the AI Agent Platform owns an independently implemented application-side
mapping. The binding manifest and fixtures ship with the Connectors owner bundle and both adapters
must pass the same generic semantic vectors. No B10x catalog needs to be compiled into the
upstream SDK.

The first implementation profile maps signed 16-bit little-endian PCM at 8 kHz, mono, 20 ms and
320 bytes per frame. This is one negotiated `VoiceSession` descriptor rather than the identity of
the generic contract. The upstream `L16/8000/1` label stays inside the binding. The frozen
`babelforce.v1` catalog includes application movement and session-variable semantics and remains
only in the downstream Babelforce adapter.

[Legacy RTVBP WebSocket negotiation](https://github.com/babelforce/rtvbp/blob/dc0a60f7425b4899885f372152028457791b1e72/docs/designs/multi-catalog.md)
maps an absent subprotocol to `rtvbp.v1`/`babelforce.v1`. The generic endpoint requires an explicit
exact local binding profile and refuses a headerless offer instead of inheriting that compatibility
default. The SDK's generated `demo.v1` profile is a multi-catalog test and is not an application
contract.

## 6. Authority, placement, and lifecycle

The 60-second, proof-bound, single-redemption authority from architecture ADR 0016 binds the exact
endpoint, tenant, actor, Connection, Grant, operation, channel kind, RTVBP profile, proof key, and
session lease. The connecting endpoint generates the proof key and presents the authority plus DPoP
on the exact WebSocket upgrade. Only the serving endpoint atomically redeems `(iss, jti)` before
accepting bytes. The first public route has the application endpoint serve and the voice endpoint
connect; a private satellite therefore connects outward. Authority expiry controls establishment;
hangup, cancellation, revocation, lease, and bounded drain control an established session.

A public gateway may dial or accept the independently reachable application endpoint according to
the admitted plan. A satellite beside a private PBX initiates RTVBP outward. SaaS never dials into
the private network, and the federation control stream never relays RTVBP or audio. No permitted
direct route produces `session_unserved`.

Each call task belongs to one gateway/application generation until hangup or the bounded drain
deadline. New calls move to a ready successor; a live SIP dialog and its media are never migrated.
Media/frame/queue/request limits are finite. Caller-to-application input loss remains the generic
bounded-loss/degradation case. Saturating the voice-side application-to-call queue cannot silently
drop synthesized speech, so that binding selects typed `media_overload` and closes. Application-side
speech detection owns a barge-in request; the voice endpoint clears bounded playback, while Agent
steering/cancellation remains a separate fact.

## 7. Dependency baseline and release gates

The reviewed initial identities are:

| Dependency | Exact identity | Resolved commit | Gate |
|---|---|---|---|
| `codewandler/sipx` | `v1.0.0-rc.23` | `004ac534b8b222060ad2d2308763efe6e1dedc10` | development characterization only while prerelease |
| RTVBP Rust SDK | `sdk/rust/v0.1.0` | `dc0a60f7425b4899885f372152028457791b1e72` | final release; generic runtime baseline; local binding only |

Both declare Rust 1.88. This workspace now declares 1.88 and its pinned-MSRV lane is the release
gate. Cargo versions are exact, the
lockfile records the resolved graph, and release evidence records licenses, source commits, local
binding identity, conformance results, and artifact digest. RTVBP's released crate asset is pinned
at SHA-256 `7d1d675e359016a5c8711bc0a29783ad9ce57a2f80f47ab5c77bc0152935ff9b`.
Its public generic request/event traits, handler registration, transport traits, and configurable
WebSocket subprotocols are the executable seam for the local binding. A mutable Git ref or
compatible range is refused. The selected RTVBP runtime has internal unbounded control/transport
queues; stable or exposed support requires bounded configuration/change, a bounded replacement of
those layers, or measured process containment. A stable Connectors voice claim also requires a
stable `sipx` API or a reviewed compatibility/upgrade exception.

`cargo audit` reports no known vulnerability in this locked voice graph, but it reports the
unmaintained `bincode 1.3.3` through RTVBP's `webrtc 0.14.0` → `dtls 0.13.0` closure
(`RUSTSEC-2025-0141`). Stable or exposed voice support additionally requires an upstream
replacement or a reviewed maintained fork and a repinned owner bundle.

## 8. Implementation sequence and exit evidence

1. The development baseline is implemented: exact dependency pins, admitted loopback plan, outbound
   SIP dialog, G.711-to-neutral PCM normalization, bounded RTVBP transports, independent application
   adapter, and a supervised authenticated duplex SIP → RTVBP → fake-application test with one
   terminal event and whole-task teardown.
2. Complete S-032's remaining loopback matrix: registration, inbound dialog, DTMF, authentication
   refusal, reconnect, learned-peer refusal, and overload/interruption teardown. Outbound ringing
   cancellation, including observed SIP CANCEL, is now covered.
3. Complete S-033's cross-repository lifecycle vectors, including lease, revocation, generation
   drain, and an outward satellite/unserved fixture. The local WebSocket now proves immediate
   causal overload and bounded joined teardown even when writes never progress.
4. The repository-authored B10x Provider declaration, generated `sip-dial` member, source
   pin, lock row, pack, and web projection land atomically with the runtime proof. Asterisk's
   separate source remains interoperability evidence for one configured peer.
5. The operator-authorized non-loopback mode has completed one exact dev-cluster TCP SIP and RTP
   echo characterization. It is intentionally not a stable-network claim: the route remains
   deployment-owned, exact-aperture, and explicitly marked development.
6. The released-alpha search/describe/invoke plus generic session contract is bound to an
   owner-credentialed local Unix socket. The configured backend returns an `execution_ref`, keeps
   the `VoiceRuntime` task and termination control, and reports `outcome_unknown` rather than
   inventing ownership after restart. Clean-room harness conformance and durable hosted
   reconciliation remain release gates.
7. Publish a signed Connectors owner bundle and prove a clean-room application-channel consumer
   before any stable, hosted, or authoritative-writer claim.

The Babelforce compatibility adapter is a downstream follow-on. It neither blocks the generic
loopback proof nor changes the ownership of `babelforce.v1`.

**2026-08-15 composition-boundary amendment.** The repository tree and dependency narrative in
sections 2 and 3 predate the reusable runtime split. `service` now owns authority, dispatch, SIP,
and voice application logic; `server` owns only inbound local and hosted transports. The focused
`integration-sip` adapter joins that application logic to `voice-runtime`, while
`connectors-runtime` validates configuration and builds the exact adapter registry. The
`connectors` binary is only a command-line surface over `connectors-client` and
`connectors-runtime`; it no longer owns SIP policy, session state, runtime launch, or deployment
composition. Examples live with `connectors-config`. This amendment supersedes the older physical
paths without changing the protocol-neutral voice or exact-driver dependency fences.
