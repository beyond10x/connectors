---
format: aep.planning-md/1
id: story:m3-connect-a-provider-and-invoke-it
kind: story
status: draft
title: M3 — connect a real provider, grant it, invoke it
refs:
- provider: legacy
  reference: S-008
relations:
- derived_from: epic:build-order
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-008-m3-connect-a-provider-and-invoke-it.md:23`. **read**

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

## Context

Deliver the middle layer and the one invocation path: a tenant configures an integration, a human
authorizes a vendor connection through a Connect Session, a connector operator grants authority over
declared facts, and a client presents personal-local or short-lived exact-audience Identity
authority to invoke a declared operation. Neither the Identity credential nor the vendor credential
changes owners. This is the milestone at which the client contract is real.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server] · design `docs/design/02-architecture.md`. **read**

Source `note:` field, quoted: “architecture §9 milestone M3. Exit: admit personal-local or pinned Identity authority → connect a real provider → grant → invoke, all audited. Connectors never owns the hosted login/session. This is where the product exists; everything before it is scaffolding and everything after it is reach”

## Status

`backlog` in the source. Quoted from `docs/stories/S-008-m3-connect-a-provider-and-invoke-it.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-008-m3-connect-a-provider-and-invoke-it.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-16 · 6 revision(s)
- Legacy id `S-008`, recorded as the reference `legacy:S-008`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
