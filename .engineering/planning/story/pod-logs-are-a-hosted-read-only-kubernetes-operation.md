---
format: aep.planning-md/1
id: story:pod-logs-are-a-hosted-read-only-kubernetes-operation
kind: story
status: implemented
title: Pod logs are a hosted read-only Kubernetes operation
refs:
- provider: legacy
  reference: S-054
relations:
- derived_from: epic:mcp-entry
scope:
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-054-pod-logs-are-a-hosted-read-only-kubernetes-operation.md:25`. **read**

- Search lists `kubernetes.pod.logs` for read-group principals and hides it otherwise;
  describe returns input/output schemas and a lease; invoke passes container/tail_lines/
  since_seconds through to the reader, proven against a fake.
- A non-admitted namespace refuses `not_granted`; a stale lease refuses `stale_authority`;
  an oversized log body yields front-trimmed output with `truncated: true` inside a
  validating envelope; upstream 403 maps to `not_granted`; input caps are enforced.
- `bash scripts/gate.sh` exits 0.

## Context

Add `kubernetes.pod.logs` (effect read_only, approval not_required) to the hosted Kubernetes
integration per design 14: input `{namespace, pod, container?, tail_lines 1..=1000 default
200, since_seconds 1..=86400?}`, backed by the pod log subresource with `limitBytes` 128 KiB,
output `{namespace, pod, container?, tail_lines, text, truncated}` with serialize-then-trim-
oldest-lines so the envelope never trips `result_too_large`. A default `DeploymentReader`
trait method refuses `unavailable` so the local placement and test fakes compile unchanged.
Gate admission on the same `namespace_access.read_groups` as deployment status via one arm in
`admits_kubernetes_operation`; search advertises the operation only under read access, with
`log`/`logs`/`pod` terms.

Source frontmatter: pillar Platform · areas [integrations, server] · design `../design/14-mcp-transport-for-the-hosted-connectors-server.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-054-pod-logs-are-a-hosted-read-only-kubernetes-operation.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-054-pod-logs-are-a-hosted-read-only-kubernetes-operation.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-054`, recorded as the reference `legacy:S-054`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
