---
format: aep.planning-md/3
id: story:served-kubernetes-playground-pod-create
kind: story
status: draft
title: The served Kubernetes adapter creates pods only in playground namespaces
summary: pods.create, opt-in, playground namespaces a subset of write_namespaces; admitted pod spec fields need the operator's decision
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:served-kubernetes-restart-scale-and-delete
revision: 1
---
Requested by Assembly (assembly-dev session), 2026-11-10.

## Outcome

The served Kubernetes adapter offers `pods.create`, with `effect: write`. It is separate from the
other writes and comes later. It runs only in the namespaces the instance configuration lists as
playground, and those must be a subset of `write_namespaces`. A configuration that breaks this is
refused at start. The operation is absent from the descriptor unless it is enabled.

## Acceptance

- With `pods.create` not enabled, the descriptor does not list it. A test pins this.
- A create in a namespace outside the playground list is refused before any upstream request.
- A configuration whose playground namespaces are not a subset of `write_namespaces` is refused at
  start by a named reason.
- A pod spec outside the admitted field set is refused before any request, naming the field. The
  field set comes from the decisions below. One test per refused field.
- A recorded-fixture test pins the exact create request for an admitted spec.

## Out of scope

- Every other kind of object creation.
- Exec, attach and port-forward into the created pod.

## Ambiguities

These need the operator's decision before acceptance:

- Which pod spec fields are admitted: `hostNetwork`, `hostPID`, `hostIPC`, `privileged` and other
  `securityContext` members, `hostPath` and other volume types, and `serviceAccountName`.
- Resource limits: whether they are required, and their per-instance maximum.
- An image allow-list: whether there is one, and its form (registry, repository, digest).
