---
id: S-066
title: "The transport refusal names its class"
pillar: Platform
status: done
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [integrations]
---

# The transport refusal names its class

## Goal

S-065's live diagnosis (rev 16) pinned `grafana_dashboards_list` to an upstream-transport
failure that is specific to that operation's resolved request — `grafana-datasources-list`
succeeds on the same direct route, credential and origin every reconcile tick. The final
discriminator (timeout vs connect vs TLS vs body-read) is discarded at the transport mapping
(`integration-monitoring`, the `.map_err(|_| UpstreamFailure::Transport)` sites), so the last
step of the diagnosis is blind.

## Acceptance

- [x] The transport arm of the refusal log carries an error class derived from the transport
  error (timeout / connect / tls / body-read / other) — never the error's Display string,
  which can embed the URL.
- [x] The transport class distinguishes an oversized upstream body from unreachability; an
  executor cap breach refuses as "response too large", never "unreachable".
- [x] `grafana_dashboards_list` dispatches with a bounded page `limit` and follows `continue`
  server-side up to a hard budget, so the 4.6 MB namespace lists instead of refusing.
- [x] The alertmanager document resolves the v2 API sub-path; `alertmanager_alerts` returns live
  alerts on dev through the mediated route. (The document half is test-proven through the real
  resolve path; the live-dev half needs the next deploy to pick the rebuilt catalog pack up.)
- [x] The alertmanager 403 is IN scope — diagnosis below proved it a document defect, not
  provisioning.

## Notes

- Page-size byte math (S-066): the live namespace listing is 4,615,274 bytes over at most 1000
  dashboards — Grafana still served everything at `limit=1000`, so the mean raw object is
  ≥ 4.6 KiB. `DASHBOARD_PAGE_LIMIT = 5` keeps one upstream page under the 256 KiB
  `MAX_RESULT_BYTES` egress bound even at ten times that mean (5 × 46 KiB ≈ 230 KiB), and five
  is the page size the live diagnosis verified to answer with a `continue` token.
  `MAX_DASHBOARD_FETCHES = 20` bounds one dispatch at 20 pages (raw aggregate ≤ 20 × 256 KiB =
  5 MiB in memory, at most 100 dashboards against the default caller limit of 100); the
  projected aggregate is re-checked against `MAX_RESULT_BYTES` before it answers. When the
  budget trips first the answer is the honest short page: `next_cursor` set, `complete` false.
- Catalog process choice (S-066): the alertmanager document is generated, so the fix landed in
  the source layer — the vendor-derived spec projection composes the upstream Swagger basePath
  `/api/v2` into the operation path (`/api/v2/alerts`) and `providers/alertmanager.toml` drops
  the sub-path from `base_url` — then `catalog build` regenerated
  `catalog/alertmanager.catalog.json`, `connectors.lock`, the pack and the site projection;
  `diff` settled and `check` verifies. A base-level sub-path is silently dropped on the mediated
  route because planning derives the proxy path from `request.url` minus `{base}` alone, which
  is exactly the live 403.

## Progress

- 2026-08-24, direct probes against https://grafana.infra.babelforce.com with the tenant's
  Grafana service-account token (read from Vault custody; token never persisted):
  - `grafana_dashboards_list`: the app-platform API answers **200 in 0.45 s**; the full
    namespace listing is **4,615,274 bytes**, and Grafana serves it whole even at
    `limit=1000`, while `limit=5` pages correctly with a `continue` token. The executor caps
    responses at `MAX_RESULT_BYTES` (256 KiB, egress.rs `maximum_response_bytes`), and the
    cap breach is classified as transport — hence "unreachable". Fix: the dispatch sends a
    small page `limit` and follows `continue`, and the refusal class for an oversized body
    says so instead of "unreachable".
  - `alertmanager_alerts`: the document requests `{base}/alerts`; through the Grafana
    datasource proxy that sub-path answers **403 "plugin proxy route access denied"**, while
    `{base}/api/v2/alerts` answers **200 with 231,711 bytes** of live alerts using the same
    token and uid. Fix: the alertmanager document (or the mediation adapter) resolves the
    Alertmanager v2 API path.
- 2026-08-24, implemented on `impl/S-066`: the egress transport classifies its failures
  (`EgressTransportError::Transport(timeout|connect|tls|body-read|other)`, cap breach stays
  `ResponseTooLarge`), the monitoring refusal log carries `transport_class` and the new
  `upstream-response-too-large` cause (client answer: `result_too_large`, "monitoring upstream
  response exceeds the result bound"), the dashboards dispatch walks bounded upstream pages
  (constants and byte math in Notes), and the alertmanager document resolves
  `{base}/api/v2/alerts` via the spec projection + `catalog build`. Three failing-first tests in
  `backend_tests.rs` cover all three fixes. Remaining for whoever closes this: verify
  `alertmanager_alerts` answers live alerts on dev after the next deploy carries the rebuilt
  catalog pack.
