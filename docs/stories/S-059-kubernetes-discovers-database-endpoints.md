---
id: S-059
title: "Kubernetes discovers database endpoints"
pillar: Platform
status: ready
priority: 3
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
