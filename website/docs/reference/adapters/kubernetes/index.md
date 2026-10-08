---
title: Kubernetes
sidebar_position: 3
description: Kubernetes resource inventory, endpoint and host discovery and Helm release reads, and the native profiles specified beyond them.
---

# Kubernetes

Inventory resources and discover candidate endpoints.

## Current capabilities

The adapter implements `resources.list`, `endpoints.discover` and optionally `hosts.discover`.
Configuration selects namespaces and resource kinds. A separate `helm_release_reads` selection
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
unless the table above names them. The repository's
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
