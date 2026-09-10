---
title: What works today
slug: /introduction/status
sidebar_position: 2
---

# Source release v0.2.0

Version **0.2.0** delivers the persistent Linux GitLab CLI and eleven native read
operations. The earlier **v0.1.0 milestone** remains a reviewed specification
baseline for Kubernetes including discovery, GitLab and SQL.

## Available runtime

The [GitLab adapter](/adapters/gitlab) reads projects, issues, files, exact-commit
pipelines/jobs, bounded traces and merge requests. Pinned-head validation checks
the selected MR SHA and successful head pipeline and reports blockers; it never
performs or authorizes a merge.

Local setup, adapter supervision and connection management use SQLite metadata
and qualified Secret Service custody. Saved PATs can be reused after restarts,
repaired, revalidated and locally revoked. Approval-signing keys can be initialized,
rotated, recovered, revoked and retired through the CLI.

Kubernetes resource inventory and endpoint/host discovery, and PostgreSQL schema
and read-only query operations, remain available through standalone services.
The generic client and one-hop federation host retain their existing interfaces.

## Verification and remaining work

The GitLab increment has six production CLI journeys using disposable HTTPS and
keyring fixtures, native MR tests, local packaging checks, and the repository
gate including Rust 1.88. Dedicated GitLab sandbox acceptance is still open.

Private mutation-ledger, audit and approval proof/spend ports have executable
failure and restart tests. Public approval issuance, qualified production approval
time and the complete write-dispatch binding remain unfinished. These private
ports do not make governed GitLab writes available.

Kubernetes and PostgreSQL still need the persistent local connection lifecycle.
Their remaining selected workflows precede MCP and the remaining providers.
Other adapter pages distinguish designs and typed specifications from running
implementations; a release does not imply completion of the full provider plan.

This is a source release for Linux x86_64, built with Rust 1.88 or later. Existing
installed configuration and credentials are not automatically migrated.
Binary/package distribution and website/cloud deployment are separate.
