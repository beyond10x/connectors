---
title: What works today
slug: /introduction/status
sidebar_position: 2
---

# Source release v0.11.0

Version **0.11.0** serves GitLab from its pinned OpenAPI document through the
catalog provider and retires the native GitLab adapter. The earlier **v0.1.0
milestone** remains a reviewed specification baseline for Kubernetes including
discovery, GitLab and SQL.

## Available runtime

[GitLab](/adapters/gitlab) runs through the [catalog provider](/adapters/catalog):
projects, issues, files, branches, pipelines, jobs, traces and merge requests as
reads, and merge-request create, update and a guarded merge as approved writes,
every one bound from the pinned source with no adapter code per endpoint. A
live GitLab has answered all of them.

Local setup, adapter supervision and connection management use SQLite metadata
and qualified Secret Service custody. Saved PATs can be reused after restarts,
repaired, revalidated and locally revoked. Approval-signing keys can be initialized,
rotated, recovered, revoked and retired through the CLI.

Kubernetes resource inventory and endpoint/host discovery, and PostgreSQL schema
and read-only query operations, remain available through standalone services.
The generic client and one-hop federation host retain their existing interfaces.

## Verification and remaining work

The catalog provider has engine fixtures for binding, guards and text
responses, host-child fixtures over disposable HTTPS, a drift check on the
committed bundle, and the repository gate including Rust 1.88. Its GitLab
sandbox acceptance is recorded in the source checkout's evidence directory.

The development checkout extends this release with local approval policy,
protected proof issuance and a guarded GitLab merge through private protocol two.
Disposable CLI fixtures join the mutation ledger, audit, approval spend and one
native PUT, including lost-response observation after owner shutdown without
resending. Public/version-one interfaces retain reads. The wider crash/failure
matrix and dedicated sandbox acceptance remain unfinished.

Kubernetes and PostgreSQL still need the persistent local connection lifecycle.
Their remaining selected workflows precede MCP and the remaining providers.
Other adapter pages distinguish designs and typed specifications from running
implementations; a release does not imply completion of the full provider plan.

This is a source release for Linux x86_64, built with Rust 1.88 or later. Existing
installed configuration and credentials are not automatically migrated.
Binary/package distribution and website/cloud deployment are separate.
