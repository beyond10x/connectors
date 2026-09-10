---
title: What works today
slug: /introduction/status
sidebar_position: 2
---

# Specifications and implementation

The local **v0.1.0 milestone** records reviewed specifications for Kubernetes including discovery, GitLab and SQL. It is a specification baseline, with explicit implementation obligations.

## Implemented first slice

Independent Rust services support GitLab project, issue, file and exact-commit CI reads; Kubernetes resource inventory and endpoint/host discovery; and PostgreSQL schema and read-only query operations. A generic client and one-hop federation host can reach these services.

The local CLI also manages saved GitLab PAT connections through SQLite metadata,
qualified Secret Service custody and supervised adapter processes. Dedicated
GitLab sandbox acceptance remains open; Kubernetes and PostgreSQL still need
this local connection lifecycle.

## Reviewed semantics awaiting implementation

The newer governed service, managed authentication, durable attempts, approval, audit, discovery-state and retention models define behavior that the first-slice runtime does not yet implement. A generated type or a reviewed lifecycle does not make that behavior available in a running adapter.

## Further direction

Other adapters have designs and, in some cases, authored native ESS models. Their pages distinguish design coverage from runtime support. Sessions, media and other future profiles remain outside the selected three-adapter implementation scope.

This is a local documentation preview. Public distribution and deployment are not configured.
