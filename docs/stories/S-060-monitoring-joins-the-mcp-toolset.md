---
id: S-060
title: "Monitoring joins the MCP toolset"
pillar: Platform
status: ready
priority: 1
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
