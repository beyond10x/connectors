---
id: S-008
title: "M3 — connect a real provider, grant it, invoke it"
pillar: Platform
status: backlog
priority:
design: docs/design/02-architecture.md
epic: build-order
areas: [domain, protocol, service, server]
note: "architecture §9 milestone M3. Exit: admit personal-local or pinned Identity authority → connect a real provider → grant → invoke, all audited. Connectors never owns the hosted login/session. This is where the product exists; everything before it is scaffolding and everything after it is reach"
---

# M3 — connect a real provider, grant it, invoke it

## Goal

Deliver the middle layer and the one invocation path: a tenant configures an integration, a human
authorizes a vendor connection through a Connect Session, a connector operator grants authority over
declared facts, and a client presents personal-local or short-lived exact-audience Identity
authority to invoke a declared operation. Neither the Identity credential nor the vendor credential
changes owners. This is the milestone at which the client contract is real.

## Acceptance

- [ ] **Integrations** exist as a first-class noun: enabled state, the deployment's BYO OAuth
      registration as file-shaped secrets, allowed scopes, settings defaults, and the destination
      policy for private-host providers. An integration **configures** a provider and can never
      extend one — a test proves surface comes only from the catalog.
- [ ] **Connect sessions**: short-lived, server-created, single-purpose, optionally bound to a set of
      allowed integrations, carrying reconciliation tags (end-user id, external ids), surfaced as a
      hosted URL and headlessly. A session never carries or returns credential material to its
      creator, and its terminal event names the connection id and nothing else.
- [ ] **Acquisition**: OAuth authorization-code and operator/API-key entry both complete through a
      connect session, storing owner-bound credentials with prepared, atomic multi-step mutations;
      refresh is owned by the platform. No memory fallback, no path inside a working tree, and a
      refusal names the address and never the value.
- [ ] **Connection lifecycle**: `created → authorized → callable`, with `degraded → reauthorize`
      repairing **in place**. Connection ids are stable across reauthorization — proven by a test in
      which a Grant and a subscription referencing a Connection survive a re-auth. Multiple labelled
      Connections per Integration are first-class, and both scopes (tenant-shared,
      principal-owned) exist.
- [ ] **Grants**: tenant-scoped, per-provider, admitting by selector over declared facts (risk
      ceiling, effects subset, idempotency) plus explicit exceptions where **deny beats allow beats
      predicate**; CAS-revisioned mutation with previewable proposals and receipts. No store bound is
      an outage (503); an empty store is a refusal (403); a refusal never names the axis that refused.
      Grants bind to connections, never to credentials.
- [ ] **Declared invoke rides one path**: personal-local authentication or Identity verification →
      admitted principal with exact Connectors audience scope → effective catalogue (sealed
      generation) → connector Grant admission through unconstructible proof types → Connection
      resolution → document → `RequestPlan` (data) → credential placement → egress → connector
      audit. Permission subjects are computed **before** placement, so a query-placed secret can
      never enter an approval prompt or an evidence record. Generic v1 has no raw proxy; S-030 owns
      any later operator-only break-glass path.
- [ ] Connector scopes and Grants are independently tested. Identity service principals remain
      Identity-owned, and an Identity-carried Connection/Grant reference cannot skip receiver lookup
      or fine-grained Grant admission.
- [ ] Exactly **one** request-composition path exists, held by a fence: a consumer that edits a plan
      has become a second one, and that is refused by design.
- [ ] **Exit**: end-to-end tests for personal-local and pinned Identity authority — admit, connect a
      real provider, grant, invoke — with every connector step audited under the closed vocabulary.

## Progress

- 2026-08-14 — a development-only vertical slice now projects the source-grounded Asterisk
  `sip.dial` member through generic search/describe/invoke, exact personal owner snapshot,
  Connection initiation, configured Grant/approval references, alias-only routing, live session
  custody and payload-free local audit. It is evidence for the eventual one-path invoke design, not
  M3 completion: general Integration/Connection/Grant persistence, credential acquisition, hosted
  authority, lifecycle and a stable real-provider exit remain open.
- 2026-08-14 — the personal-local Slack slice adds a value-free Integration policy, one-use
  operator-entry Connect Session, prepared owner-only credential custody, crash-recovered durable
  Connection metadata, and stable post-creation ids. It deliberately does not claim M3: general
  reauthorization-in-place, multiple acquisition kinds, Grant persistence, hosted identity, audit,
  and declared invoke through that Connection remain open.
- 2026-08-14 — `connectors connect slack` now acts as the first-party acquisition façade: it hides
  Connect Session references and the one-use completion endpoint, prompts without echo, follows the
  Connection to callable, and prints only the human result. The public guide names the hosted
  **Add Slack** equivalent. Slack's app-level token is app-wide, so moving its custody to Integration
  and acquiring workspace Connections through OAuth remains required before a multi-workspace
  product claim.

## Notes

- Exit criterion from the 2026-08-14 architecture amendment: *"admit local or Identity authority →
  connect a real provider → grant → invoke, all audited."*
- Depends on [S-007](S-007-m2-the-platform-skeleton-serves.md). Carries
  [S-011](S-011-deployment-declared-destination-aperture.md) (the egress policy the invoke path and
  channel runner share) and [S-014](S-014-auth-as-tool-result.md) (what invoke returns when the
  connection is missing or degraded); [S-013](S-013-connect-session-oauth-custody-in-personal-posture.md)
  decides the personal-posture custody chain this milestone's connect sessions need.
- The predecessor proved most of these invariants in anger; the ones with the sharpest measured
  history are subjects-before-placement, deny>allow>predicate, and stable connection ids across
  re-auth. Read the domain model's Runtime side before designing any of them differently.
