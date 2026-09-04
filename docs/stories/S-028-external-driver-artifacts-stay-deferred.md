---
id: S-028
title: "External driver artifacts stay deferred behind attestation"
pillar: Platform
status: blocked
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [domain, service, server, docs]
note: "delivery item 6 is a gate, not implementation scope; unblock only after built-in pressure and a separate security ADR"
---

# External driver artifacts stay deferred behind attestation

## Goal

Prevent “beyond HTTP” from silently becoming an arbitrary plugin/process host before built-in
drivers prove a packaging boundary and a separate supply-chain decision is accepted.

## Acceptance

- [ ] At least one built-in non-HTTP driver documents concrete pressure that cannot be resolved by
      the closed registry.
- [ ] A separate RFC/ADR defines artifact identity, signing, provenance, review, capability manifest,
      credential delivery, isolation, resource bounds, update/rollback, and revocation.
- [ ] Substrate executes only generic bounded work and never interprets vendor semantics.
- [ ] No external artifact enters a caller-selected path or becomes model-selectable.
- [ ] Until all gates close, schema and runtime reject external implementation forms.

## Progress

- (blocked by design; intentionally deferred)

## Superseded by

`story:external-driver-artifacts-stay-deferred` in the AEP planning store, at
`.engineering/planning/story/external-driver-artifacts-stay-deferred.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
