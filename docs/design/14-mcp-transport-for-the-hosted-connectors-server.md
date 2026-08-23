# 14 — MCP transport for the hosted Connectors server

Status: accepted 2026-08-24. Backs stories S-053, S-054, S-055, S-056 (epic `mcp-entry`).
Decisions fixed by Timo, 2026-08-24: exactly three meta-tools; deployment through the existing
dev release machinery; pod logs as a new hosted operation.

## What this is

The hosted Connectors server gains a second inbound transport: an MCP endpoint at
`POST {base_path}/mcp`, speaking JSON-RPC 2.0 (MCP revisions 2025-03-26 and 2025-06-18),
stateless, without SSE or session ids. It is an entry point to functionality that already
exists — search, describe, invoke, and datasource reads — not a new authority surface.

`tools/list` returns exactly `tool_search`, `tool_describe`, `tool_invoke`. The projected,
simplified tool names (`k8s_namespace_list`, `k8s_deployment_list`, `k8s_deployment_status`,
`k8s_pod_status`, `k8s_pod_logs`) are data returned by `tool_search` and accepted by
`tool_invoke`; they live in a static, versioned table (`TOOLSET_VERSION`) in
`crates/server/src/hosted/mcp/toolset.rs`.

## The one rule that matters: every MCP call funnels through the admission seam

The existing private axum handlers `operation()` and `datasource()` in
`crates/server/src/hosted.rs` split into verify + decide halves; the decide halves
(`operation_decided`, `datasource_decided`) carry the entire existing body — scope map,
`HostedAdmissionPolicy`, re-describe, `HostedAuthority` grant/approval enforcement, dispatch.
`hosted/mcp.rs` verifies the bearer once per request and then reaches backends only through
those decided seams. `mcp.rs` and `toolset.rs` never name `.backend`, `ConnectorBackend`,
`dispatch_admitted`, or `admit_*`; a catalog-invariant sibling of rule 16 scans for exactly
that, and rule 16's doc comment names `/mcp` as a funneled entry. MCP therefore adds zero
policy: search/describe ride `connectors.catalog.read`, op-backed invoke rides
`connectors.invoke`, datasource reads ride `connectors.catalog.read`, and the receiver policy
plus Grants apply unchanged.

Role projection is derived, never re-implemented: `tool_search` executes the caller's own
operation search and datasource bindings through the seams and shows a table entry only when
its requirement is satisfied by those principal-filtered results. A renamed or withdrawn
underlying operation degrades to a hidden tool, never to a bypass.

## Leases are hidden from MCP callers

`tool_describe` returns name, title, description, effect, approval and schemas — no
`description_ref`. `tool_invoke` re-describes server-side, builds the invoke with the fresh
lease, and on `stale_authority` silently re-describes once before surfacing the refusal.
Refusals map to MCP results with `isError: true` and `structuredContent.code` carrying the
protocol's error code.

## kubernetes.pod.logs

A hosted read-only operation (approval not required) beside `kubernetes.deployment.status`:
input `{namespace, pod, container?, tail_lines 1..=1000 (default 200), since_seconds?}`, backed
by `GET /api/v1/namespaces/{ns}/pods/{pod}/log` with `limitBytes` 128 KiB, output
`{namespace, pod, container?, tail_lines, text, truncated}`. The handler serializes, then trims
whole oldest lines until the envelope fits `MAX_RESULT_BYTES`, forcing `truncated: true` — a
log read can never trip `result_too_large`. Admission: the same `namespace_access.read_groups`
gate as deployment status, via one arm in `admits_kubernetes_operation`. RBAC needs the
`pods/log` `get` subresource, added in the deployment chart. Logs may contain secrets; the
group gate and the namespace allowlist are the containment, and that is stated here rather
than assumed. The local placement does not advertise the operation yet (default trait method
refuses `unavailable`); local support is an open follow-up.

## Identity scope subset (S-055, lands in beyond10x/identity)

Identity admits any subset of the four self-service scopes (`connectors.catalog.read`,
`connectors.connections.self`, `connectors.events.self`, `connectors.invoke`) without the
`operator` group; any other member in the requested set still demands `operator`. Without this,
an `sre`/`dev` principal cannot hold `catalog.read invoke` in one token and the MCP endpoint is
operator-only in practice.

## Deliberately deferred

- `connectors mcp` stdio bridge with automatic login/refresh (S-056): does not fit the
  thin-CLI fence without first growing a `connectors-client` module; the HTTP endpoint is
  independently usable with a bearer.
- Monitoring-backed tools in the toolset; local-placement pod logs; grant-seeding surface for
  effect-bearing MCP invocations.

## Known costs

MCP responses carry both a text block and `structuredContent` (~2x payload, worst case
~512 KiB). Access tokens live 300 s; interactive clients see 401 after expiry until S-056.
