---
title: The catalog provider
sidebar_position: 3
description: How one engine serves ordinary HTTP APIs from pinned API documents, with writes guarded by checks declared as data.
lede: The catalog provider turns a pinned API document and a reviewed selection into operations, with no Rust per endpoint; adding an endpoint changes data, not code.
source: adapters/catalog/src/lib.rs, adapters/catalog/providers/gitlab/connector.json, adapters/catalog/tests/engine.rs, adapters/catalog/tests/shipped.rs, adapters/catalog/tests/binary_responses.rs, docs/local-catalog-provider.md, contracts/catalog/v1alpha1/semantics.md
---

# The catalog provider

`connectors-catalog-provider` serves each catalog connector from a bundle compiled from its pinned
API document ([build a catalog bundle](../guides/build-a-catalog-bundle.md)) and a reviewed
selection of its operations; the connector's page under [Connectors](../connectors/index.md)
lists them.

## Writes and their guards

A write runs only after the operator's approval policy names it and a protected approval binds
its whole input. A guard is data in the selection, not code:

- before the one request it reads the provider (for a merge request: open, mergeable, at the
  pinned head, with the pinned pipeline green) and refuses unless every check holds;
- after the request it compares the answer, and a failed check leaves the outcome **uncertain**,
  never refused, because the provider may already have applied the write.

No corrective request is ever sent. A write that carries no guard, such as GitLab `issue.create`,
is sent once as approved; the same input approved and sent again creates a second issue.

Each connector's page in the repository gives its configuration, auth profiles and
limits; the [catalog reference](../reference/adapters/catalog/index.md) links them.

## What it does not do

- It carries parameters, media types and response statuses, not request or response schemas: a
  body is passed through and validated by the provider.
- It does not serve operations that need a header or cookie parameter, or a multipart or form
  body, so uploads are not served. A selection naming one is refused at load or fails at request
  time with a safe error.
- It reads binary content only where a selection declares a binary response: the bytes come back
  base64-encoded with their media type, length and SHA-256, at most 2 MiB, and a redirect is
  followed only to an origin the selection names and the connection admits.
- It never retries a write, and retries a read only once, when the provider's `Retry-After` fits
  the deadline.
