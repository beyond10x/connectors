---
title: Google Drive
sidebar_position: 3
description: Read Google Drive file metadata, uploaded file content, exports and the change feed, and create, update and copy file metadata under approval.
lede: One Drive instance per Google account reads files and the change feed and writes file metadata under approval; it never uploads content.
source: docs/catalog-google-drive.md, docs/catalog-google-oauth.md, adapters/catalog/providers/google-drive/connector.json, adapters/catalog/providers/google-drive/operations.json, adapters/catalog/tests/google_drive.rs
connectors: "drive"
---

# Google Drive

The `drive` connector of the provider Google is served by the catalog provider from the pinned
Drive API v3 document. Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Drive guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-drive.md)
has every parameter, paging rule and refusal.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `about.get`, `files.list`, `files.get`, `files.getMedia`, `files.export`, `changes.getStartPageToken`, `changes.list` | `files.create` and `files.copy`, unguarded; `files.update`, guarded: it reads the file first and writes nothing unless its `version` is the one you pinned |

`files.getMedia` reads an uploaded file's content, such as a PDF, under the read-only Drive scope.
It answers the bytes as base64 with their media type, length and SHA-256, and refuses a file over
2 MiB as `capacity`; the printed refusal carries no size, so read `size` with `files.get` to learn
it. A Google Docs, Sheets or Slides file has no bytes to download and is refused as forbidden, with
a reason telling you to read it with `files.export`.

The writes carry metadata only; no write uploads file content. A metadata write can still move,
trash, lock or re-share a file, so read the whole input before approving.

## Prerequisites at the provider

1. In the Google Cloud console, select or create a project and enable the Google Drive API.
2. Configure the OAuth consent screen and add the scopes this connector requests: `openid`,
   `https://www.googleapis.com/auth/drive.readonly` and
   `https://www.googleapis.com/auth/drive.file`.
3. Create the OAuth client and choose the publishing status as in
   [Google OAuth setup](../guides/google-oauth-setup.md).

## Enable the connector and add an instance

```toml
[connectors.drive]
enabled = true

[connectors.drive.instances.private]
profile = "google.oauth"
```

An instance that names no profile is read-only: it uses `google.oauth.readonly`, requests
`https://www.googleapis.com/auth/drive.readonly` alone and offers no file write. The `profile = "google.oauth"` above
adds the write scope; connect it with `--profile google.oauth`. A connection saved before this default is under `google.oauth` and is never rebound: keep that profile named to keep it, or `connections revoke` it and connect again. To admit only a company's accounts,
add `allowed_domains = ["example.org"]` to the table (one to eight domains): a connect, reconnect or
revalidation whose Google account has another domain, or none, is refused as `insufficient_scope`
before anything is stored.

The instance uses Drive's default endpoint, `https://www.googleapis.com/drive/v3`, and pins
nothing: everything else comes from the declaration shipped with `connectors-catalog-provider`.

## Connect

```sh
connectors connections connect --connector drive --instance private \
  --profile google.oauth --credential-file ~/Downloads/client_secret_1234.json
```

The command prints a consent address; consent in a browser on the same machine within 240
seconds. A consent granting only one of the two Drive scopes fails validation as insufficient
scope. `gmail.private` and `drive.private` are two instances and two consents.

## A first read

```sh
connectors invoke drive/private files.list \
  --input-json '{"q":"trashed = false","pageSize":10,"fields":"nextPageToken,files(id,name,mimeType)"}'
```

Follow `nextPageToken` as `pageToken` until a page has none.

Read an uploaded file's content with `files.getMedia`:

```sh
connectors invoke drive/private files.getMedia --input-json '{"fileId":"<id>"}'
```

## A first guarded write

Set up the [signing key and clock](../guides/guarded-write.md#once-per-instance), then publish this policy with `approvals policy-set`:

```sh
printf '{"operations":["files.create","files.update","files.copy"]}\n' > policy.json
```

Rename a file. Read its current `version` with `files.get` and `{"fileId":"<id>","fields":"id,name,version"}`,
then pin it in `rename.json`:

```json
{"fileId": "<id>", "version": "<version>", "fields": "id,name,version", "body": {"name": "Q4 plan"}}
```

[Prepare, issue and invoke](../guides/guarded-write.md#prepare-issue-and-invoke) `files.update` on `drive/private` with `rename.json`, `--idempotency-key rename-1`.

A file whose `version` moved since
you read it is refused before the PATCH, as `not_attempted`. The answer carries the new
`version` to pin next.

## Reconnect when the connection is Stale

Google no longer honours the refresh token and the connection turns `stale`. [Reconnect](../guides/reconnect-a-connection.md) `drive/private` with `--credential-file ~/Downloads/client_secret_1234.json`.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/google_drive.rs`); no live
  Drive account has been read and no live file written.
- `files.update` cannot update a shared-drive file yet: its preflight read does not carry
  `supportsAllDrives`.
- The `version` pin is checked before the PATCH only; Drive offers no precondition on it.
- What `drive.file` lets a token write is Google's stated scope semantics, not observed here.
- The provider does not walk pages; [a read answered `429` follows the shared rule](../guides/first-read.md#rate-limits).
