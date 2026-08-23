---
id: S-053
title: "The hosted server speaks MCP through the admission seam"
pillar: Platform
status: done
design: ../design/14-mcp-transport-for-the-hosted-connectors-server.md
epic: mcp-entry
areas: [server, testing]
---

# The hosted server speaks MCP through the admission seam

## Goal

Add `POST {base_path}/mcp` to the hosted router: stateless JSON-RPC 2.0 MCP (revisions
2025-03-26/2025-06-18; no SSE, no session id, no new dependency) with methods `initialize`,
`notifications/*` (202), `ping`, `tools/list` (exactly `tool_search`, `tool_describe`,
`tool_invoke`), `tools/call`. Split the private `operation()`/`datasource()` handlers into
verify + `*_decided` halves inside `hosted.rs`; the MCP modules (`hosted/mcp.rs`,
`hosted/mcp/toolset.rs`) verify the bearer once and reach backends only through the decided
seams. Toolset v1 per design 14 (`k8s_namespace_list`, `k8s_deployment_list`,
`k8s_deployment_status`, `k8s_pod_status`, `k8s_pod_logs`), role-filtered by the caller's own
seam results; `description_ref` hidden with server-side re-describe and one stale-authority
retry. Include `k8s_pod_logs` in the table even though S-054 lands the operation — the
requirement mechanism hides it until the op exists, which also makes the two stories
order-independent.

## Acceptance

- `tools/list` returns exactly the three meta-tools; a read-group principal's `tool_search`
  lists the k8s entries its seam results support, a group-less principal's lists none.
- An op-backed `tool_invoke` provably performs Describe then Invoke through the decided seam
  with a matching lease; datasource-backed tools route through the datasource seam; refusal
  paths surface as `isError` + `structuredContent.code` (`not_granted` without
  `connectors.invoke`, `approval_required` with evidence passthrough proven, stale-authority
  retried once); 401/-32700/-32602/413 behave per design 14.
- A catalog-invariant sibling of rule 16 forbids `.backend`/`ConnectorBackend`/
  `dispatch_admitted`/`admit_` tokens in the MCP modules' production lines, and rule 16's doc
  comment names `/mcp` as a funneled entry.
- `bash scripts/gate.sh` exits 0 (module fence: `hosted.rs` stays under 1500 — the
  connect/oauth handler move to `hosted/connect.rs` is the sanctioned valve if needed).

## Progress

- 2026-08-24 — filed from design 14 (Timo's decisions: three meta-tools, entry point on the
  hosted server, no new service).
- 2026-08-24 — implemented on `impl/S-053`: `POST /mcp` registered in the hosted router
  (512 KiB frame bound; GET/DELETE answer 405); `operation()`/`datasource()` split into
  verify + `operation_decided`/`datasource_decided` halves inside `hosted.rs` (1472 lines,
  fence holds without the connect.rs valve); new `hosted/mcp.rs` (JSON-RPC layer, bearer
  verified once before any JSON-RPC processing, server name `b10x-connectors`) and
  `hosted/mcp/toolset.rs` (TOOLSET_VERSION v1, five entries incl. requirement-hidden
  `k8s_pod_logs`, seam-derived role projection, server-side describe→invoke with one silent
  stale-authority retry, approval_required pre-check with evidence passthrough). Rule 17
  invariant lands beside rule 16 (dated note on 16 names `/mcp` as a funneled entry) and was
  demonstrated red against a planted `.backend` token, then restored byte-identically.
  Eleven route tests in `crates/server/src/hosted/tests/mcp.rs` cover the Acceptance; all
  failed 404 at base 2bb7d40 before the route existed.
- 2026-08-24 — merged to main after independent review (PASS, 0 blocking, 1 minor: the rule
  16/17 line scanner's string-literal `//` limitation, a pre-existing shared mechanic). The
  verified-split was independently confirmed byte-identical (datasource) / call-site-only
  (operation); spoofing, evidence-not-demanded, retry-once and robustness probes all held.
