---
format: aep.planning-md/1
id: story:monitoring-tool-schemas-tell-the-truth
kind: story
status: implemented
title: Monitoring tool schemas tell the truth
refs:
- provider: legacy
  reference: S-064
relations:
- derived_from: epic:endpoint-plane
scope:
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-064-monitoring-tool-schemas-tell-the-truth.md:27`. **read**

- [x] Each monitoring tool's schema states exactly the fields its operation accepts, required
      matches what refusal enforces, cursors are optional, and numeric-or-string timestamps are
      either accepted or documented.
- [x] The e2e sequence `grafana_dashboards_list {target}` and `prometheus_query_range` with epoch
      ints passes or refuses with a message naming the expected encoding.

## Context

Live e2e (2026-08-24) against the deployed MCP toolset found the monitoring tool input schemas
misleading in three ways:

- `grafana_dashboards_list` requires `["namespace", "limit", "continue"]` — a Kubernetes
  workload-list shape, not a Grafana dashboards shape; `continue` (a resume cursor) being
  REQUIRED forces callers to invent an empty string.
- `target` is enum-projected but not in `required`, while the tools refuse without it
  (`invalid_input`), so the schema admits calls that can never succeed.
- `prometheus_query_range` demands string `start`/`end`/`step`; integer epoch seconds — the
  natural client encoding — refuse as `invalid_input` with no hint. Either accept both or say
  "string" is unix-seconds-or-RFC3339 in the description.

Source frontmatter: pillar Platform · areas [integrations, server] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-064-monitoring-tool-schemas-tell-the-truth.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-064-monitoring-tool-schemas-tell-the-truth.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-064`, recorded as the reference `legacy:S-064`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
