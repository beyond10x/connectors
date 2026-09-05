---
format: aep.planning-md/1
id: story:personal-local-workload-read
kind: story
status: active
title: Personal-local Kubernetes serves a callable workload inventory
tags:
- ready
- wave-cli
scope:
- confidence: cited
  path: crates/connectors-runtime/Cargo.lock
- confidence: cited
  path: crates/integration-kubernetes/Cargo.toml
- confidence: cited
  path: crates/integration-kubernetes/src/hosted_tests.rs
- confidence: inferred
  path: crates/integration-kubernetes/src/lib.rs
- confidence: cited
  path: crates/integration-kubernetes/src/local.rs
- confidence: inferred
  path: crates/integration-kubernetes/src/local_inventory.rs
- confidence: inferred
  path: crates/integration-kubernetes/src/local_inventory_tests.rs
- confidence: cited
  path: crates/integration-kubernetes/src/local_workloads.rs
- confidence: cited
  path: crates/integration-kubernetes/src/workloads.rs
- confidence: inferred
  path: docs/guides/connect-kubernetes.md
revision: 24
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

## Original scope requirements

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

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 7 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `crates/integration-kubernetes/src/local.rs` — cited.
- `crates/integration-kubernetes/src/local_workloads.rs` — cited.
- `crates/integration-kubernetes/src/workloads.rs` — cited.
- `crates/integration-kubernetes/src/local_inventory.rs` — inferred.
- `crates/integration-kubernetes/src/lib.rs` — inferred.
- `crates/integration-kubernetes/src/local_inventory_tests.rs` — inferred.
- `docs/guides/connect-kubernetes.md` — inferred.

Medium confidence. Choose the two bounded read operations kubernetes.namespace.list and kubernetes.workload.list through existing operation CLI. Reuse selected activated Connection admission and bounded reader; never select the first cluster. The committed connectors.inventory ESS value types define results without claiming Kubernetes ownership/lifecycle. Namespace list means the configured admitted namespaces; empty does not mean every namespace. Template images survive zero pods/replicas. Existing compact datasource shape stays compatible.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 5 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.
