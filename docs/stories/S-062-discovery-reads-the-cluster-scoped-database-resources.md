---
id: S-062
title: "Discovery reads the cluster-scoped database resources"
pillar: Platform
status: done
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [integrations, server]
---

# Discovery reads the cluster-scoped database resources

## Goal

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

## Acceptance

- The reader lists the cluster-scoped collection and follows `spec.providerConfigRef` to the
  `ProviderConfig`, emitting `{engine, name, provider_config, secret_ref{name,namespace}, ready}`;
  secret VALUES are still never read — endpoint resolution stays with the SQL driver's
  credential custody at connect time.
- Namespace association is derived honestly (the secret reference's namespace), and the
  datasource description stops promising "namespace-scoped" facts the resources do not declare.
- A wrong-scope regression is impossible to reintroduce silently: a 404 from a group that IS
  served (its discovery document lists `databases`) surfaces as an error, not an empty list.
- e2e on dev: the list read returns 24 + 23 records with correct engines and ready flags.

## Notes

- Found by live e2e; the conformance-style fake in `hosted_database_tests.rs` modeled the
  namespaced shape and passed — the fake must be corrected to the real cluster-scoped shape.
- **Decision (2026-08-24): the namespace gate over a cluster-scoped inventory is the
  connection secret's namespace.** The binding stays per admitted namespace (the same
  `read_groups` admission as every other Kubernetes datasource), and a binding lists exactly
  the Databases whose referenced ProviderConfig keeps its `credentials.connectionSecretRef`
  in that namespace — on the dev cluster all 47 resources resolve to secrets in the product
  namespace (`latest`), so the existing binding sees the whole inventory. A Database whose
  secret lives in a non-admitted namespace, whose `providerConfigRef` dangles, or whose
  ProviderConfig carries no secret reference associates with **no** binding: it is excluded
  (and a get through the binding answers `not_found`), never guessed into one and never an
  error. Rationale: the secret's namespace is the one honest, admission-relevant namespace
  fact the resource chain declares — it is where the credential custody (S-058) will read —
  and gating on it keeps "you may see what your namespace holds" true for cluster-scoped
  resources without inventing a second admission model.

## Progress

- 2026-08-24 — filed from the live e2e findings (release rev 14: 0 records where 24 + 23 exist).
- 2026-08-24 — implemented on `impl/S-062`. The reader lists the CLUSTER-scoped collections
  (`/apis/{group}/v1alpha1/databases`) and joins each Database's `spec.providerConfigRef`
  against the group's cluster-scoped ProviderConfigs (one bounded page; a truncated
  ProviderConfig list is refused rather than joined half-read). Descriptors are now
  `{engine, name, provider_config, secret_ref{name, namespace}, ready}` (projection version 2)
  — the host/port/database fields the resources never declared are gone, and the secret
  reference comes from the ProviderConfig's `credentials.connectionSecretRef`; no code path
  reads a Secret value. Wrong-scope honesty: a 404 on a collection list consults the group's
  own discovery document (`/apis/{group}/v1alpha1`) — group absent → empty inventory, group
  served and listing the collection → `unavailable` error (`absent_collection_is_empty` in
  `crate::databases`). The datasource description now says cluster-scoped and names the
  per-binding association. The fake in `hosted_database_tests.rs` models the real
  cluster-scoped shape (no `metadata.namespace`, spec `{deletionPolicy, providerConfigRef}`,
  ProviderConfig join), which is what failed RED against the old namespaced wire types.
  Failing-first proof at merge base 97c8616: `missing field namespace` parse panic. Still
  open: the live e2e read (24 + 23 records) needs a deployment; **chart RBAC must move to a
  ClusterRole** — cluster-scoped get+list on plural `databases` AND `providerconfigs` in both
  groups (`mysql.sql.crossplane.io`, `postgresql.sql.crossplane.io`); the namespaced RBAC
  S-059 recorded cannot authorize the cluster-scoped list.

## Superseded by

`story:discovery-reads-the-cluster-scoped-database-resources` in the AEP planning store, at
`.engineering/planning/story/discovery-reads-the-cluster-scoped-database-resources.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
