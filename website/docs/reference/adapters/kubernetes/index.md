---
title: Kubernetes
sidebar_position: 3
description: Kubernetes resource inventory, single-object, namespace, rollout-history and pod-log reads, kubeconfig contexts, pod exec under approval, endpoint and host discovery and Helm release reads, and the native profiles specified beyond them.
---

# Kubernetes

Inventory resources and discover candidate endpoints.

## Current capabilities

The adapter implements `resources.list`, `resources.get`, `namespaces.list`,
`deployments.history`, `endpoints.discover` and optionally `hosts.discover`. Configuration selects
namespaces and resource kinds: `pods`, `services`, `deployments`, `endpointslices`, `replicasets`
and `events`.

| id | what it reads |
|---|---|
| `resources.list` | a bounded page of one configured kind in one configured namespace; events and ReplicaSets are kinds |
| `resources.get` | one object of a configured kind by name; an absent object answers `not_found`, not an empty page |
| `namespaces.list` | each configured namespace by its exact name, omitting the ones the cluster does not have; the cluster's namespace list is never read |
| `deployments.history` | the ReplicaSets one Deployment controls by owner uid, each with its rollout revision; needs both `deployments` and `replicasets` configured |

A namespace or kind outside the configured scope is refused before any request. These three
reads are checked against API-shaped fixtures.

Three more operations are each enabled by their own configuration field:

| id | enabled by | what it does |
|---|---|---|
| `pods.logs` | `pod_logs: true` | reads a bounded tail of one container's log in a configured namespace: at most 1,000 lines, 128 KiB and one day back, timestamped, in the cluster's order; no follow |
| `contexts.list` | `kubeconfig`, the absolute path of an owner-only kubeconfig, in the local configuration | lists that file's contexts (name, cluster, namespace, current flag), never credentials or servers; makes no cluster request and reads no other file |
| `pods.exec` | `pod_exec: true`, local CLI only | runs one command, an argument vector with no shell, in a named container; a write on private protocol two that needs an approval and is recorded as an attempt; returns the exit status and stdout and stderr, each bounded, within at most 60 s; a lost stream is an unknown outcome and is never sent again |

`pods.exec` is the adapter's only write; the read exchange and the standalone service never list
it, and it has no stdin or terminal. The three are checked against recorded API answers and
streams, not a live cluster. A separate `helm_release_reads` selection
additionally advertises the `helm_releases.*` release-state reads (history, status, values and
manifest), whose values and manifests are disclosed only as redacted projections. It runs as a
standalone service ([Getting started](../../../getting-started.md) starts one) or under the local
CLI with a saved bearer token, validated by one SelfSubjectReview.

Real k3s acceptance on 2026-10-02 covered selected reads across restart, authenticated RBAC
denial versus empty results, continuation scope and revision binding, and local repair,
revocation, custody and child-stop controls. These observations do not establish per-operation
SelfSubjectAccessReview checks, which are not implemented, or the remaining execution and Helm
workflows.

## Access and limits

Bind the intended Kubernetes API endpoint, bearer credential and certificate authority
explicitly. Discovery returns observations with provenance and reachability context; it does not
connect to discovered addresses. A discovered database endpoint grants no access to the database.

Resource discovery state, richer authentication, logs, mutations and mediated routes are
described in native specifications. These profiles are not supported by the current binary
unless the tables above name them. The repository's
[Kubernetes CLI guide](https://github.com/beyond10x/connectors/blob/main/docs/local-kubernetes-cli.md)
covers the local configuration and saved token.

## Native contract reference

The [contract reference](../../contracts/index.md#adapter-owned-native-contracts) gives each page's support status.

- [Kubernetes reads](./contracts/reads.md)
- [Kubernetes Helm release reads](./contracts/helm.md)
- [Kubernetes authentication](./contracts/auth.md)
- [Kubernetes discovery](./contracts/discovery.md)
- [Kubernetes logs](./contracts/logs.md)
- [Kubernetes mutations](./contracts/mutations.md)
- [Kubernetes mediated routes](./contracts/routes.md)

The [typed native model](./model/index.md) is generated from `adapters/kubernetes/spec/ess`.
