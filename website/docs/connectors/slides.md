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
Slides API v1 document. The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Slides guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-slides.md)
has every parameter and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a Google account is shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `presentations.get`, `presentations.pages.get`, `presentations.pages.getThumbnail` | `presentations.create`, unguarded; `presentations.batchUpdate`, guarded: it reads the presentation first and sends nothing unless its `revisionId` is the one the body pins |

## Prerequisites at the provider

1. In the Google Cloud console, select or create a project and enable the Google Slides API.
2. Configure the OAuth consent screen and add the scopes this connector requests: `openid`,
   `https://www.googleapis.com/auth/presentations` and
   `https://www.googleapis.com/auth/presentations.readonly`. One instance per Google account
   requests every scope its operations need, reads and writes alike.
3. Under *Credentials*, create an OAuth client ID of type *Desktop app*, download its JSON and
   make it readable by you alone (`chmod 600`).
4. Choose the publishing status. Google expires the refresh token of an app in *Testing* seven
   days after consent, and every account that connects must be a test user. An app used only by
   its developer can be set to *In production* without Google's verification
   ([Google's exemption for personal use](https://support.google.com/cloud/answer/13464323)),
   which lifts the seven-day expiry.

## Enable the connector and add an instance

```toml
[connectors.slides]
enabled = true

[connectors.slides.instances.private]
```

The instance uses Slides' default endpoint, `https://slides.googleapis.com`, and pins nothing.

```sh
connectors instances list | jq -c '.[] | arrays | .[]'
```

```json
{"connector":"slides","instance":"private","instance_id":"slides.private","restart":"never","startup":"on-demand"}
```

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

Once per instance, create the signing key (`connectors approvals key-init --connector slides
--instance private`) and add an `[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)).
Publish a policy naming the writes:

```sh
printf '{"operations":["presentations.create","presentations.batchUpdate"]}\n' > policy.json
connectors approvals policy-set --connector slides --instance private --input-file policy.json
```

Add a slide at the revision the read above answered, in `slide.json`:

```json
{"presentationId": "<id>", "body": {"requests": [{"createSlide": {}}], "writeControl": {"requiredRevisionId": "<revisionId>"}}}
```

Take the connection, and the schema and revision of `presentations.batchUpdate`
from `invoke --describe`, then prepare, issue and invoke:

```sh
connection=$(connectors --output json connections list --connector slides --instance private \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json invoke slides/private presentations.batchUpdate --describe \
  | jq -r .result.schema)
revision=$(connectors --output json invoke slides/private presentations.batchUpdate --describe \
  | jq -r .result.revision)
connectors --output json approvals prepare --connector slides --instance private \
  --connection "$connection" --operation presentations.batchUpdate \
  --schema "$schema" --revision "$revision" --input-file slide.json
connectors approvals clock-check --connector slides --instance private
connectors approvals issue --connector slides --instance private \
  --connection "$connection" --operation presentations.batchUpdate \
  --schema "$schema" --revision "$revision" --input-file slide.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-slide-1.json"
connectors invoke slides/private presentations.batchUpdate \
  --input-file slide.json --approval-file "$HOME/private/proof-slide-1.json" \
  --idempotency-key slide-1
```

A deck edited since you read it is refused with no POST; Google checks the pinned revision again
and applies the requests all or none.

## Reconnect when the connection is Stale

When Google stops honouring the refresh token, an invoke answers `next_action:
repair_connection` and the connection turns `stale`. Reconnect with the same client file and the
same Google account:

```sh
connectors --output json connections list --connector slides --instance private \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector slides --instance private \
  --expected-revision "<revision from the list>" \
  --credential-file ~/Downloads/client_secret_1234.json
```

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
