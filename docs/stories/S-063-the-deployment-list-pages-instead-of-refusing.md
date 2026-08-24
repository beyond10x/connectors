---
id: S-063
title: "The deployment list pages instead of refusing"
pillar: Platform
status: done
design: ../design/14-mcp-transport-for-the-hosted-connectors-server.md
epic: mcp-entry
areas: [integrations, server]
---

# The deployment list pages instead of refusing

## Goal

`tool_invoke k8s_deployment_list {namespace: "latest"}` refuses with `result_too_large`
(e2e 2026-08-24): the namespace holds too many deployments for one response envelope, and the
tool exposes no way to page or bound the result. A bounded refusal is correct fail-closed
behaviour, but a list tool that cannot list a busy namespace is not usable.

**Amended 2026-08-24 (Timo):** minimal records over a paging surface. List-shaped Kubernetes
datasource results must not carry raw or verbose object data — a list record is a name plus the
smallest rollout summary, and depth belongs to the get/describe path. If slim records fit, the
MCP tool carries no `limit`/`cursor` arguments at all; upstream paging stays an internal,
bounded implementation detail, and a hard total cap with an explicit truncation marker replaces
an unbounded walk.

## Acceptance

- [x] List records carry the minimal projection only — `name`, `desired_replicas`,
      `ready_replicas`, `rollout_state` — and nothing K8s-internal; object identity
      (`namespace`, `uid`, `resource_version`, generations, replica breakdowns) stays with the
      detail record.
- [x] `k8s_deployment_list {namespace}` — no paging args — succeeds against a fake namespace of
      100+ deployments whose raw objects are large enough to have tripped the live refusal;
      `result_too_large` does not occur.
- [x] The aggregation is hard-bounded (500 records / 40 seam pages) and a cut listing says so
      with `truncated: true`.
- [x] The tool description names the contract: minimal summaries, whole namespace in one call,
      the truncation bound, and `k8s_deployment_status` for one deployment's depth.

## Notes

- **Where the live refusal actually came from:** not the outbound result envelope. The old
  12-field compact record serialized to ~350 B; a full protocol page of 25 was ~9 KiB against
  `MAX_RESULT_BYTES` = 256 KiB, so slimming alone could never have fixed it. The tripping bound
  was the *upstream* apiserver response (`bounded_body`, `MAX_KUBERNETES_RESPONSE_BYTES` =
  256 KiB): raw Deployment objects arrive whole — commonly 10–60 KiB of managedFields,
  annotations and pod template — so one request for 25 of them exceeds the bound on a busy
  namespace regardless of how little the projection keeps.
- **Record-shape decision:** compact = `{name, desired_replicas, ready_replicas,
  rollout_state}` (~100–140 B serialized). `uid`/`resource_version` move wholly into the detail
  record, which is where the restart operation's exact-object authority already reads them;
  the detail record's serialized shape is unchanged. Projection declaration version bumped
  1 → 2; description leases re-derive and heal through the existing stale-authority retry.
- **Upstream walk:** `list_workloads` in both placements now fetches at most
  `UPSTREAM_PAGE_LIMIT` = 5 raw objects per apiserver request (headroom for ~51 KiB/object
  against the 256 KiB response bound) and spends at most `MAX_UPSTREAM_LIST_FETCHES` = 8
  requests per protocol page, returning a short page with a cursor when the budget runs out.
  A single raw object beyond the response bound still refuses honestly.
- **Tool aggregate bound:** 500 records × ~130 B ≈ 65 KiB structured, ~130 KiB with the
  doubled MCP payload (text block + `structuredContent`) — inside design 14's accepted
  ~512 KiB worst case. Cap constants live beside `list_deployments` in the toolset.
- Other list-shaped surfaces checked: `k8s_namespace_list` rides configured bindings (no raw
  object fetch, ≤ 25 namespaces — config-scale, fine). Deliberately *not* fixed here, same
  latent exposure, filed as findings: the detail read's pods/events fetches (`limit` 51 raw
  objects in one request) and the `kubernetes.databases` list can trip the same upstream bound
  on bloated objects.

## Progress

- 2026-08-24 — implemented on `impl/S-063`. `WorkloadCompact` slims to the four minimal fields
  with `WorkloadMeta` carrying identity into the detail record
  (`integration-kubernetes/src/workloads.rs`); both placements walk the apiserver in bounded
  upstream pages (`hosted.rs`, `local_workloads.rs`); the MCP tool drops `limit`/`cursor`,
  aggregates the seam's pages server-side and returns `{deployments, truncated}`
  (`server/src/hosted/mcp/toolset.rs`). Proven by
  `a_busy_namespace_lists_in_full_despite_the_upstream_response_bound`
  (integration-kubernetes, fake apiserver serving 120 × ~12 KiB raw objects — red at the merge
  base with the live refusal text), and
  `a_busy_namespace_is_listed_whole_without_a_paging_surface` /
  `a_pathological_namespace_is_cut_with_an_explicit_truncation_marker` (server). Live
  re-verification against the dev cluster's `latest` namespace follows the next deploy.
