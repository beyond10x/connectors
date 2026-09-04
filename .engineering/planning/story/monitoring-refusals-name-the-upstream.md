---
format: aep.planning-md/1
id: story:monitoring-refusals-name-the-upstream
kind: story
status: implemented
title: Monitoring refusals name the upstream
refs:
- provider: legacy
  reference: S-065
relations:
- derived_from: epic:endpoint-plane
revision: 5
---
## Acceptance

Verbatim from `docs/stories/S-065-monitoring-refusals-name-the-upstream.md:24`. **read**

- [x] A refused monitoring dispatch logs one structured server-side line naming the operation,
      the route (direct/mediated), and the upstream HTTP status (never the body, which may carry
      data) — enough to distinguish 401 vs 403 vs 404 vs transport failure from `kubectl logs`.
- [x] The refusal's `structuredContent`/message distinguishes at least "upstream refused
      (status class)" from "upstream unreachable" from "credential custody failed", without
      leaking upstream body content.
- [ ] With that in place, the live grafana-parent and alertmanager failures are diagnosed on the
      dev deployment and their concrete cause is recorded in this story (fix here if it is a
      document/path bug; a provisioning follow-up if it is credential rights).

## Context

Two monitoring surfaces refuse `unavailable — monitoring connector runtime is unavailable` on
the dev deployment (e2e 2026-08-24, rev 15) while their siblings work: the parent Grafana
tools (`grafana_dashboards_list` on Grafana 13.2.0, whose app-platform API answers the
origin), and `alertmanager_alerts` — while `prometheus_query_range` and `loki_query_range`
ride the same credential and the same datasource proxy successfully. Diagnosis is impossible
from outside because the backend maps every transport error, non-2xx and non-JSON body to the
same context-free refusal (`integration-monitoring/src/backend.rs:110-114` and the
`operation_unavailable()` call sites), and nothing is logged server-side.

Source frontmatter: pillar Platform · areas [integrations] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-065-monitoring-refusals-name-the-upstream.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-065-monitoring-refusals-name-the-upstream.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-065`, recorded as the reference `legacy:S-065`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
