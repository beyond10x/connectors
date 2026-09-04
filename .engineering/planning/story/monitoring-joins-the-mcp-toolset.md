---
format: aep.planning-md/1
id: story:monitoring-joins-the-mcp-toolset
kind: story
status: implemented
title: Monitoring joins the MCP toolset
refs:
- provider: legacy
  reference: S-060
relations:
- derived_from: epic:endpoint-plane
scope:
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-060-monitoring-joins-the-mcp-toolset.md:23`. **read**

- `tool_search` for a monitoring-read-group principal lists the monitoring tools with input
  schemas that surface the configured targets (connection choices) honestly; a group-less
  principal sees none; router tests cover a happy invoke and a refusal per design 14's
  patterns.
- The rule-17 seam guard still binds the toolset; `bash scripts/gate.sh` exits 0.

## Context

The hosted deployment already reaches the central Grafana on the infra cluster with fifteen
configured Prometheus/Loki/Alertmanager targets, but the MCP toolset projects only Kubernetes.
Add the monitoring tools to the static toolset — grafana dashboards list/get, datasources
list, prometheus_query_range, loki_query_range, alertmanager_alerts — with the same
requirement-driven role filtering (an entry appears only when the caller's own seam results
support it) and the same lease hiding. All six underlying operations are catalogued read-only
and ride the read path; the toolset adds zero policy.

Source frontmatter: pillar Platform · areas [server] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-060-monitoring-joins-the-mcp-toolset.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-060-monitoring-joins-the-mcp-toolset.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-060`, recorded as the reference `legacy:S-060`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
