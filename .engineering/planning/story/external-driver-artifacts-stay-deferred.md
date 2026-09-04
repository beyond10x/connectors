---
format: aep.planning-md/1
id: story:external-driver-artifacts-stay-deferred
kind: story
status: active
title: External driver artifacts stay deferred behind attestation
refs:
- provider: legacy
  reference: S-028
relations:
- derived_from: epic:beyond-http
scope:
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
- confidence: cited
  path: docs
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-028-external-driver-artifacts-stay-deferred.md:20`. **read**

- [ ] At least one built-in non-HTTP driver documents concrete pressure that cannot be resolved by
      the closed registry.
- [ ] A separate RFC/ADR defines artifact identity, signing, provenance, review, capability manifest,
      credential delivery, isolation, resource bounds, update/rollback, and revocation.
- [ ] Substrate executes only generic bounded work and never interprets vendor semantics.
- [ ] No external artifact enters a caller-selected path or becomes model-selectable.
- [ ] Until all gates close, schema and runtime reject external implementation forms.

## Context

Prevent “beyond HTTP” from silently becoming an arbitrary plugin/process host before built-in
drivers prove a packaging boundary and a separate supply-chain decision is accepted.

Source frontmatter: pillar Platform · areas [domain, service, server, docs] · design `docs/design/03-beyond-http.md`. **read**

Source `note:` field, quoted: “delivery item 6 is a gate, not implementation scope; unblock only after built-in pressure and a separate security ADR”

## Status

`blocked` in the source. Quoted from `docs/stories/S-028-external-driver-artifacts-stay-deferred.md:5`: `status: blocked`. **read**

## Provenance

Migrated from `docs/stories/S-028-external-driver-artifacts-stay-deferred.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 1 revision(s)
- Legacy id `S-028`, recorded as the reference `legacy:S-028`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
