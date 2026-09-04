---
format: aep.planning-md/1
id: story:personal-local-workload-read
kind: story
status: draft
title: Personal-local Kubernetes serves a callable workload inventory
revision: 2
---
## Context

Personal-local Kubernetes publishes two operations and one datasource, and only the operations can
be called from this machine.

| surface | personal-local | hosted |
|---|---|---|
| `kubernetes.deployment.status` | yes | yes |
| `kubernetes.deployment.rollout-restart` | yes | yes |
| `kubernetes.pod.logs` | no | yes |
| `kubernetes.workloads` datasource | **served, not callable** | yes (MCP) |
| `kubernetes.databases` datasource | no | yes |

`KubernetesLocalBackend::capabilities` returns `datasources: true` and `owns_datasource` delegates
to `WorkloadSurface::owns` (`crates/integration-kubernetes/src/local.rs`), so the daemon answers
`b10x.connector-datasource.v0alpha1` for `kubernetes.workloads`. The `connectors` CLI has no
`datasource` verb at all — its command tree is login, logout, mcp, admin, init, auth, providers,
doctor, serve, serve-hosted, connect, connection, event, operation, completions — so nothing an
operator can run reaches it. The hosted placement reaches the same projection through MCP.

The gap that leaves: the only way to learn what runs in an activated cluster is to already know a
deployment's namespace and name and ask `kubernetes.deployment.status` for it one at a time. There
is no answer to "what is deployed here", "what image is this running", or "which namespaces are
admitted" — which is the first question anything reading a cluster asks.

Found on 2026-09-04 while proving that every authorized cluster serves reads (two EKS contexts
activated, both answered `kubernetes.deployment.status` for `kube-system/coredns` with distinct
generations).

## Acceptance

An operator with an activated Kubernetes Connection can list, from the `connectors` CLI, the
namespaces that Connection admits and, per namespace, each deployment with its container images,
desired and ready replica counts — without naming a deployment first. The reads go through the same
admission the workload operations already pass, are refused for a Connection that was never
activated, and every authorized Connection answers rather than one of them.

## Scope

- `crates/integration-kubernetes/src/local.rs` — the personal-local dispatch, whichever surface the
  decision below picks.
- `crates/integration-kubernetes/src/local_workloads.rs` — the projection already exists; what is
  missing is a caller.
- `crates/connectors-cli/src/lib.rs` — a verb, if the decision is to expose the datasource protocol
  rather than to add operations.

## Notes

Two shapes, and the choice is the first thing to settle:

1. **A `connectors datasource` verb** — search, describe, bindings, read — mirroring `operation`.
   It makes the whole `b10x.connector-datasource.v0alpha1` protocol reachable, so the hosted
   `kubernetes.databases` projection would light up on personal-local for free the day it is
   composed there. It is also a new public CLI surface with its own lease and paging vocabulary.
2. **Catalogued operations** (`kubernetes.workload.list`, `kubernetes.namespace.list`) beside the
   two that exist. Smaller, reuses the description-lease path a caller already knows, and needs no
   new protocol verb — but it publishes a second, narrower answer to a question the datasource
   already answers, and the two would drift.

Not implemented here by instruction; this story records the gap and the fork.
