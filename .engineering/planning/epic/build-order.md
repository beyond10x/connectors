---
format: aep.planning-md/1
id: epic:build-order
kind: epic
status: active
title: Build order
revision: 4
---
## Provenance

Read from the `epic: build-order` key in 4 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-007` `story:m2-the-platform-skeleton-serves` — backlog — M2 — the platform skeleton serves in both postures
- `S-008` `story:m3-connect-a-provider-and-invoke-it` — backlog — M3 — connect a real provider, grant it, invoke it
- `S-009` `story:m4-events-reach-a-client-by-push-and-by-pull` — backlog — M4 — a provider event reaches a client by push and by pull, with provenance
- `S-010` `story:m5-flux-re-points-and-the-gitlab-plugin-retires` — blocked — M5 — flux re-points at the platform and the gitlab plugin is deleted

## Status

`active`, derived from its stories (backlog: 3, blocked: 1). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

**Left `active` on 2026-09-15 — needs the operator.** `docs/design/21-clean-room-rewrite.md` was accepted
that day (decision sheet items 1 and 15) and six sibling epics archived under it; this one does not
classify cleanly and was not moved.

Split verdict: M3 (S-008, connect a real provider, grant it, invoke it) is covered by the successor's
local CLI for GitLab, Kubernetes and PostgreSQL (`connectors_v2/README.md`), but M2 (S-007, the skeleton
serving in **both** postures) and M4 (S-009, events by push and by pull) are not — the successor "does not
advertise … durable events" and claims no hosted posture, while design 21 §2 keeps both as K10 and K6.
M5 (S-010) names `flux`, a former-org repository name listed by atlas ADR 0001:46. Design 21 §6 also replaces this milestone
ladder wholesale with its own phases 0–5, so the ladder may simply have no addressee left.

What settles it: whether the K6/K10 keep rows are extracted for the successor (design 21 §6 phase 1) or
dropped, and whether M5 survives the brand retirement at all.
