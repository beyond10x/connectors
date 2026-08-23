# Design 03: beyond HTTP — drivers, satellites, and the byte plane

**Status:** accepted v1 design; native-voice amendment accepted; external runtime artifacts deferred · **Date:** 2026-08-13
**Inputs:** the predecessor rich-runtime program; the architecture boundaries pinned from the b10x monorepo;
how AMI/AGI (Sangoma Asterisk), DNS/UDP, SIP, TCP protocols and audio channels reach the connector
model.

HTTP is the first connector protocol, not the definition of a connector. An external capability
remains a Provider with Operations, Events, credentials, grants, risk facts and audit regardless of
how bytes reach the vendor. The platform must therefore grow beyond HTTP without turning provider
documents into programs or restoring an arbitrary plugin host.

The predecessor's `http | socket | process | container | plugin | remote` runtime enum captured a
real need but mixed protocol, execution, implementation and topology into one word. This design
keeps those axes independent.

## 1. Five independent axes

| Axis | Answers | Initial vocabulary |
|---|---|---|
| **interaction shape** | What lifecycle does the member expose? | unary operation, stream, subscription, leased session, session establishment |
| **protocol driver** | Which closed protocol implementation speaks to the external system? | HTTP; later named drivers such as DNS or Asterisk AMI |
| **placement** | Where does execution happen? | this deployment, an isolated substrate workload, a federated satellite |
| **implementation form** | How is the driver supplied? | built into connectors for v1; an attested out-of-process artifact only after a later decision |
| **required capability** | Which host authority must be granted? | public/private network, Unix socket, file-shaped secret, process, container, device-specific capabilities |

None is caller input. The compiled connector document fixes the interaction shape, driver and
required capabilities. Deployment policy resolves placement. An operator may install or admit an
implementation, but a model, Identity service principal, connector Grant, or invocation request
cannot choose a driver, artifact, credential, destination class, or worker.

## 2. Protocol drivers are closed platform code in v1

Request/response over a non-HTTP transport is still an **Operation**: parameters enter, the driver
constructs one protocol request, and a declared result or error returns. What varies is framing and
lifecycle, not the grant model.

- The platform ships a **closed, versioned set of protocol drivers**. HTTP is driver one. SIP is the
  selected first non-HTTP/session proof under
  [Design 05](05-native-sip-and-rtvbp.md); Asterisk AMI remains a plausible later unary/event proof,
  and DNS another small candidate. Further drivers arrive only for measured provider needs.
- The catalog configures a named driver with data. An AMI action's fields, result and risk are
  declared; TCP framing, login state and `ActionID` correlation are implementation.
- A generic raw-TCP, arbitrary-code or declarative-handshake language is refused. Such a language is
  an interpreter and an open plugin model hidden in data.
- A vendor ritual that cannot be declarative gets a small **owned transport arm** behind the same
  seam. Slack Socket Mode is the predecessor example: special handshake code, generic grants,
  connection custody, events, replay and lifecycle on both sides of it.
- Risk, idempotency, effects, grants and audit are transport-blind. `AMI Originate` is not made safe
  because it is framed differently from `QueueStatus`.

This refines the v1 non-goal: connectors has no arbitrary or in-process plugin runtime. Built-in
drivers and reviewed owned transport arms are ordinary platform implementation behind a closed
registry.

The first such arm is now the personal-local Slack Socket Mode alpha in
[Design 06](06-personal-slack-socket-mode.md). It adds no generic handshake language: the catalog
declares channel credential names and closed events, while reviewed Slack code owns only the fixed
ticket exchange and transport envelope. General M3/M4 persistence and hosted serving remain open.

## 3. Connector plans are data; substrate execution is generic

The canonical document carries enough information to produce a zero-I/O **connector execution
plan**: provider/member identity, interaction shape, driver identity, capability requirements,
connection/configuration addresses, lifecycle bounds and result/error contracts. It never carries a
credential value or executable behavior.

Connectors admits the principal and grant, resolves the connection, and dispatches that plan through
its closed driver registry. A missing driver or capability is a named refusal before credential
access; it never falls back to HTTP, Flux, an ambient executable or another billing/authority domain.

`beyond10x/substrate` is a separate, Flux-free execution service. It knows how to run and observe
bounded processes, containers and workloads; it does not understand providers, connector grants,
vendor handshakes or catalog documents. Built-in drivers do not require substrate merely because
they open a socket. If a later connector driver is supplied as an executable artifact, connectors
may ask substrate to run it behind a generic bounded contract, while retaining all vendor semantics
and authorization itself.

## 4. Reachability is placement: the satellite role

A LAN-bound endpoint is unreachable from SaaS by construction, and the destination aperture must
refuse it. The answer is not a bespoke bridge application: it is the **same connectors service
deployed near the protocol** in a satellite role. A satellite beside a PBX, cluster or private API
uses local drivers and declared destinations, then establishes an authenticated relationship upward.

Federation makes the topology appear as one effective catalogue under
[architecture ADR 0018 — Connectors satellites federate outward under bounded authority](https://github.com/b10x/b10x/blob/bf6859717f986dc0e2a3b8a713e087d426741d92/architecture/adr/0018-connectors-satellite-federation.md).
The main deployment imports a signed monotonic catalog/policy generation and invokes/subscribes
through a bounded outward-established remote seam under its own grants and audit. Until that
implementation exists, clients may connect to two deployments. That is an honest temporary
topology and creates no second connector model.

A satellite is:

- a deployment role of `connectors`, not a new repository or product;
- optionally composed with a local substrate when generic process/container isolation is required;
- registered and operated by the cloud composition layer in hosted postures;
- authenticated through identity/foundation trust, never by an invocation-supplied tenant or
  destination;
- outward-connecting from the private network. SaaS does not dial inward.

Personal, organization and SaaS postures retain the same capability set. They differ in how a
placement is fulfilled, not in which connector members exist.

## 5. Semantic streams and continuous bytes are different planes

Bounded semantic streams—operation chunks, declared Events, terminal outcomes, cancellation and
lease liveness—belong to the authenticated connector subscribe/lifecycle protocol. Continuous
terminal, tunnel or media bytes do not ride ordinary invoke or event delivery.

For the latter, connectors governs **session establishment**: authenticate, admit the grant, resolve
the connection, select the fixed driver/placement, negotiate bounded parameters, and return a
short-lived endpoint authority. Bytes then flow directly between the client and the selected local
driver, satellite or substrate endpoint. Revocation and expiry remain control-plane facts.

[Architecture ADR 0016 — Direct-byte establishment uses operation-scoped authority](https://github.com/b10x/b10x/blob/bf6859717f986dc0e2a3b8a713e087d426741d92/architecture/adr/0016-operation-scoped-session-authority.md)
fixes that authority as an asymmetrically signed, proof-bound, 60-second, single-redemption grant.
Reconnect gets a fresh grant. The serving endpoint must be independently reachable; otherwise the
session is `unserved`, and the federation control path never becomes a byte relay.

This byte-plane split covers PTYs, port forwards, raw tunnels and RTP/audio without turning the
credential broker into a high-volume proxy. Asterisk's ARI `externalMedia` and AudioSocket are
concrete examples: the platform establishes the leg; the media engine carries media.

## 6. Voice proving sequence

The original design left SIP/media implementation out of scope and treated Asterisk AMI as the
leading candidate. The cross-component native-voice decision supersedes that choice without
widening Connectors into a PBX:

- **SIP endpoint and media termination** are now in scope as one closed built-in `sip_v1` driver,
  backed by exact `sipx` dependencies. The driver is a user agent at the configured carrier/PBX
  edge; it is not a SIP proxy, registrar service, arbitrary dial proxy, TURN service, video stack,
  or tenant/product router.
- **RTVBP** begins behind the SIP endpoint as the separately authorized direct-byte boundary. Its
  local adapter binding preserves the protocol-neutral Connectors `VoiceSession` contract;
  `babelforce.v1` stays in the downstream Babelforce distribution.

- **ARI** already fits the HTTP operation plus WebSocket Event/Channel model, including establishing
  audio legs through `externalMedia`.
- **AMI** is a later non-HTTP driver when its operational surface is required: a TCP
  session, correlated actions as Operations, and AMI events as declared Events.
- **FastAGI** is an inbound session transport only if legacy dialplans require it. It generalizes the
  owned inbound transport topology; it is not a reason to invent a raw framing DSL.

## 7. Deferred extension: attested runtime artifacts

Some protocol adapters may eventually be too specialized or dependency-heavy to ship inside the
connectors binary. The predecessor's signed runtime-artifact design is valuable prior art, but it is
not a v1 commitment. Admitting it requires a separate security and supply-chain decision with at
least these constraints:

- executable or image identity is immutable, digest-pinned, signed and provenance-bearing;
- platform/runtime compatibility and entrypoint are declared; tags, ambient `PATH` and arbitrary
  host paths are refused;
- execution is out-of-process through substrate, never a dynamic library loaded into connectors;
- credentials are operation-scoped, ephemeral, absent from argv/environment/logs/results and never
  persisted by substrate or the artifact;
- the caller cannot select, install or downgrade an artifact;
- tamper, platform mismatch, absent isolation and missing capability all refuse before secret
  handoff;
- no artifact becomes a Flux release output or creates a local Flux fallback.

Until that decision exists, every supported driver is built into connectors and reviewed with the
platform.

## 8. Ownership across the b10x stack

| Owner | Beyond-HTTP responsibility |
|---|---|
| `connectors` | declarations, execution plans, built-in drivers, owned transport arms, credentials, grants, lifecycle, federation protocol and audit |
| `substrate` | generic bounded process/container/workload execution and observed state; no vendor meaning |
| `identity` | trusted principals, organizations and service identities used by deployments and satellites |
| `cloud` | hosted composition, satellite registration/placement, artifact distribution if later accepted, metering and operations |
| `agent`, Flux, autodev, product applications | consume catalogue/invoke/subscribe/session contracts; never select runtime internals |

## 9. Delivery order

1. Extend the connector document with orthogonal interaction-shape, driver and capability facts and
   define one zero-I/O plan seam.
2. Establish substrate's standalone, Flux-free minimum host execution contract; do not delay built-in
   HTTP/non-HTTP drivers on substrate when no external execution is required.
3. Design identity/cloud trust for satellite registration and federation.
4. Prove the built-in SIP driver against loopback registration, inbound and outbound dialogs,
   DTMF, bounded audio, hangup, cancellation, and refusal fixtures.
5. Prove direct RTVBP establishment against the same call with a fake application endpoint and no
   model, including replay, unreachable-route, bounded-loss, interruption, and drain cases.
6. Consider external runtime artifacts only after the built-in driver seam has real pressure and the
   separate security decision is accepted.

This design does not schedule an AMI implementation or accept external runtime artifacts. Native
voice is prepared by S-032/S-033 but remains gated by the platform-family source fences, an actual
source-grounded catalog member using `sip_v1`, the generic `VoiceSession` owner contract, the local
RTVBP binding, the MSRV change, and released owner evidence.

The ownership and five-axis model are accepted by
[architecture ADR 0010 — Beyond HTTP is a five-axis connector model](https://github.com/b10x/b10x/blob/bf6859717f986dc0e2a3b8a713e087d426741d92/architecture/adr/0010-beyond-http-is-a-five-axis-connector-model.md).
Foundation trust, channel authority, event ingestion, satellite federation, and contract release
are accepted by architecture ADRs
[ADR 0015 — Foundation services share one trust envelope](https://github.com/b10x/b10x/blob/bf6859717f986dc0e2a3b8a713e087d426741d92/architecture/adr/0015-foundation-trust-envelope.md)
through
[ADR 0019 — Foundation contracts ship as signed reproducible bundles](https://github.com/b10x/b10x/blob/bf6859717f986dc0e2a3b8a713e087d426741d92/architecture/adr/0019-contract-release-and-conformance.md).
Delivery item 1's document facts landed in S-023. The next seam and later runtime work remain owned
by S-024 through S-028 and the native-voice slices S-032/S-033 in the story board. The
connectors-owned
`fixtures/substrate-wire-0.1.0-axis-projection.json` pins substrate's owner-issued 0.1.0 bundle and
records the non-mechanical vocabulary mapping; it is not the full provider projection or S-031's
release machinery.
