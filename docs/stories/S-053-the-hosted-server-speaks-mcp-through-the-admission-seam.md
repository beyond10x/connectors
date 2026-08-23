---
id: S-053
title: "The hosted server speaks MCP through the admission seam"
pillar: Platform
status: ready
priority: 1
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
