# Design 03: beyond HTTP — drivers, satellites, and the media split

**Status:** exploratory sketch (nouns not yet fixed; do not build from this) · **Date:** 2026-08-13
**Prompted by:** how AMI/AGI (Sangoma Asterisk), DNS/UDP, SIP, TCP protocols and audio channels
reach the connector model.

The question decomposes into three concerns that must not be answered with one mechanism.

## 1. Transport — protocol drivers, a closed set

Request/response over a non-HTTP transport is still an **Operation**: `(params → request →
response)`. What varies is framing, not shape. The model:

- The platform ships a **closed, versioned set of protocol drivers**; `http` is driver one.
  Candidates, in likely order of value: `asterisk-ami` (TCP session, ActionID correlation,
  actions as operations, AMI events as channel events), `dns` (UDP query/response), further
  drivers strictly on demand.
- The catalog **configures** drivers with data (an AMI action's fields and response schema are
  declared exactly like an HTTP operation's params and template) and never defines framing. A
  generic "raw TCP with declarative framing" is refused on principle — that is a protocol
  interpreter grown in data, the open-vocabulary trap.
- The risk vocabulary applies with full force: `Originate` is not `QueueStatus`. Grants, audit
  and the one invocation path are transport-blind.
- **Inverted protocols** (FastAGI: the vendor system calls us mid-session and expects
  synchronous command exchanges) are owned inbound **session transports**: a channel binding
  whose sessions carry an event stream plus correlated command responses — decision 0009's
  topology generalized past webhooks, not a new concept.
- Posture gating carries over from the predecessor: locally-executing / socket drivers are
  admitted in personal and org postures; SaaS reaches them only via a satellite (§2).

## 2. Reachability — the satellite posture

A LAN-bound protocol endpoint (AMI on a PBX) is unreachable from SaaS by construction, and the
destination aperture (S-011) must refuse it. The answer is not a bespoke bridge app: it is **the
same platform binary deployed near the protocol** — a satellite. `connectors serve` beside the
PBX binds local protocols through its drivers and exposes the standard seams upward: catalogue,
invoke, subscribe, one token. Declared destinations of that deployment are the local endpoints.

Later, **federation** makes it one catalogue: the main deployment treats a satellite as a remote
runtime (the predecessor's `Runtime::Remote` intuition) and re-exports its operations under its
own grants and audit. Until federation exists, a client holds two connections — acceptable, and
no new product surface. SaaS never dials into a private network; the satellite dials up.

## 3. Media — control plane here, bytes elsewhere

Continuous bidirectional media (RTP, audio channels) is neither an operation nor a discrete
event, and it never rides the invoke path — the credential broker must not become a media proxy.

- The platform brokers **session establishment**: auth, addressing, codec parameters — all
  declarable data — and returns a media endpoint reference.
- Media then flows **directly** between the client and the media engine. Asterisk ships the
  native mechanism (ARI `externalMedia`, AudioSocket); the media engine *is* the bridge, and we
  are its control plane.
- SIP below the media-engine level (dialogs, INVITE/BYE as operations) is deliberately out of
  scope unless a real case demands it: we do not reimplement telephony.

## Asterisk, concretely

- **Today's model already covers ARI whole** (REST + WebSocket channel — the predecessor's
  connector is ARI-based), including audio legs via `externalMedia`.
- **AMI** is the first non-HTTP driver when operational surface demands it.
- **AGI** is mostly superseded by ARI; FastAGI support only if legacy dialplans require it, as
  an inbound session transport per §1.

## What this sketch deliberately does not do

Fix nouns (driver? runtime kind? satellite? federation?), specify the driver SPI, or schedule
anything. When the first non-HTTP need is real, this becomes a numbered design with fixed
vocabulary and a story; until then it records the decomposition so the answer is not re-derived.
