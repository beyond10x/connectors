---
format: aep.planning-md/3
id: story:served-kubernetes-restart-scale-and-delete
kind: story
status: draft
title: The served Kubernetes adapter restarts, scales and deletes pods only in its write namespaces
summary: Six opt-in write operations, write_namespaces a subset of namespaces, no secrets, exec, attach or port-forward
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:served-kubernetes-authenticates-in-cluster
- depends_on: story:served-kubernetes-admits-service-account-tokens
revision: 1
---
Requested by Assembly (assembly-dev session), 2026-11-10.

## Outcome

The served Kubernetes adapter offers six write operations. Each is its own operation with
`effect: write`, and each is absent from the descriptor unless the instance enables it:

| id | upstream |
|---|---|
| `deployments.restart` | rollout-restart: patch `spec.template.metadata.annotations["kubectl.kubernetes.io/restartedAt"]` |
| `statefulsets.restart` | the same patch on a StatefulSet |
| `daemonsets.restart` | the same patch on a DaemonSet |
| `deployments.scale` | patch the `scale` subresource's `spec.replicas`, at most the instance's maximum |
| `statefulsets.scale` | the same on a StatefulSet |
| `pods.delete` | delete one Pod by name |

Namespace rule:

- Every operation runs only in `namespaces` (`adapters/kubernetes/src/lib.rs:36-38`).
- Writes run only in a new `write_namespaces`, which must be a subset of `namespaces`. A
  configuration that breaks this is refused at start.
- A write outside `write_namespaces` is refused before any upstream request.

The operations reach no Secrets and never exec, attach or port-forward. Today the only mutation
is `pods.exec`, which the read path refuses as `forbidden` (`lib.rs:1346-1349`). The read
operations are listed at `lib.rs:145-161`.

## Acceptance

- With no write enabled, the descriptor holds no write operation and is byte-identical to today's.
  A test pins both.
- Each operation enabled alone appears with `effect: write` and goes through the approved write
  path. A recorded-fixture test pins the exact request: the method, the path, the patch body and
  its content type.
- A configuration whose `write_namespaces` is not a subset of `namespaces` is refused at start by
  a named reason.
- A write naming a namespace outside `write_namespaces` is refused before any upstream request
  (the fixture sees zero requests).
- A scale above the instance's maximum, or below 0, is refused before any request.
- No write operation can address a Secret, or the `exec`, `attach` or `portforward`
  subresources. A test tries each one and sees zero requests.

## Out of scope

- `pods.create` (story:served-kubernetes-playground-pod-create).
- Secret reads and writes, exec, attach and port-forward.
- Writes from the local CLI composition.

## Ambiguities

- Decision for the operator, not decided here: whether a write is ever served to a caller admitted
  by a static bearer (v2), or only to a caller admitted by a service-account token. This story
  depends on in-cluster authentication and service-account admission, and that order assumes the
  second. Write the decision down before acceptance.
- Whether the replica maximum is one value per instance or one per workload.
- Whether `pods.delete` takes a grace period, and what bounds it.
