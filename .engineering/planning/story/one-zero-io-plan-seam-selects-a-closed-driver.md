---
format: aep.planning-md/1
id: story:one-zero-io-plan-seam-selects-a-closed-driver
kind: story
status: active
title: One zero-IO plan seam selects a closed built-in driver
refs:
- provider: legacy
  reference: S-024
relations:
- derived_from: epic:beyond-http
scope:
- confidence: cited
  path: crates/connector-resolve
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-024-one-zero-io-plan-seam-selects-a-closed-driver.md:20`. **read**

- [x] A typed zero-IO plan names one closed driver and its reviewed operation/channel facts.
- [x] Grant admission and permission subjects are fixed before credential placement.
- [x] Exactly one dispatch composition point applies egress, credential, redaction, and audit
      policy before handing a driver its bounded plan.
- [x] Unknown drivers and unmet capabilities refuse by name; there is no process/plugin fallback.
- [ ] Fence tests fail if a second policy-composition or vendor-dial path appears.

## Context

Extend request planning beyond HTTP without letting planning perform IO or letting each driver
recompose grants, credentials, egress, redaction, and audit independently.

Source frontmatter: pillar Platform · areas [domain, service, server, connector-resolve] · design `docs/design/03-beyond-http.md`. **read**

Source `note:` field, quoted: “ADR 0010 delivery item 1; planning remains data and dispatch has one composition point”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-024-one-zero-io-plan-seam-selects-a-closed-driver.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-024-one-zero-io-plan-seam-selects-a-closed-driver.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-15 · 3 revision(s)
- Legacy id `S-024`, recorded as the reference `legacy:S-024`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
