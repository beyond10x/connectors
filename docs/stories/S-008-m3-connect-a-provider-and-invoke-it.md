---
id: S-008
title: "M3 — connect a real provider, grant it, invoke it"
pillar: Platform
status: backlog
priority:
design: docs/design/02-architecture.md
epic: build-order
areas: [domain, protocol, service, server]
note: "architecture §9 milestone M3. Exit: end-to-end sign in → connect a real provider → grant → invoke, all audited. This is where the product exists; everything before it is scaffolding and everything after it is reach"
---

# M3 — connect a real provider, grant it, invoke it

## Goal

Deliver the middle layer and the one invocation path: an organization configures an integration, a
human authorizes a connection through a connect session, an operator grants authority over declared
facts, and a client holding **one token** invokes a declared operation — with the vendor credential
never crossing the boundary. This is the milestone at which the client contract is real.

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
      which a grant and a subscription referencing a connection survive a re-auth. Multiple labelled
      connections per integration are first-class, and both scopes (organization-shared,
      user-owned) exist.
- [ ] **Grants**: organization-scoped, per-provider, admitting by selector over declared facts (risk
      ceiling, effects subset, idempotency) plus explicit exceptions where **deny beats allow beats
      predicate**; CAS-revisioned mutation with previewable proposals and receipts. No store bound is
      an outage (503); an empty store is a refusal (403); a refusal never names the axis that refused.
      Grants bind to connections, never to credentials.
- [ ] **Declared invoke rides one path**: principal → effective catalogue (sealed generation) → grant
      admission through unconstructible proof types → connection resolution → document → `RequestPlan`
      (data) → credential placement → egress → audit. Permission subjects are computed **before**
      placement, so a query-placed secret can never enter an approval prompt or an evidence record.
      Generic v1 has no raw proxy; S-030 owns any later operator-only break-glass path.
- [ ] Exactly **one** request-composition path exists, held by a fence: a consumer that edits a plan
      has become a second one, and that is refused by design.
- [ ] **Exit**: one end-to-end test — sign in, connect a real provider, grant, invoke — with every
      step audited under the closed vocabulary.

## Progress
- (not started)

## Notes

- Exit criterion, verbatim from architecture §9: *"end-to-end: sign in → connect a real provider →
  grant → invoke, all audited."*
- Depends on [S-007](S-007-m2-the-platform-skeleton-serves.md). Carries
  [S-011](S-011-deployment-declared-destination-aperture.md) (the egress policy the invoke path and
  channel runner share) and [S-014](S-014-auth-as-tool-result.md) (what invoke returns when the
  connection is missing or degraded); [S-013](S-013-connect-session-oauth-custody-in-personal-posture.md)
  decides the personal-posture custody chain this milestone's connect sessions need.
- The predecessor proved most of these invariants in anger; the ones with the sharpest measured
  history are subjects-before-placement, deny>allow>predicate, and stable connection ids across
  re-auth. Read the domain model's Runtime side before designing any of them differently.
