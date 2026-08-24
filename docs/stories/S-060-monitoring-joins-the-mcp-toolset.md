---
id: S-060
title: "Monitoring joins the MCP toolset"
pillar: Platform
status: done
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [server]
---

# Monitoring joins the MCP toolset

## Goal

The hosted deployment already reaches the central Grafana on the infra cluster with fifteen
configured Prometheus/Loki/Alertmanager targets, but the MCP toolset projects only Kubernetes.
Add the monitoring tools to the static toolset — grafana dashboards list/get, datasources
list, prometheus_query_range, loki_query_range, alertmanager_alerts — with the same
requirement-driven role filtering (an entry appears only when the caller's own seam results
support it) and the same lease hiding. All six underlying operations are catalogued read-only
and ride the read path; the toolset adds zero policy.

## Acceptance

- `tool_search` for a monitoring-read-group principal lists the monitoring tools with input
  schemas that surface the configured targets (connection choices) honestly; a group-less
  principal sees none; router tests cover a happy invoke and a refusal per design 14's
  patterns.
- The rule-17 seam guard still binds the toolset; `bash scripts/gate.sh` exits 0.

## Progress

- 2026-08-24 — filed from design 15 (Timo: the central Grafana "should be made available").
- 2026-08-24 — implemented on `impl/S-060`. Six `TargetedOperationInvoke` entries in
  `crates/server/src/hosted/mcp/toolset.rs`; the connection choice is a `target` argument
  resolved by label against the caller's own freshly described connections (lone connection:
  optional; several: required, with the admitted labels enumerated into the schema by
  `tool_describe` and into the refusal at invoke). Requirement discovery runs one seam search
  per `REQUIREMENT_QUERIES` term and merges refs; the stale-authority retry re-resolves the
  target against the fresh description. Router tests: `hosted/tests/mcp_monitoring.rs`
  (fake monitoring backend, central Grafana + two targets per fleet provider). Argument
  shapes deliberately mirror `monitoring-model::validate_input` exactly — no defaults are
  invented, so a schema-valid call is input-valid.
- 2026-08-24 — merged to main after independent review (PASS, 0 blocking, 2 minor: the
  zero-connection refusal's empty label list reads oddly, and a comment overclaims schema/
  validate_input equivalence on code-point vs byte length). Toolset grows 5 -> 11 tools.
