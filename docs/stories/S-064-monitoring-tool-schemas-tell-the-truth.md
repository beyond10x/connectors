---
id: S-064
title: "Monitoring tool schemas tell the truth"
pillar: Platform
status: ready
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [integrations, server]
---

# Monitoring tool schemas tell the truth

## Goal

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

## Acceptance

- Each monitoring tool's schema states exactly the fields its operation accepts, required
  matches what refusal enforces, cursors are optional, and numeric-or-string timestamps are
  either accepted or documented.
- The e2e sequence `grafana_dashboards_list {target}` and `prometheus_query_range` with epoch
  ints passes or refuses with a message naming the expected encoding.

## Notes

- Separately observed, not this story: the "Global infrastructure Grafana" target answers
  `unavailable — monitoring connector runtime is unavailable` while all five Prometheus targets
  work (dev and infra both verified live, distinct node counts 5 vs 3). That is provisioning
  (the Grafana connection/credential on the hosted config), tracked with the deployment, not a
  toolset defect.
