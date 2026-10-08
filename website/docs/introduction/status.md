---
title: What works today
slug: /introduction/status
sidebar_position: 2
---

# Source release v0.35.0

Version **0.35.0** specifies at ESS source format `ess/15` with the released
ESS 0.56.0 and plans with the released AEP 0.69.1; `connectors --version` answers. Like 0.12.0, it keeps connection metadata (identities, credential references,
fences, revisions and audit history) in Entity Runtime over an Eventlog SQLite
store. GitLab is served from its pinned OpenAPI document through the catalog
provider, as in 0.11.0, which retired the native GitLab adapter. The earlier **v0.1.0
milestone** remains a reviewed specification baseline for Kubernetes including
discovery, GitLab and SQL.

Since 0.28.0 the CLI's generated decoder refuses `operations invoke` business input
with a duplicate object key itself, as `cli_dynamic_input`, before the owner sees it;
exit code 2, empty stdout and no dispatch are as before.

Since 0.29.0 local metadata runs with provider-tracked capture (Entity Runtime 0.26.0 then):
a read invoke takes 2.3 s at 1,203 recorded events instead of 40.2 s. Every command still
opens and verifies the whole store, and about 8 events are recorded per read, so the cost
keeps growing with use: on 2026-10-06 an operator store of 2,800 events took 4 s per
`connections list` and 10 to 21 s per read invoke
([#101](https://github.com/beyond10x/connectors/issues/101)).

Since 0.30.0 a connection follows a configuration upgrade of its instance (a rebuilt
bundle or selection) on its next `connections revalidate`, without credential re-entry,
when its provider authority, profile and identity are unchanged. Catalog profiles can
declare an `access` read that validation makes, so a credential that identifies but
cannot read is refused as `insufficient_scope`; the Confluence guide declares one.
Zendesk's ticket export accepts `per_page`.

Since 0.31.0 the registry clock floor is recorded in Entity Runtime at most once a minute
instead of on every command: in the store-cost test a read invoke appended 4.0 clock
records among its 7 events, and now appends none. Every command still opens and
verifies the whole store, which Entity Runtime has to change
([entity-runtime#55](https://github.com/beyond10x/entity-runtime/issues/55)). The owner
uses one malloc arena and no longer copies every history on open: peak RSS of one
measured process at 601 events fell from 696 MB to 557–588 MB; the rest is Entity
Runtime's verified model
([#103](https://github.com/beyond10x/connectors/issues/103),
[entity-runtime#59](https://github.com/beyond10x/entity-runtime/issues/59)). A provider
refusal names the upstream reason (`service_reason`) when it carries no credential,
address or invisible text; a read answered `429` is retried once when its `Retry-After`
fits the deadline and otherwise names `retry_after_seconds`; an invoke on lapsed
validation evidence advises `revalidate_connection`. GitLab `issue.create` is an
approved write, `connections launch` hands one connection's protected document to a
pinned consumer, and the `datasource.feed/v1alpha1` contract family is specified with a
catalog feed engine that `operations list --family` discovers; no real provider binds a
feed yet, and a feed is not yet read through a saved connection.

Since 0.32.0 `--output json` answers carry JSON values where they carried JSON text:
`operations invoke` answers the provider result as a JSON value on every path, reads through
the owner included, and `operations describe`, the `adapters describe` descriptor and the
compatibility `describe` answer operation schemas as JSON objects. This is breaking for a
caller that decoded those fields a second time. GitLab is the first provider that binds the
`datasource.feed/v1alpha1` family (profile `gitlab-merge-requests/1`): member projects as
containers and merge requests as items, read through a saved connection with
`operations invoke` and resumed from a watermark. The binding is checked against recorded
GitLab shapes and a stand-in provider; it has not yet been read from a running GitLab. A feed
profile now declares what its provider can observe (deletions, kind, revision, visibility),
and the feed suite holds a binding to what it declares. Local metadata runs on Entity
Runtime 0.29.0 and Eventlog 0.8.0, which keep each committed record once instead of three
times; their durable open checkpoints are not enabled yet, so every command still verifies
the whole store ([#101](https://github.com/beyond10x/connectors/issues/101)). Stores this
version writes stay readable by 0.31.0. ESS is 0.55.0.

Since 0.33.0 the per-invoke metadata cost no longer grows with the store
([#101](https://github.com/beyond10x/connectors/issues/101)): a command run while an owner
runs reads the store through the owner's held handle, and a store `setup init` creates
carries Entity Runtime durable open checkpoints. In the store-cost test the median read
invoke took 229 ms at 601 recorded events, 341 ms at 1,201 and 255 ms at 6,000; 0.32.0 took
1,211 ms at 601 and failed 3 of 5 invokes at 1,201. Checkpoints are one-way: 0.32.0 and
earlier refuse a store that has them. An existing store keeps working with older releases
until its owner runs `setup checkpoints-enable --confirm one-way`. Every fresh open (a
command run without an owner, a second process, the owner when it starts, recovery) still
verifies the whole store, so a raw edit of the database file is refused by the first command
after it. Peak resident memory in the same test was 329 MB at 601 events and 560 MB at 1,201
([#103](https://github.com/beyond10x/connectors/issues/103)). Due expiries are recorded in
batches of at most 32. Entity Runtime is 0.30.2, Eventlog 0.8.1 and ESS 0.56.0.

Since 0.34.0 a connection saved under another configuration revision of its instance is
answered with the step that clears it: `revalidate_connection` when only the configuration
revision changed and the credential can follow, `create_connection` otherwise, instead of a
`lifecycle_conflict` with `retry_status` that no status change cleared. A new connection
under the configured revision is admitted under the same instance id and moves the instance
there when it publishes; a changed provider authority or profile declaration still needs a
new instance id. `connectors help describe`, `help invoke` and `help serve` print their
usage, and bash completion covers those three commands. Due expiries are recorded in batches
of at most 128. The repository plans with AEP 0.69.1.

Since 0.35.0 the HTTP host serves `POST /v1alpha2/invoke` when its service configuration names
a `state` directory. Every admitted invocation is anchored in the execution audit before
dispatch and answers `audit_ref` and `audit_status`; an admitted write records an attempt
before its one dispatch and answers `mutation` naming it, or `outcome_unknown` with that
attempt when its answer is lost. `connectors_client::Client::invoke_v1alpha2` returns the
attempt. `AttemptRecord.connection_ref` is optional, which breaks `connectors-client` callers
that read it. Retained audit observations are held in memory and lost on restart.

## Available runtime

[GitLab](/adapters/gitlab) runs through the [catalog provider](/adapters/catalog):
projects, issues, files, branches, pipelines, jobs, traces and merge requests as
reads, and merge-request create, update and a guarded merge as approved writes,
every one bound from the pinned source with no adapter code per endpoint. A
live GitLab has answered all of them. Since 0.16.0 the GitLab selection also lists
projects, tags, releases and project events, and bounds `per_page` to 1–100. Since 0.22.0
it also lists a ref's commits and compares two refs, and since 0.23.0 a project's
deployments with their environment. The 2026-10-02 dedicated sandbox replay
returned HTTP 200 for all eighteen current reads, each within its requested page
bounds, and reused saved credentials across an owner restart. Tags, releases and
deployments returned empty lists. An initial revalidation refusal remains
unexplained; later explicit retries succeeded. The replay made no provider writes;
the live mutation evidence above comes from earlier runs. Since 0.32.0 GitLab also binds the feed family as data: `feed.containers` lists the projects the token's user is a member of and `feed.items` reads a project's merge requests, every project listed `private` ([GitLab feed profile](https://github.com/beyond10x/connectors/blob/main/adapters/catalog/contracts/feed/v1alpha1/gitlab.md)).

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
[Zendesk Support](https://github.com/beyond10x/connectors/blob/main/docs/catalog-zendesk.md)
runs since 0.26.0 from its pinned Support API document, redacted of example
credentials and contact data: tickets, users and organizations changed since a
`start_time`, one of each by id, and a ticket's comments, seven reads and no writes.
Lists return one page per call; the caller walks pages. Since 0.29.0 it authenticates
with an OAuth client's credentials (`zendesk.oauth`, scheme `oauth2_client_credentials`),
which needs no refresh; the API token as HTTP basic (`zendesk.basic`) remains until
Zendesk retires API tokens on 2027-04-30. On 2026-10-05 `zendesk.oauth` connected to a
live account and the incremental exports, `user.show` and the reads a ticket triage
makes answered 200.

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

[Tavily](/adapters/tavily) runs as a native adapter since 0.27.0, binding the shared
[websearch family](/contracts/data/websearch): `websearch.search`, `websearch.fetch`
and `websearch.crawl`. Each is a read Tavily serves as a POST, which the host admits
for the adapter's three fixed paths through `post_json`, with no write approval.
Search and crawl have answered live through the local CLI; fetch has been exercised
against a local fake server only.

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
effects, including lost-response observation without resending. The catalog port
now has passing evidence for all ten logical obligations and fifteen historical
variants. Exact-file seccomp faults exercise settlement failure; cleanup controls
distinguish strongly owned processes from observation-only child handles. The
original effects use the production CLI; some settled replay observations use a
same-image host-library helper. Results span multiple runs with documented reuse
where relevant source is unchanged, rather than one final executable running all
variants. The fault cases require Linux x86_64 seccomp user notifications.
Public/version-one interfaces retain reads.

Kubernetes and PostgreSQL implement persistent local connections, saved credential
reuse and owner restarts. Four new k3s v1.31.5 cases passed for selected reads,
RBAC denial versus empty results, continuation binding and lifecycle controls;
four existing CLI journeys also passed. Six PostgreSQL 17.6 cases passed: five
production CLI cases and one direct adapter invocation-drop cancellation case.
The PostgreSQL fixture used plaintext loopback, and its cancellation timing is
an observation rather than a universal deadline. SSAR and remaining Kubernetes/
Helm workflows are still open. MCP delivery remains pending: 0.26.0 adds MCP
invocation, projection, auth lifecycle, mutation replay and composition
contracts, with no MCP runtime. Metadata operation cost still grows with the event store; Entity Runtime issue 51 tracks the upstream
fix, and the timeout cleanup change does not remove that performance limit.
Other adapter pages distinguish designs and typed specifications from running
implementations; a release does not imply completion of the full provider plan.

This is a source release for Linux x86_64. The local runtime requires Rust 1.91;
the independent libraries are checked on Rust 1.88. Existing installed
configuration and credentials are not automatically migrated.
Binary/package distribution and website/cloud deployment are separate.
