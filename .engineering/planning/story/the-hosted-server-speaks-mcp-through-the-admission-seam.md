---
format: aep.planning-md/1
id: story:the-hosted-server-speaks-mcp-through-the-admission-seam
kind: story
status: implemented
title: The hosted server speaks MCP through the admission seam
refs:
- provider: legacy
  reference: S-053
relations:
- derived_from: epic:mcp-entry
scope:
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-053-the-hosted-server-speaks-mcp-through-the-admission-seam.md:28`. **read**

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

## Context

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

Source frontmatter: pillar Platform · areas [server, testing] · design `../design/14-mcp-transport-for-the-hosted-connectors-server.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-053-the-hosted-server-speaks-mcp-through-the-admission-seam.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-053-the-hosted-server-speaks-mcp-through-the-admission-seam.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-053`, recorded as the reference `legacy:S-053`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
