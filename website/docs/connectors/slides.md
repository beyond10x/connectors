---
title: Google Slides
sidebar_position: 5
description: Read Google Slides presentations and pages, and create and update presentations under approval.
lede: One Slides instance per Google account reads presentations and pages, and updates a deck only at the revision you read and approved.
source: docs/catalog-google-slides.md, docs/catalog-google-oauth.md, adapters/catalog/providers/google-slides/connector.json, adapters/catalog/providers/google-slides/operations.json, adapters/catalog/tests/google_slides.rs
connectors: "slides"
---

# Google Slides

The `slides` connector of the provider Google is served by the catalog provider from the pinned
Slides API v1 document. Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Slides guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-slides.md)
has every parameter and refusal.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `presentations.get`, `presentations.pages.get`, `presentations.pages.getThumbnail` | `presentations.create`, unguarded; `presentations.batchUpdate`, guarded: it reads the presentation first and sends nothing unless its `revisionId` is the one the body pins |

## Prerequisites at the provider

1. In the Google Cloud console, select or create a project and enable the Google Slides API.
2. Configure the OAuth consent screen and add the scopes this connector requests: `openid`,
   `https://www.googleapis.com/auth/presentations` and
   `https://www.googleapis.com/auth/presentations.readonly`.
3. Create the OAuth client and choose the publishing status as in
   [Google OAuth setup](../guides/google-oauth-setup.md).

## Enable the connector and add an instance

```toml
[connectors.slides]
enabled = true

[connectors.slides.instances.private]
```

The instance uses Slides' default endpoint, `https://slides.googleapis.com`, and pins nothing.

## Connect

```sh
connectors connections connect --connector slides --instance private \
  --profile google.oauth --credential-file ~/Downloads/client_secret_1234.json
```

Consent in a browser on the same machine within 240 seconds. A consent without the
`presentations` scope fails validation as insufficient scope.

## A first read

```sh
connectors invoke slides/private presentations.get \
  --input-json '{"presentationId":"<id>","fields":"revisionId,slides(objectId)"}'
```

`fields` narrows the answer, which matters because any answer over 4 MiB is refused.

## A first guarded write

Set up the [signing key and clock](../guides/guarded-write.md#once-per-instance), then publish this policy with `approvals policy-set`:

```sh
printf '{"operations":["presentations.create","presentations.batchUpdate"]}\n' > policy.json
```

Add a slide at the revision the read above answered, in `slide.json`:

```json
{"presentationId": "<id>", "body": {"requests": [{"createSlide": {}}], "writeControl": {"requiredRevisionId": "<revisionId>"}}}
```

[Prepare, issue and invoke](../guides/guarded-write.md#prepare-issue-and-invoke) `presentations.batchUpdate` on `slides/private` with `slide.json`, `--idempotency-key slide-1`.

A deck edited since you read it is refused with no POST; Google checks the pinned revision again
and applies the requests all or none.

## Reconnect when the connection is Stale

Google no longer honours the refresh token and the connection turns `stale`. [Reconnect](../guides/reconnect-a-connection.md) `slides/private` with `--credential-file ~/Downloads/client_secret_1234.json`.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/google_slides.rs`); no live
  presentation has been read, created or changed.
- The preflight reads the whole presentation: a deck whose full answer exceeds 4 MiB cannot be
  updated through this connector.
- A view-only account cannot update: its preflight gets no `revisionId` and fails as
  `upstream_protocol`.
- An image URL in a request is published with the image; never use a thumbnail `contentUrl` as
  one.
- Comment requests are Developer Preview and can fail inside a `200`; read
  `commentUpdateState` in the answer.
