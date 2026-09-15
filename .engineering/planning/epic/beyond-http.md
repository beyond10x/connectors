---
format: aep.planning-md/1
id: epic:beyond-http
kind: epic
status: archived
title: Beyond HTTP
revision: 6
---
## Provenance

Read from the `epic: beyond-http` key in 6 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-023` `story:beyond-http-facts-enter-the-document` — done — Beyond-HTTP facts enter the connector document orthogonally
- `S-024` `story:one-zero-io-plan-seam-selects-a-closed-driver` — in-progress — One zero-IO plan seam selects a closed built-in driver
- `S-025` `story:satellite-federation-has-a-threat-modeled-contract` — backlog — Satellite federation has a threat-modeled contract
- `S-026` `story:one-real-non-http-driver-proves-the-model` — in-progress — One real non-HTTP driver proves the five-axis model
- `S-027` `story:direct-byte-session-establishment-is-operation-scoped` — in-progress — Direct-byte session establishment is operation-scoped
- `S-028` `story:external-driver-artifacts-stay-deferred` — blocked — External driver artifacts stay deferred behind attestation

## Status

`active`, derived from its stories (backlog: 1, blocked: 1, done: 1, in-progress: 3). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

Archived 2026-09-15 on the acceptance of `docs/design/21-clean-room-rewrite.md` (org-state review
decision sheet 2026-09-15, items 1 and 15; atlas ADR 0051, 2026-09-15).

**Excluded by design 21 §4.** Every story here lands on a struck row or on capability §2's keep list does
not carry:

- S-024 (a zero-IO plan seam selecting a closed built-in driver) and S-026 (one real non-HTTP driver
  proving the five-axis model) are the built-in driver family. §4 strikes both of the non-voice ones —
  "Browser + SQL drivers — `driver-cdp`, `driver-sql` — 4,292 — Drop from v1 spec" — and relocates the
  third, `driver-sip`, with the voice stack (§4 first row).
- S-027 (direct-byte session establishment is operation-scoped) is the Git byte plane shipped in 0.6.1
  (`CHANGELOG.md`, "internal TLS Smart Git byte plane … short-lived, read-only fetch sessions"). §4:
  "Git fetch sessions — designs 19, 20; git ESS domain — Drop from v1 spec."
- S-025 (a threat-modeled satellite-federation contract) is on no keep row: §2's K1–K11 name no
  federation capability, and `README.md` already states that "Full SaaS and satellite federation remain
  outside the current support claim."
- S-028 (external driver artifacts stay deferred behind attestation) defers the same driver family; it is
  `blocked` and has no subject left once the family is struck.
- S-023 (beyond-HTTP facts enter the document orthogonally) is `done` and its result lives in the catalog
  document shape, which §2 keeps as K1 and the extraction phase re-derives from fixtures.

§7 D3 confirms the strip list as proposed. Note for phase 1: the successor has since shipped a PostgreSQL
adapter (`schema.list`, `query.read`; `connectors_v2/README.md`) — §4's "drop" binds this repository's v1
spec, not what the successor may carry.

Its 6 `docs/stories/S-*` records keep their own statuses (1 `done`, 3 `in-progress`, 1 `backlog`, 1 `blocked`). This
repository's `AGENTS.md` states no convention for stories under an archived epic, so none was moved.
