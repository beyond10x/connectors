---
id: S-062
title: "Discovery reads the cluster-scoped database resources"
pillar: Platform
status: ready
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
