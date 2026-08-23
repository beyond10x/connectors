---
id: S-054
title: "Pod logs are a hosted read-only Kubernetes operation"
pillar: Platform
status: ready
priority: 2
design: ../design/14-mcp-transport-for-the-hosted-connectors-server.md
epic: mcp-entry
areas: [integrations, server]
---

# Pod logs are a hosted read-only Kubernetes operation

## Goal

Add `kubernetes.pod.logs` (effect read_only, approval not_required) to the hosted Kubernetes
integration per design 14: input `{namespace, pod, container?, tail_lines 1..=1000 default
200, since_seconds 1..=86400?}`, backed by the pod log subresource with `limitBytes` 128 KiB,
output `{namespace, pod, container?, tail_lines, text, truncated}` with serialize-then-trim-
oldest-lines so the envelope never trips `result_too_large`. A default `DeploymentReader`
trait method refuses `unavailable` so the local placement and test fakes compile unchanged.
Gate admission on the same `namespace_access.read_groups` as deployment status via one arm in
`admits_kubernetes_operation`; search advertises the operation only under read access, with
`log`/`logs`/`pod` terms.

## Acceptance

- Search lists `kubernetes.pod.logs` for read-group principals and hides it otherwise;
  describe returns input/output schemas and a lease; invoke passes container/tail_lines/
  since_seconds through to the reader, proven against a fake.
- A non-admitted namespace refuses `not_granted`; a stale lease refuses `stale_authority`;
  an oversized log body yields front-trimmed output with `truncated: true` inside a
  validating envelope; upstream 403 maps to `not_granted`; input caps are enforced.
- `bash scripts/gate.sh` exits 0.

## Progress

- 2026-08-24 — filed from design 14 (pod logs chosen as a new hosted operation over Loki).
