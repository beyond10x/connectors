---
title: Google Drive
sidebar_position: 3
description: Read Google Drive file metadata, exports and the change feed, and create, update and copy file metadata under approval.
lede: One Drive instance per Google account reads files and the change feed and writes file metadata under approval; it never uploads content.
source: docs/catalog-google-drive.md, docs/catalog-google-oauth.md, adapters/catalog/providers/google-drive/connector.json, adapters/catalog/providers/google-drive/operations.json, adapters/catalog/tests/google_drive.rs
connectors: "drive"
---

# Google Drive

The `drive` connector of the provider Google is served by the catalog provider from the pinned
Drive API v3 document. The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Drive guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-drive.md)
has every parameter, paging rule and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a Google account is shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `about.get`, `files.list`, `files.get`, `files.export`, `changes.getStartPageToken`, `changes.list` | `files.create` and `files.copy`, unguarded; `files.update`, guarded: it reads the file first and writes nothing unless its `version` is the one you pinned |

The writes carry metadata only; no write uploads file content. A metadata write can still move,
trash, lock or re-share a file, so read the whole input before approving.

## Prerequisites at the provider

1. In the Google Cloud console, select or create a project and enable the Google Drive API.
2. Configure the OAuth consent screen and add the scopes this connector requests: `openid`,
   `https://www.googleapis.com/auth/drive.readonly` and
   `https://www.googleapis.com/auth/drive.file`. One instance per Google account requests every
   scope its operations need, reads and writes alike.
3. Under *Credentials*, create an OAuth client ID of type *Desktop app*, download its JSON and
   make it readable by you alone (`chmod 600`); the CLI refuses a file anyone else can read.
4. Choose the publishing status. Google expires the refresh token of an app in *Testing* seven
   days after consent, and every account that connects must be a test user. An app used only by
   its developer can be set to *In production* without Google's verification
   ([Google's exemption for personal use](https://support.google.com/cloud/answer/13464323)),
   which lifts the seven-day expiry.

One client file serves every Google connector; the
[Google OAuth guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-oauth.md)
describes it.

## Enable the connector and add an instance

```toml
[connectors.drive]
enabled = true

[connectors.drive.instances.private]
```

The instance uses Drive's default endpoint, `https://www.googleapis.com/drive/v3`, and pins
nothing: everything else comes from the declaration shipped with `connectors-catalog-provider`.

```sh
connectors instances list | jq -c '.[] | arrays | .[]'
```

```json
{"connector":"drive","instance":"private","instance_id":"drive.private","restart":"never","startup":"on-demand"}
```

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

Follow `nextPageToken` as `pageToken` until a page has none. Validation lasts 60 seconds after
connect; later, `connections revalidate` renews it without a credential.

## A first guarded write

Once per instance, create the signing key (`connectors approvals key-init --connector drive
--instance private`) and add an `[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)).
Publish a policy naming the writes:

```sh
printf '{"operations":["files.create","files.update","files.copy"]}\n' > policy.json
connectors approvals policy-set --connector drive --instance private --input-file policy.json
```

Rename a file. Read its current `version` with `files.get` and `{"fileId":"<id>","fields":"id,name,version"}`,
then pin it in `rename.json`:

```json
{"fileId": "<id>", "version": "<version>", "fields": "id,name,version", "body": {"name": "Q4 plan"}}
```

Take the connection, and the schema and revision of `files.update` from `invoke --describe`, then
prepare, issue and invoke:

```sh
connection=$(connectors --output json connections list --connector drive --instance private \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json invoke drive/private files.update --describe \
  | jq -r .result.schema)
revision=$(connectors --output json invoke drive/private files.update --describe \
  | jq -r .result.revision)
connectors --output json approvals prepare --connector drive --instance private \
  --connection "$connection" --operation files.update \
  --schema "$schema" --revision "$revision" --input-file rename.json
connectors approvals clock-check --connector drive --instance private
connectors approvals issue --connector drive --instance private \
  --connection "$connection" --operation files.update \
  --schema "$schema" --revision "$revision" --input-file rename.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-rename-1.json"
connectors invoke drive/private files.update \
  --input-file rename.json --approval-file "$HOME/private/proof-rename-1.json" \
  --idempotency-key rename-1
```

`$subject_sha256` is the prepare answer's `subject_sha256`. A file whose `version` moved since
you read it is refused before the PATCH, as `not_attempted`. The answer carries the new
`version` to pin next.

## Reconnect when the connection is Stale

When Google stops honouring the refresh token, an invoke answers `next_action:
repair_connection` and the connection turns `stale`. Reconnect with the same client file and the
same Google account:

```sh
connectors --output json connections list --connector drive --instance private \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector drive --instance private \
  --expected-revision "<revision from the list>" \
  --credential-file ~/Downloads/client_secret_1234.json
```

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/google_drive.rs`); no live
  Drive account has been read and no live file written.
- `files.update` cannot update a shared-drive file yet: its preflight read does not carry
  `supportsAllDrives`.
- The `version` pin is checked before the PATCH only; Drive offers no precondition on it.
- What `drive.file` lets a token write is Google's stated scope semantics, not observed here.
- The provider does not walk pages; a read answered `429` or a quota `403` is `rate_limited` with
  `retry_after_seconds` when the retry does not fit.
