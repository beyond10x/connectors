# RTVBP adapter design

**Status:** proposed, not implemented. Native authority lives in
[the session binding](contracts/session/v1alpha1/semantics.md).

## Contracts

| Contract | Profile / use | Why |
|---|---|---|
| `sessions` | `duplex_transport` binding over RTVBP's envelope; `outbound` (connect outward to the application endpoint) | carries the session control across RTVBP |
| `media` | `pcm-s16le-8k-mono-20ms` mapped to `b10x.voice.v1` (`L16/8000/1` label stays inside the binding) | same semantics, independent implementation |
| `auth.capability` | `session-authority` (present with DPoP on upgrade), `inbound-verifier` (when serving) | proof-bound establishment |
| `auth.profile` | `rtvbp.session_authority` (`session_authority`, subject `none`) | no vendor credential; host-issued per session |

RTVBP selects no `auth.evidence` check. Atomic one-use redemption is an
`inbound-verifier` capability decision backed by the session-redemption owner; a
durable ledger does not turn it into an eighth readiness-evidence name.

## Session and media protocol messages

Offer, accept, reject, cancel and close are session control messages. Media
`signal` and `interrupt` are negotiated media control messages. They use the
versioned RTVBP duplex binding and are not ordinary operation ids; an independently
admitted operation would need a separate declaration.

Bridge (composition, not an adapter): consumes `media` twice and `sessions` for joint termination; no auth contracts of its own.

Local audio (composition binding): consumes `media` with the same profile; device selection is deployment configuration, never caller input (`crates/driver-audio/README.md`, enumeration reserved).


Independent outbound application transport implements b10x.voice.v1 over shared
session/media contracts. Own native configuration, source pins and fixtures here.
[Media composition](../../docs/compositions/media-session.md) owns concrete
pairings; neither SIP nor this adapter imports the other. Federation relays neither
RTVBP nor audio; live dialogs do not migrate.
