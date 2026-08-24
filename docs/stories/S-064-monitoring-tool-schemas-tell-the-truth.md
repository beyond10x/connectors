---
id: S-064
title: "Monitoring tool schemas tell the truth"
pillar: Platform
status: in-progress
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

- [x] Each monitoring tool's schema states exactly the fields its operation accepts, required
      matches what refusal enforces, cursors are optional, and numeric-or-string timestamps are
      either accepted or documented.
- [x] The e2e sequence `grafana_dashboards_list {target}` and `prometheus_query_range` with epoch
      ints passes or refuses with a message naming the expected encoding.

## Notes

- Separately observed, not this story: the "Global infrastructure Grafana" target answers
  `unavailable — monitoring connector runtime is unavailable` while all five Prometheus targets
  work (dev and infra both verified live, distinct node counts 5 vs 3). That is provisioning
  (the Grafana connection/credential on the hosted config), tracked with the deployment, not a
  toolset defect.
- **Timestamp decision (S-064):** Prometheus `start`/`end` accept unsigned integer epoch seconds
  beside strings, and `step` accepts integer seconds beside a PromQL duration string — the
  vendor API documents `rfc3339 | unix_timestamp` and `duration | float`, and the resolver
  renders a JSON number into the query string unchanged. Loki `start`/`end` stay strings,
  because Loki parses a bare integer as unix *nanoseconds* and integer epoch seconds would
  silently query 1970; the refusal now names that encoding instead of the anonymous
  `invalid_input`.
- The `namespace`/`limit`/`continue` naming is not a leaked Kubernetes shape: the operation
  genuinely calls Grafana's app-platform dashboards API
  (`{base}/apis/dashboard.grafana.app/v1/namespaces/{namespace}/dashboards`), whose surface is
  Kubernetes-styled by Grafana's own design. The lie was `required`: the canonical document
  marks `limit` and `continue` omittable, and the validator's exact-key-set check required them
  anyway.

## Progress

- 2026-08-24 — implemented on `impl/S-064`. `monitoring_model::validate_input` now enforces the
  document's own contract (undeclared keys refused, required parameters demanded, omittable ones
  omittable) instead of requiring every declared parameter; the toolset schemas mirror the
  document (`grafana_dashboards_list` requires only `namespace`); `tool_describe` adds `target`
  to `required` exactly while several connections are configured, beside the existing enum
  narrowing. Proven by `monitoring_tool_schemas_state_the_documents_contract` (server),
  `dashboards_list_dispatches_the_documents_required_only_input_over_http` and
  `prometheus_range_accepts_integer_epoch_seconds_on_the_mediated_route`
  (integration-monitoring), and the validator tests in `monitoring-model` — all red at the
  merge base with the live symptom (`invalid_input — monitoring operation input is invalid`).
- The dispatch test also answers the wiring question behind the live `unavailable`: the wired
  operation document is `http_v1` and plans/dispatches through to the HTTP executor with the
  required-only input, so a live `unavailable` can only arise from the HTTP exchange itself
  (credential custody, audit store, or the origin's answer — e.g. a Grafana without the
  app-platform dashboards API), not from toolset or document wiring. Live re-verification
  against the deployed endpoint follows the next deploy.
