---
format: aep.planning-md/1
id: story:local-approval-binding
kind: story
status: implemented
title: Verify canonical approvals and spend them once for prepared attempts
relations:
- decomposes: initiative:complete-local-connectors
- depends_on: story:local-mutation-ledger
- serves: vision:independent-contract-adapters
- depends_on: story:local-execution-audit
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/service/delegation.md
- confidence: inferred
  path: crates/connectors-host
- confidence: inferred
  path: docs
- confidence: inferred
  path: ess/domains/delegation.yaml
- confidence: inferred
  path: ess/domains/mutations.yaml
- confidence: inferred
  path: website
revision: 6
---
## Outcome

Implement the local host's canonical approval proof and durable one-use spend binding needed by GitLab governed writes, using the selected F03 framing and the existing mutation/audit foundations at main 631caabffa46702e5dddeb02a1a3aa7d0547e05c.

## Acceptance

Against a real private SQLite authority, a correctly signed exact approval subject can produce at most one acknowledged spend receipt for its original live prepared attempt across concurrent calls and restart, while malformed, changed, expired, revoked or ambiguously acknowledged proof/spend state cannot open an approved dispatch gate.

## Model and reviewed binding

Ownership remains contracts/service/delegation.md, ess/domains/delegation.yaml CanonicalApprovalSubject, ApprovalClaims and ApprovalRedemption, and ess/domains/mutations.yaml AttemptRecord. The existing bounded clock interval supplies trusted time evidence; a deployment must qualify its source. Pinned ESS validation and compilation pass before this decomposition. ConfiguredApprovalKey is a new current trusted configuration value, not a discovered key, issuer entity or caller admission. It binds the configured issuer, audience, kid, Ed25519 public key, validity bounds and revocation fact; the consumer's policy guard owns its authority and serialization.

Review of the existing ledger showed that an unkeyed attempt retains target/input fingerprint but no complete approval authority/origin subject. Approval-aware preparation will therefore retain the full immutable canonical subject in the existing AttemptRecord value, including unkeyed writes. Existing records have an absent capture and remain observable/recoverable; no migration invents a subject or upgrades them into spend authority. Current policy still reconstructs and verifies the complete subject at spend. This closes the durable binding gap without creating a second attempt or changing keyed replay identity.

Implement exact mandatory-null closed canonical subjects and compact JWS with fixed Ed25519 algorithm, b10x.connectors-approval.v1+jws type, configured issuer/audience/key selection, bounded canonical base64url/JSON and strict duplicate/unknown/member and integer rejection. Reuse the existing sorted-key canonical rule and already-pinned ring implementation; verify the selected RFC identifier and independent standard byte vectors. Signing mechanics use a purpose-specific protected key capability and a fresh cryptographically random reference. They are not an authorization UI or an externally admitted issuer service.

The receiver policy binding returns a guard that holds its current authority/key decision through the spend transaction acknowledgement. Recheck exact subject, signature, key validity/revocation and bounded clock interval inside the spend decision; no earlier verifier receipt is a lease. Enforce exact nbf=iat-5 and exp=iat+295, integer/unit arithmetic and nbf<=lower with upper<exp. Uncertain clock evidence grants no spend, and no raw SystemTime default is supplied.

Add migration six for the optional prepared subject and immutable ApprovalRedemption rows. Preserve migrations one through five, all authority identities and old read/observation behavior. Only admitted approval-aware preparation installs the port. Enforce the exact tagged issuer/reference tuple and one redemption per Prepared attempt atomically under the same local metadata authority, with separately acknowledged audit, preparation, spend and dispatch decisions. Retain tombstones without automatic expiry or eviction; configured capacity refuses new spend. Redemptions reference the independently retained instance/attempt and hold no signing seed, proof evidence, credentials or native result.

Spend accepts the original opaque live preparation handle and produces a non-Clone process-local receipt only after definite acknowledgement. Its consumption is required by a separate approved-dispatch entry point; ordinary ledger dispatch refuses required/event-claim modes without the corresponding selected receipt. Unknown acknowledgement and recovery cannot reconstruct the receipt, refund the reference or repeat a business effect. Existing-key replay remains before approval verification and cannot spend again. Event-claim and gateway-delivery proof support remain unadvertised.

## Verification prerequisites

Test canonical byte/nullable/Unicode identity, every subject coordinate, RFC Ed25519 vectors, fixed type/algorithm/issuer/audience/key selection, duplicate/unknown JSON, noncanonical encodings and numeric bounds, current clock interval/expiry boundaries and key/policy changes between initial verification and spend. Use real SQLite and bounded threads/processes for same-reference/different-attempt and same-attempt/different-reference races, immutable tombstones, lost acknowledgements and rollback, one live spend receipt, old/missing subject refusal, guarded dispatch, abort and restart recovery without refund/resend, capacity and migration continuity. Signing fixtures are cryptographic evidence, not protected persistent issuer or provider acceptance. Run the required gate including Rust 1.88, affected ESS/generation/conformance, website/reference checks and existing optimized CLI journeys with task-owned TMPDIR and two Cargo jobs; retain failures, commands and identities under docs/evidence/local-approval-binding-20260910.

## Scope, sequencing and remaining ownership

Single implementation and planning writer on primary main; no concurrent implementation is scheduled. Inferred surfaces are crates/connectors-host, Cargo.lock, contracts/service/delegation.md, ess/domains/delegation.yaml, ess/domains/mutations.yaml, docs and website. Overlap with mutation/audit and earlier GitLab runtime work is explicitly serialized, including Cargo builds and CLI fixtures sharing binaries. The four planning critics are read-only.

The parent retains protected persistent local issuer-key custody/publication and rotation, actual local approval UI/CLI preparation and issuance, authenticated caller policy bindings, production clock qualification, generated mutation ingress and the full connection-bound provider dispatch coordinator, then native GitLab validation/writes and dedicated sandbox acceptance. This private proof/spend port does not advertise those surfaces or close full GitLab. C14 create/update head-guard semantics remain unresolved; no weaker race guarantee is assumed. GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider order and all original acceptance/reproducible-delivery requirements remain unchanged. No Connectors publication, deployment, paid governed run or approval bypass is selected.


## Implementation checkpoint — 2026-09-10

The private approval binding is implemented and verified in docs/evidence/local-approval-binding-20260910/README.md. Seventeen focused tests pass, including four actual process exits, current authority/key/time checks, exact canonical framing and RFC vector, one-use races, immutable retention, lost acknowledgement, old/missing capture, corrupt-storage refusal and original-receipt dispatch. The complete repository gate passes with Rust 1.88; the host suite has 75 passes and six ignored auxiliary/integration entries. Website/reference checks and all 15 authored example tests pass. All five optimized production GitLab CLI journeys pass in 85.17 seconds against disposable HTTPS and qualified Secret Service fixtures.

Preparation, spend and dispatch retain distinct acknowledgements. The final receiver policy guard remains held through spend acknowledgement; clock expiry or uncertainty after commit leaves the reference spent and returns no receipt. Required/event-claim candidates cannot use the ordinary gate. Complete approved preparation captures the full authority/origin subject for unkeyed attempts; old captures remain absent. Faults never create a reconstructible send capability. Persistent issuer custody, actual approval CLI and provider composition remain parent-owned next steps.

All four second-round critics approve. The design critic's first-round missing dependency on the audit migration was fixed and recorded through AEP; no findings remain open in this decomposition. The inherited-model and three-slot scheduling deviations are recorded with the evidence. No approval bypass, Connectors publication or provider sandbox completion is claimed.
