---
format: aep.planning-md/1
id: story:the-deployment-list-pages-instead-of-refusing
kind: story
status: implemented
title: The deployment list pages instead of refusing
refs:
- provider: legacy
  reference: S-063
relations:
- derived_from: epic:mcp-entry
scope:
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-063-the-deployment-list-pages-instead-of-refusing.md:27`. **read**

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

## Context

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

Source frontmatter: pillar Platform · areas [integrations, server] · design `../design/14-mcp-transport-for-the-hosted-connectors-server.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-063-the-deployment-list-pages-instead-of-refusing.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-063-the-deployment-list-pages-instead-of-refusing.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-063`, recorded as the reference `legacy:S-063`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
