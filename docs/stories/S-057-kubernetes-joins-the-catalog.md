---
id: S-057
title: "Kubernetes joins the catalog"
pillar: Platform
status: backlog
areas: [integrations, catalog, server]
---

# Kubernetes joins the catalog

## Goal

Kubernetes is the last hand-coded HTTP surface: every other REST provider is declared in
`providers/*.toml` and executed by the one generic adapter (`integration-catalog` — "One adapter
for every declared provider"). The Kubernetes integration stays Rust only because the generic
executor lacks three primitives: a file-sourced bearer that is re-read per call (the in-cluster
ServiceAccount token), per-namespace group policy (`namespace_access.read_groups` is finer than
tenant Grants), and bounded plain-text responses (pod logs). Teach the generic executor those
primitives, declare the Kubernetes API surface as a repository-authored spec + provider TOML
(deployment status, rollout restart, pod logs — grounded in the official API reference, per the
authored-spec discipline), and delete the corresponding hand-written paths from
`integration-kubernetes`. The workloads datasource (pods+events join) may remain code; it is a
projection, not an HTTP call.

## Acceptance

- `providers/kubernetes.toml` (repository-authored spec with provenance) catalogs the current
  operation surface; the generic `CatalogBackend` executes it in-cluster behind the same
  admission outcomes the hand-written backend produced, proven by the existing hosted tests
  running against the declarative path.
- The generic executor's new primitives (file-sourced bearer, namespace group policy, bounded
  text response) are declared, not hard-coded to Kubernetes.
- The deleted Rust is actually deleted; `integration-kubernetes` retains only what no
  declaration can express, each retention justified in its module doc.

## Progress

- 2026-08-24 — filed from Timo's "it's just HTTP" review of S-054; depends on the mcp-entry
  epic landing first (S-054 defines the pod-logs surface this story re-declares).

## Superseded by

`story:kubernetes-joins-the-catalog` in the AEP planning store, at
`.engineering/planning/story/kubernetes-joins-the-catalog.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
