---
format: aep.planning-md/1
id: epic:post-m1
kind: epic
status: active
title: Post-M1
revision: 5
---
## Provenance

Read from the `epic: post-m1` key in 5 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-018` `story:the-explorer-works-against-the-new-site-json` — done — The web explorer works against the site JSON M1 actually emits
- `S-019` `story:retire-the-flux-connectors-identity` — done — Retire the flux-connectors identity from the artifacts
- `S-020` `story:a-ci-gate-exists` — in-progress — A CI gate exists, and it runs what the monorepo claims it runs
- `S-021` `story:coverage-regains-its-second-direction` — backlog — Coverage regains its second direction: every gap between declared and published has a reason
- `S-022` `story:orphaned-inputs-are-removed-or-re-owned` — done — Orphaned inputs are removed or re-owned

## Status

`active`, derived from its stories (backlog: 1, done: 3, in-progress: 1). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

**Left `active` on 2026-09-15 — needs the operator.** `docs/design/21-clean-room-rewrite.md` was accepted
that day (decision sheet items 1 and 15) and six sibling epics archived under it; this one does not
classify cleanly and was not moved.

It is maintenance of *this* repository rather than a capability: S-020 (a CI gate that runs what the
"monorepo" claims) and S-021 (coverage regains its second direction) are the two open stories. Design 21
§4 keeps three checks "with teeth" and says of the rest "The rest is not re-derived", and §6 phase 5
archives this repository as predecessor — but a predecessor still needs a gate until the four consumers
of §3 are repinned, and nothing in design 21 says when that gate stops mattering.

What settles it: whether this repository keeps a running CI gate until §6 phase 5 cutover, or freezes
read-only now.
