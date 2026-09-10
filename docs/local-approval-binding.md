# Local approval proof and spend binding

The host implements private Rust ports for canonical approval signing,
verification, one-use redemption and the approved mutation-ledger gate. They
follow [the F03 contract](../contracts/service/delegation.md) and its
[typed model](../ess/domains/delegation.yaml). The CLI still exposes reads;
these ports alone do not authorize provider writes or delegated ingress.

## Exact proof and current admission

`Subject` captures all target revisions, connection, caller/tenant/realm,
current authority and executor snapshots, direct/federated origin, optional
route, canonicalization, input digest and approval mode. Identifiers preserve
exact UTF-8 bytes. Missing optional coordinates have mandatory explicit null
members in the signed JSON. Strict duplicate/unknown-member decoding followed
by exact canonical reserialization refuses omissions, alternate numeric or
escape spellings, whitespace and member ordering outside the selected format.

Compact JWS uses exactly three canonical unpadded base64url segments, fixed
`Ed25519`, fixed `b10x.connectors-approval.v1+jws`, and a closed header and claims
shape. Bounds are 512 bytes for the decoded header, 8 KiB for the subject, 12 KiB
for claims and 18 KiB for compact evidence. The configured public key has 32
bytes and signatures have 64. The fully specified algorithm name follows
[RFC 9864 section 2.2](https://www.rfc-editor.org/rfc/rfc9864.html#section-2.2);
tests check an independent [RFC 8032 vector](https://www.rfc-editor.org/rfc/rfc8032.html#section-7.1).
The host uses the existing pinned `ring 0.17.14`, with no new crypto algorithm.

The receiver supplies the expected complete subject and a `ReceiverPolicy`.
Its current admission guard selects a trusted configured issuer/audience/key
and serializes authority and key changes until dropped. The bounded `kid` is
only a configuration lookup hint; proofs cannot select their own trust or cause
network key discovery. A separate `IssuancePolicy` owns issuance authorization.
No default allow-all policy is supplied. The embedding coordinator must obey
metadata-before-policy lock ordering and perform bounded local admission work.

Signing consumes a purpose-specific protected seed capability and generates a
fresh 256-bit random reference using the operating system random source through
ring. Proof bytes and the seed have no Debug/Clone/serialization implementation
and remain outside SQLite. This in-memory signer does not provide persistent
issuer custody, publication, rotation, human approval or a public issuer API.

Both issuance and verification require trusted time containment within a maximum
four-second interval, with checked integer/unit arithmetic. There is no default
SystemTime source. Issuance uses the midpoint floor in seconds; `nbf=iat-5` and
`exp=iat+295` are exact. Reception requires `nbf<=lower` and `upper<exp`, with no
extra skew. Current configured key validity and revocation also apply. Production
clock and policy qualification remain consumer obligations.

## Separate durable decisions

Only approval-aware preparation installs SQLite migration six. Previous migration
bytes and authority identities are preserved; normal setup remains version three.
The migration adds the optional full subject to existing attempts and a separate
immutable redemption table. Old records remain observable and recoverable, but
their absent subjects are never invented during upgrade.

Initial verification binds the exact proof digest and complete subject to an
opaque live preparation. Approval-aware preparation compares its candidate's
namespace and target/input fingerprint and captures that subject for keyed and
unkeyed attempts. This evidence is historical association, never current policy.

Spend accepts the original preparation handle. It checks its process, authority,
nonce, still-Prepared state, subject and current connection publication fence.
It re-verifies the exact original proof against fresh consumer policy, key and
clock inside the local metadata transaction, then checks time again immediately
before commit and after commit before returning a receipt. Late expiry or lost
clock certainty leaves the reference spent without granting dispatch. The
consumer policy guard remains held through acknowledgement.
No provider call runs while metadata or that guard is held.

SQLite enforces exact `(issuer,reference)` uniqueness and one redemption per
attempt. Kid, ingress alias, route and request id cannot create separate spend
namespaces. Each row retains its instance, attempt, canonical subject and time
observation, without proof bytes, signing seeds or credentials. Update/delete
triggers preserve tombstones; no expiry or eviction API exists. Trusted capacity
configuration allows 1–100,000 retained redemptions per instance and refuses
new spends when full.

Only definite acknowledgement returns a non-Clone, nonserializable, process-local
spend receipt. Each live preparation permits one spend call; any refusal or
uncertain result requires abort/recovery, so a retry cannot manufacture a new
acknowledgement after loss. A new admitted attempt still meets durable reference
uniqueness. Recovery never recreates a receipt or refunds a consumed reference.

The separately acknowledged approved gate consumes both original handles and
rechecks their exact association, durable redemption and current connection
fences before the Prepared-to-Dispatching CAS. Ordinary dispatch refuses required
and event-claim modes. Audit admission, preparation, spend, dispatch and settlement
retain distinct commits; the coordinator must additionally own current caller,
credential, audit and provider capabilities. A spent approval followed by abort
remains spent with no attempted business effect.

## Verification and remaining delivery

The Rust tests use actual private SQLite authorities, deterministic clocks and
configured policy fixtures. They test proof bytes, current changes, concurrent
spends, lost acknowledgements, rollback, immutable retention, original-handle
association and four actual child-process crash boundaries. A simulated effect
file establishes no native provider acceptance. These are local single-authority
guarantees; divergent backups/failover must not activate the same leaf identity.

Protected persistent issuer custody and rotation, actual local approval CLI,
authenticated caller policy, production clock qualification, generated mutation
ingress and the complete connection-bound dispatch coordinator remain required.
Native GitLab writes and dedicated sandbox acceptance follow those foundations.
C14 create/update head-guard semantics remain unresolved; no race window has
been accepted. GitLab remains first, followed by Kubernetes, PostgreSQL, MCP and
the remaining providers.
