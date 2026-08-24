---
id: S-059
title: "Kubernetes discovers database endpoints"
pillar: Platform
status: done
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [integrations, server]
---

# Kubernetes discovers database endpoints

## Goal

The admitted namespace carries the endpoint inventory as Crossplane resources (measured on the
dev cluster's `latest`: 24 MySQL + 23 PostgreSQL database resources, 35 connection secrets).
The hosted Kubernetes integration becomes an endpoint discoverer: a read-only datasource over
the admitted namespaces that lists database managed resources and derives endpoint descriptors
— engine, host, port, database, connection-secret reference — never credential bytes. The
descriptors are what S-058's connections consume, so a discovered database is usable without
anyone writing configuration.

## Acceptance

- A datasource (bindings per admitted namespace; read verbs list/get) returns the derived
  endpoint descriptors for both engines, proven against fakes shaped like the real
  `database.{mysql,postgresql}.sql.crossplane.io` resources; secret references are named,
  secret values never read.
- The RBAC the deployment needs (get/list on those CRDs) is stated in the story's close note
  for the chart, and admission stays behind the same namespace `read_groups`.
- The catalog invariant family covers the new seam; `bash scripts/gate.sh` exits 0.

## Progress

- 2026-08-24 — filed from design 15 (Timo: k8s is "not only operation provider but also
  endpoint discoverer").
- 2026-08-24 — implemented on `impl/S-059`: a second hosted datasource `kubernetes.databases`
  beside `kubernetes.workloads` — bindings per admitted namespace behind the same
  `read_groups` gate, read verbs list (paged) and get. Descriptors
  `{engine, name, host?, port?, database?, secret_ref{name, namespace}?, ready}` are derived
  from the Crossplane `databases.{mysql,postgresql}.sql.crossplane.io/v1alpha1` managed
  resources (`crate::databases`): engine from the API group, `status.atProvider` over
  `spec.forProvider`, absent facts stated as `null` rather than guessed, connection-secret
  references by name only — no code path reads a Secret. The in-cluster reader GETs the two
  namespaced CRD list endpoints with the shared bounded-read/per-call-token pattern; 403 maps
  to `not_granted` and 404 (CRD absent) to an empty inventory, so a cluster without
  Crossplane simply discovers nothing. Listing pages MySQL first then PostgreSQL through an
  authority-bound cursor store of its own; defaulted `DeploymentReader` methods keep the
  local placement and existing fakes compiling unchanged (the S-054 `pod_logs` precedent).
  Nine tests against provider-sql-shaped fakes in `hosted_database_tests.rs` (split out to
  stay under the 1500-line module fence), including the no-secret-value canary; the
  repo-wide invariant scans (rule 15's `approval_evidence_ref` sweep, the module-size and
  brand fences) cover the new module mechanically. **For the close note (chart RBAC): the
  in-cluster ServiceAccount needs `get` + `list` on the `databases` plural of both API
  groups — `mysql.sql.crossplane.io` and `postgresql.sql.crossplane.io` — in each admitted
  namespace.**
- 2026-08-24 — merged to main after independent review (PASS, 0 blocking, 2 minor: an unused
  import warning and the untested trailing-empty-page edge). Chart RBAC needed at deploy time:
  get+list on plural `databases` in groups mysql.sql.crossplane.io and
  postgresql.sql.crossplane.io — confirmed present on the live cluster.
