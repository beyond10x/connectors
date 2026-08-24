---
id: S-065
title: "Monitoring refusals name the upstream"
pillar: Platform
status: ready
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [integrations]
---

# Monitoring refusals name the upstream

## Goal

Two monitoring surfaces refuse `unavailable — monitoring connector runtime is unavailable` on
the dev deployment (e2e 2026-08-24, rev 15) while their siblings work: the parent Grafana
tools (`grafana_dashboards_list` on Grafana 13.2.0, whose app-platform API answers the
origin), and `alertmanager_alerts` — while `prometheus_query_range` and `loki_query_range`
ride the same credential and the same datasource proxy successfully. Diagnosis is impossible
from outside because the backend maps every transport error, non-2xx and non-JSON body to the
same context-free refusal (`integration-monitoring/src/backend.rs:110-114` and the
`operation_unavailable()` call sites), and nothing is logged server-side.

## Acceptance

- A refused monitoring dispatch logs one structured server-side line naming the operation,
  the route (direct/mediated), and the upstream HTTP status (never the body, which may carry
  data) — enough to distinguish 401 vs 403 vs 404 vs transport failure from `kubectl logs`.
- The refusal's `structuredContent`/message distinguishes at least "upstream refused
  (status class)" from "upstream unreachable" from "credential custody failed", without
  leaking upstream body content.
- With that in place, the live grafana-parent and alertmanager failures are diagnosed on the
  dev deployment and their concrete cause is recorded in this story (fix here if it is a
  document/path bug; a provisioning follow-up if it is credential rights).

## Notes

- Verified live while filing: Grafana `/api/health` 200 (version 13.2.0),
  `/apis/dashboard.grafana.app/v1` 401 unauthenticated; five Prometheus targets and Loki
  answer with real data through the proxy; Alertmanager and the Grafana parent both refuse
  `unavailable` after input validation passes.
