---
format: aep.planning-md/1
id: story:discovery-reads-the-cluster-scoped-database-resources
kind: story
status: implemented
title: Discovery reads the cluster-scoped database resources
refs:
- provider: legacy
  reference: S-062
relations:
- derived_from: epic:endpoint-plane
scope:
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-062-discovery-reads-the-cluster-scoped-database-resources.md:29`. **read**

- The reader lists the cluster-scoped collection and follows `spec.providerConfigRef` to the
  `ProviderConfig`, emitting `{engine, name, provider_config, secret_ref{name,namespace}, ready}`;
  secret VALUES are still never read — endpoint resolution stays with the SQL driver's
  credential custody at connect time.
- Namespace association is derived honestly (the secret reference's namespace), and the
  datasource description stops promising "namespace-scoped" facts the resources do not declare.
- A wrong-scope regression is impossible to reintroduce silently: a 404 from a group that IS
  served (its discovery document lists `databases`) surfaces as an error, not an empty list.
- e2e on dev: the list read returns 24 + 23 records with correct engines and ready flags.

## Context

`kubernetes.databases` returns zero records on the dev cluster while 24 MySQL and 23 PostgreSQL
`Database` resources exist (e2e 2026-08-24, release rev 14). Two mismatches with the real
provider-sql shape, found live:

- The reader lists `/apis/{group}/v1alpha1/namespaces/{ns}/databases`
  (`crates/integration-kubernetes/src/hosted.rs:608-615`), but the CRDs are **cluster-scoped**
  (`kubectl api-resources`: NAMESPACED=false). The API answers 404, which the
  404-means-no-Crossplane mapping silently turns into an empty inventory — the wrong path is
  indistinguishable from an absent provider.
- The live `Database` spec carries **no endpoint facts** — only `providerConfigRef` and
  `deletionPolicy`; host/port live behind the referenced `ProviderConfig`'s
  `credentials.connectionSecretRef` (name + namespace). The descriptor model
  `{host, port, database, secret_ref}` cannot be filled from the Database resource alone.

Source frontmatter: pillar Platform · areas [integrations, server] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-062-discovery-reads-the-cluster-scoped-database-resources.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-062-discovery-reads-the-cluster-scoped-database-resources.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-062`, recorded as the reference `legacy:S-062`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
