# Design 03: beyond HTTP — drivers, satellites, and the byte plane

**Status:** draft for review; v1 direction fixed, external runtime artifacts deferred · **Date:** 2026-08-13
**Inputs:** the predecessor rich-runtime program; B10x architecture repository boundaries;
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
implementation, but a model, Service Account or invocation request cannot choose a driver, artifact,
credential, destination class or worker.

## 2. Protocol drivers are closed platform code in v1

Request/response over a non-HTTP transport is still an **Operation**: parameters enter, the driver
constructs one protocol request, and a declared result or error returns. What varies is framing and
lifecycle, not the grant model.

- The platform ships a **closed, versioned set of protocol drivers**. HTTP is driver one. Asterisk
  AMI is the leading first non-HTTP proof; DNS is another plausible small driver. Further drivers
  arrive only for measured provider needs.
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

## 3. Connector plans are data; substrate execution is generic

The canonical document carries enough information to produce a zero-I/O **connector execution
plan**: provider/member identity, interaction shape, driver identity, capability requirements,
connection/configuration addresses, lifecycle bounds and result/error contracts. It never carries a
credential value or executable behavior.

Connectors admits the principal and grant, resolves the connection, and dispatches that plan through
its closed driver registry. A missing driver or capability is a named refusal before credential
access; it never falls back to HTTP, Flux, an ambient executable or another billing/authority domain.

`b10x/substrate` is a separate, Flux-free execution service. It knows how to run and observe
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

Federation later makes the topology appear as one effective catalogue. The main deployment imports
the satellite's attested catalogue generation and invokes/subscribes through a bounded remote seam
under its own grants and audit. Until federation exists, clients may connect to two deployments.
That is an honest temporary topology and creates no second connector model.

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

This byte-plane split covers PTYs, port forwards, raw tunnels and RTP/audio without turning the
credential broker into a high-volume proxy. Asterisk's ARI `externalMedia` and AudioSocket are
concrete examples: the platform establishes the leg; the media engine carries media.

## 6. Asterisk as the proving sequence

- **ARI** already fits the HTTP operation plus WebSocket Event/Channel model, including establishing
  audio legs through `externalMedia`.
- **AMI** is the leading first non-HTTP driver when its operational surface is required: a TCP
  session, correlated actions as Operations, and AMI events as declared Events.
- **FastAGI** is an inbound session transport only if legacy dialplans require it. It generalizes the
  owned inbound transport topology; it is not a reason to invent a raw framing DSL.
- **SIP/media internals** remain out of scope. B10x is a control plane for existing media
  systems, not a telephony implementation.

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

## 8. Ownership across the B10x stack

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
4. Prove one real non-HTTP driver—prefer AMI when demand is concrete—with unary actions, events,
   refusal tests and no generic framing language.
5. Add session/byte-plane establishment against a concrete terminal, tunnel or media journey.
6. Consider external runtime artifacts only after the built-in driver seam has real pressure and the
   separate security decision is accepted.

This design does not schedule an AMI implementation or accept external runtime artifacts. It fixes
where those concerns belong so later work does not re-derive the old mixed runtime model.
