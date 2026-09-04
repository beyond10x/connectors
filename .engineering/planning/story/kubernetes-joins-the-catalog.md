---
format: aep.planning-md/1
id: story:kubernetes-joins-the-catalog
kind: story
status: draft
title: Kubernetes joins the catalog
refs:
- provider: legacy
  reference: S-057
revision: 2
---
## Acceptance

Verbatim from `docs/stories/S-057-kubernetes-joins-the-catalog.md:25`. **read**

- `providers/kubernetes.toml` (repository-authored spec with provenance) catalogs the current
  operation surface; the generic `CatalogBackend` executes it in-cluster behind the same
  admission outcomes the hand-written backend produced, proven by the existing hosted tests
  running against the declarative path.
- The generic executor's new primitives (file-sourced bearer, namespace group policy, bounded
  text response) are declared, not hard-coded to Kubernetes.
- The deleted Rust is actually deleted; `integration-kubernetes` retains only what no
  declaration can express, each retention justified in its module doc.

## Context

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

Source frontmatter: pillar Platform · areas [integrations, catalog, server]. **read**

## Status

`backlog` in the source. Quoted from `docs/stories/S-057-kubernetes-joins-the-catalog.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-057-kubernetes-joins-the-catalog.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 1 revision(s)
- Legacy id `S-057`, recorded as the reference `legacy:S-057`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
