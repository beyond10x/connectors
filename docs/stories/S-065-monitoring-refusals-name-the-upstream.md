---
id: S-065
title: "Monitoring refusals name the upstream"
pillar: Platform
status: done
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

- [x] A refused monitoring dispatch logs one structured server-side line naming the operation,
      the route (direct/mediated), and the upstream HTTP status (never the body, which may carry
      data) — enough to distinguish 401 vs 403 vs 404 vs transport failure from `kubectl logs`.
- [x] The refusal's `structuredContent`/message distinguishes at least "upstream refused
      (status class)" from "upstream unreachable" from "credential custody failed", without
      leaking upstream body content.
- [ ] With that in place, the live grafana-parent and alertmanager failures are diagnosed on the
      dev deployment and their concrete cause is recorded in this story (fix here if it is a
      document/path bug; a provisioning follow-up if it is credential rights).

## Notes

- Verified live while filing: Grafana `/api/health` 200 (version 13.2.0),
  `/apis/dashboard.grafana.app/v1` 401 unauthenticated; five Prometheus targets and Loki
  answer with real data through the proxy; Alertmanager and the Grafana parent both refuse
  `unavailable` after input validation passes.

## Progress

- 2026-08-24 — implemented on `impl/S-065`. The `HttpExecutor` seam
  (`integration-monitoring/src/backend.rs`) now classifies a failed exchange as
  `UpstreamFailure::{Transport, Status(u16), Body}` instead of collapsing it at the
  `PortExecutor`; `refuse_dispatch()` emits one JSON stderr line per refused dispatch
  (`event: monitoring_dispatch_refused`, `operation_ref`, `route: direct|mediated`, `cause:
  upstream-status|upstream-transport|upstream-body|credential-custody`, and `upstream_status`
  with the exact numeric status when one arrived) and answers the still-`unavailable`,
  still-retriable refusal with a class-naming message ("upstream_status 4xx" / "upstream is
  unreachable" / "non-JSON response" / "credential custody failed"). `load_credential` refuses
  custody failures (store reachable-but-failing) distinctly from an absent credential
  (unchanged `not_granted`). The upstream body never travels into either the log or the
  message. Tests: `refused_dispatches_distinguish_upstream_status_class_from_transport`
  (401/403/404/500/transport through the real `PortExecutor` over a scripted egress),
  `refusal_log_record_names_operation_route_and_exact_upstream_status`,
  `credential_custody_failure_is_distinguished_from_upstream_failures`. The stderr emission
  itself is a plain `eprintln!` matching the crate idiom (integration-slack precedent) and is
  not capture-tested — the record content is tested as a pure value.
- **Live diagnosis pending deploy** (acceptance item 3): the dev-deployment grafana-parent and
  alertmanager causes are to be read from the new `monitoring_dispatch_refused` lines after
  the next deploy and recorded here.

## Progress

- 2026-08-24, live diagnosis on release rev 16 with the new refusals:
  - `alertmanager_alerts` (mediated): Grafana's datasource proxy answers **403** — the
    service-account token lacks alertmanager-datasource query rights in Grafana RBAC.
    Provisioning follow-up on the infra Grafana (grant the SA alertmanager datasource
    access); not a code defect.
  - `grafana_dashboards_list` (direct): **upstream-transport**, while
    `grafana-datasources-list` succeeds on the same route, credential and origin every
    300 s reconcile tick (zero reconcile refusals in the log). The failure is specific to
    the dashboards operation's resolved request; the transport error class
    (timeout/connect/TLS) is the one detail the refusal log still discards — S-066 adds it
    and closes the operation.

## Superseded by

`story:monitoring-refusals-name-the-upstream` in the AEP planning store, at
`.engineering/planning/story/monitoring-refusals-name-the-upstream.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
