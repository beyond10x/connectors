---
id: S-066
title: "The transport refusal names its class"
pillar: Platform
status: ready
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

- The transport arm of the refusal log carries an error class derived from the transport
  error (timeout / connect / tls / body-read / other) — never the error's Display string,
  which can embed the URL.
- The transport class distinguishes an oversized upstream body from unreachability; an
  executor cap breach refuses as "response too large", never "unreachable".
- `grafana_dashboards_list` dispatches with a bounded page `limit` and follows `continue`
  server-side up to a hard budget, so the 4.6 MB namespace lists instead of refusing.
- The alertmanager document resolves the v2 API sub-path; `alertmanager_alerts` returns live
  alerts on dev through the mediated route.
- The alertmanager 403 is IN scope — diagnosis below proved it a document defect, not
  provisioning.

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
