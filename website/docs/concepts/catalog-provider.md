---
title: The catalog provider
sidebar_position: 3
description: How one engine serves ordinary HTTP APIs from pinned API documents, with writes guarded by checks declared as data.
lede: The catalog provider turns a pinned API document and a reviewed selection into operations, with no Rust per endpoint; adding an endpoint changes data, not code.
source: adapters/catalog/src/lib.rs, adapters/catalog/tests/engine.rs, adapters/catalog/tests/shipped.rs, docs/local-catalog-provider.md, contracts/catalog/v1alpha1/semantics.md
---

# The catalog provider

`connectors-catalog-provider` is one executable that serves many HTTP APIs. Three inputs decide
what it does:

1. **A bundle**, compiled from a pinned API document by `connectors-build catalog`. It carries the
   document's whole operation inventory and its SHA-256, and building it again gives the same
   bytes ([build a catalog bundle](../guides/build-a-catalog-bundle.md)).
2. **A selection set**, reviewed and shipped per provider under
   `adapters/catalog/providers/<provider>/operations.json`. Each entry exposes one source
   operation under a local id (`issues.list`, `merge_request.merge`) with its effect (read or
   write) and, for a write, an optional guard.
3. **A local configuration** naming the bundle, the API base, the authentication profile and the
   selection, with every credential kept out of it.

One engine binds each request from the selection, sends it, and classifies the answer. A read is
one bound request; a write is one POST, PUT, PATCH or DELETE under the host's approval, audit and
attempt controls.

## Writes and their guards

A write runs only after the operator's approval policy names it and a protected approval binds
its whole input. A guard is data in the selection, not code:

- before the one request it reads the provider (for a merge request: open, mergeable, at the
  pinned head, with the pinned pipeline green) and refuses unless every check holds;
- after the request it compares the answer, and a failed check leaves the outcome **uncertain**,
  never refused, because the provider may already have applied the write.

No corrective request is ever sent. A write that carries no guard, such as GitLab `issue.create`,
is sent once as approved; the same input approved and sent again creates a second issue.

## What it serves

| Provider | Source | Operations | Verified against |
|---|---|---|---|
| GitLab | pinned OpenAPI v4 document | reads across projects, repositories, pipelines and merge requests; issue and merge-request writes; the merge-request feed | a live GitLab |
| Jira Cloud | pinned platform REST v3 | issue search by JQL, one issue, comments, changelogs, a project's creatable issue types, user search | a live Jira through the API gateway (search); local fixtures (the rest) |
| Confluence Cloud | pinned REST v2 | changed pages, a space's pages, one page, comments | local fixtures |
| HubSpot CRM | pinned CRM Objects `2026-09` | records of one object type, one record | local fixtures |
| Zendesk Support | pinned Support API, redacted | tickets, users and organizations changed since a time, one of each, comments | a live account |
| Google Drive, Slides, Calendar, Gmail | Discovery documents projected to OpenAPI | reads and guarded writes; no direct message send | local fixtures |
| Runpod | pinned REST API v1 | create, list and terminate pods | a local fixture |
| Slack | Swagger 2.0 projected to OpenAPI 3.1.0 | conversation list, history and replies | a local fixture |

Each provider's page in the repository gives its configuration, authentication profile and
limits; [Adapters](../reference/adapters/catalog/index.md) links them.

## What it does not do

- It carries parameters, media types and response statuses, not request or response schemas: a
  body is passed through and validated by the provider.
- It does not serve operations that need a header or cookie parameter, a multipart or form body,
  or that answer binary content. A selection naming one is refused at load or fails at request
  time with a safe error.
- It never retries a write, and retries a read only once, when the provider's `Retry-After` fits
  the deadline.
- The catalog index service (listing and locating precompiled bundles) is specified and not
  implemented; directly configured providers need no index.
