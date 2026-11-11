---
format: aep.planning-md/3
id: story:kubernetes-reads-more-kinds-configmaps-metadata-only
kind: story
status: draft
title: Kubernetes resource reads cover daemonsets, jobs, cronjobs, HPAs, ingresses and configmap metadata
summary: Six more resources.list/get kinds; configmaps never return data or binaryData
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
Requested by a downstream consumer, 2026-11-10.

## Outcome

`resources.list` and `resources.get` read six more kinds:

- daemonsets
- jobs
- cronjobs
- horizontalpodautoscalers
- ingresses
- configmaps, as names and metadata only

Today `collection` admits exactly pods, services, events, deployments, replicasets, statefulsets
and endpointslices (`adapters/kubernetes/src/lib.rs:825-847`). Configuration validation and
dispatch both read that closed set.

## Acceptance

- Each new kind is admitted in `resource_kinds` and binds its one collection path:
  - `apps/v1` daemonsets;
  - `batch/v1` jobs and cronjobs;
  - `autoscaling/v2` horizontalpodautoscalers;
  - `networking.k8s.io/v1` ingresses;
  - `v1` configmaps.
  A recorded-fixture test per kind pins the request path for list and get.
- For configmaps, a test pins that the answer never has the `data` or `binaryData` members, for
  both list and get, even when the fixture's upstream answer carries them. The answer keeps
  `metadata`.
- A configuration that does not list a new kind does not change its descriptor or revision. A
  test pins this.
- `docs` and the Kubernetes contract name the new kinds and the configmap projection.

## Out of scope

- Secrets (story:parity-kubernetes-secret-read).
- Writes.
- Custom resources, which `custom_resources.list` already covers.

## Ambiguities

- Whether configmap metadata keeps `annotations`. The
  `kubectl.kubernetes.io/last-applied-configuration` annotation can repeat the whole `data`. The
  safe default drops that one annotation; the test should pin whichever is decided.

## Consumer preference (the downstream consumer, 2026-11-11)

This is the consumer's preference, not a decision. The ambiguity above stays open for the
operator.

- For configmaps, strip the `kubectl.kubernetes.io/last-applied-configuration` annotation and
  `managedFields`.
- Return:
  - name and namespace;
  - labels and the remaining annotations;
  - the key names and their sizes;
  - the timestamps.
