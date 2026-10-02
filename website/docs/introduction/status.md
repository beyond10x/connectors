---
title: What works today
slug: /introduction/status
sidebar_position: 2
---

# Source release v0.25.0

Version **0.25.0** specifies at ESS source format `ess/15` with the released
ESS 0.45.0 and plans with the released AEP 0.65.0; `connectors --version` answers. Like 0.12.0, it keeps connection metadata (identities, credential references,
fences, revisions and audit history) in Entity Runtime over an Eventlog SQLite
store. GitLab is served from its pinned OpenAPI document through the catalog
provider, as in 0.11.0, which retired the native GitLab adapter. The earlier **v0.1.0
milestone** remains a reviewed specification baseline for Kubernetes including
discovery, GitLab and SQL.

## Available runtime

[GitLab](/adapters/gitlab) runs through the [catalog provider](/adapters/catalog):
projects, issues, files, branches, pipelines, jobs, traces and merge requests as
reads, and merge-request create, update and a guarded merge as approved writes,
every one bound from the pinned source with no adapter code per endpoint. A
live GitLab has answered all of them. Since 0.16.0 the GitLab selection also lists
projects, tags, releases and project events, and bounds `per_page` to 1–100. Since 0.22.0
it also lists a ref's commits and compares two refs, and since 0.23.0 a project's
deployments with their environment, exercised against local fixtures only.

[Jira Cloud](https://github.com/beyond10x/connectors/blob/main/docs/catalog-jira.md) runs
through the same catalog provider with HTTP basic authentication: issue search by
JQL, issue comments and issue changelogs, as reads from the pinned platform REST v3
document. [Confluence Cloud](https://github.com/beyond10x/connectors/blob/main/docs/catalog-confluence.md)
runs the same way from its pinned REST v2 document: pages changed since a cutoff,
a space's pages, one page with its body and a page's comments. Both share one
Atlassian auth profile, `atlassian.basic`, so one account and API token serve both.
Jira has answered live through the Atlassian API gateway (`request_prefix`, since 0.20.0) with a
service-account token; Confluence has been exercised against local fixtures only.
[HubSpot CRM](https://github.com/beyond10x/connectors/blob/main/docs/catalog-hubspot.md)
runs from its pinned CRM Objects `2026-09` document with a private-app access token:
one object type's records, paged, and one record by id, for contacts, companies,
deals and every other object type. It has no updated-since filter yet and has been
exercised against local fixtures only.

[Google](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-oauth.md)
Drive, Slides, Calendar and Gmail run through the same catalog provider since 0.21.0,
from Google Discovery documents projected to OpenAPI 3.0 by `connectors-build discovery`.
Authentication is the `oauth2_refresh` scheme: `connections connect` with a Google
Desktop OAuth client file runs browser consent and stores the refresh token. Reads
cover Drive files, export and changes, Slides presentations and pages, Calendar
lists and events, and Gmail messages, threads, history and labels. Guarded writes
cover Drive metadata, Slides `batchUpdate`, Calendar events and Gmail drafts, with
no direct `messages.send`. All four have been exercised against local fixtures only,
not live Google.

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

Local approval policy, protected proof issuance and guarded GitLab writes run
through private protocol two. The retired native GitLab adapter had production
CLI fixtures joining the mutation ledger, audit, approval spend and provider
effects, including lost-response observation without resending. Porting those
journeys to the catalog child remains required; engine fixtures and earlier live
sandbox runs do not replace that evidence. Public/version-one interfaces retain
reads.

Kubernetes and PostgreSQL implement persistent local connections, saved credential
reuse and owner restarts. Their dedicated sandbox acceptance and remaining
selected workflows precede MCP and the remaining providers. Metadata operation
cost still grows with the event store; Entity Runtime issue 51 tracks the upstream
fix, and the timeout cleanup change does not remove that performance limit.
Other adapter pages distinguish designs and typed specifications from running
implementations; a release does not imply completion of the full provider plan.

This is a source release for Linux x86_64. The local runtime requires Rust 1.91;
the independent libraries are checked on Rust 1.88. Existing installed
configuration and credentials are not automatically migrated.
Binary/package distribution and website/cloud deployment are separate.
