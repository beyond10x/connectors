---
format: aep.planning-md/1
id: story:kubernetes-discovers-database-endpoints
kind: story
status: implemented
title: Kubernetes discovers database endpoints
refs:
- provider: legacy
  reference: S-059
relations:
- derived_from: epic:endpoint-plane
scope:
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-059-kubernetes-discovers-database-endpoints.md:23`. **read**

- A datasource (bindings per admitted namespace; read verbs list/get) returns the derived
  endpoint descriptors for both engines, proven against fakes shaped like the real
  `database.{mysql,postgresql}.sql.crossplane.io` resources; secret references are named,
  secret values never read.
- The RBAC the deployment needs (get/list on those CRDs) is stated in the story's close note
  for the chart, and admission stays behind the same namespace `read_groups`.
- The catalog invariant family covers the new seam; `bash scripts/gate.sh` exits 0.

## Context

The admitted namespace carries the endpoint inventory as Crossplane resources (measured on the
dev cluster's `latest`: 24 MySQL + 23 PostgreSQL database resources, 35 connection secrets).
The hosted Kubernetes integration becomes an endpoint discoverer: a read-only datasource over
the admitted namespaces that lists database managed resources and derives endpoint descriptors
— engine, host, port, database, connection-secret reference — never credential bytes. The
descriptors are what S-058's connections consume, so a discovered database is usable without
anyone writing configuration.

Source frontmatter: pillar Platform · areas [integrations, server] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-059-kubernetes-discovers-database-endpoints.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-059-kubernetes-discovers-database-endpoints.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-059`, recorded as the reference `legacy:S-059`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
