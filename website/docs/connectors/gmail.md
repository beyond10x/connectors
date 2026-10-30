---
title: Gmail
sidebar_position: 2
description: Read a Gmail mailbox and send mail through drafts under approval, with one instance per Google account.
lede: One Gmail instance per Google account reads messages, threads, history and labels, and writes mail only as a draft that is read, approved and then sent by id.
source: docs/catalog-google-gmail.md, docs/catalog-google-oauth.md, adapters/catalog/providers/google-gmail/connector.json, adapters/catalog/providers/google-gmail/operations.json, adapters/catalog/tests/google_gmail.rs
connectors: "gmail"
---

# Gmail

The `gmail` connector of the provider Google is served by the catalog provider from the pinned
Gmail API v1 document. The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Gmail guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-gmail.md)
has every parameter, paging rule and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a Google account is shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `users.getProfile`, `users.messages.list`, `users.messages.get`, `users.messages.attachments.get`, `users.threads.list`, `users.threads.get`, `users.history.list`, `users.labels.list` | `users.drafts.create`; `users.drafts.send`, guarded: it reads the draft first and sends nothing unless the draft's `message.id` is the one you approved |

`users.messages.send` is not offered: mail leaves only as a draft that existed and was read.
Every list returns one page per call.

## Prerequisites at the provider

1. In the Google Cloud console, select or create a project and enable the Gmail API.
2. Configure the OAuth consent screen and add the scopes this connector requests: `openid`,
   `https://www.googleapis.com/auth/gmail.readonly` and
   `https://www.googleapis.com/auth/gmail.compose`. One instance per Google account requests every
   scope its operations need, reads and writes alike.
3. Under *Credentials*, create an OAuth client ID of type *Desktop app* and download its JSON.
   Make it readable by you alone; the CLI refuses a credential file anyone else can read:

   ```sh
   chmod 600 ~/Downloads/client_secret_*.json
   ```

4. Choose the publishing status. Google expires the refresh token of an app in *Testing* seven
   days after consent, so each connection then needs a reconnect every week, and every account
   that connects must be listed as a test user. An app used only by its developer can be set to
   *In production* without Google's verification
   ([Google's exemption for personal use](https://support.google.com/cloud/answer/13464323)),
   which lifts the seven-day expiry.

The [Google OAuth guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-oauth.md)
describes the client file and the consent flow in full.

## Enable the connector and add an instance

Enable `gmail` in a `connectors-local/5` configuration and name one instance per account. The
instance needs no endpoint (Gmail's default, `https://gmail.googleapis.com`, applies) and no
pin: everything else comes from the declaration shipped with `connectors-catalog-provider`.

```toml
[connectors.gmail]
enabled = true

[connectors.gmail.instances.private]
description = "personal mailbox"
```

```sh
connectors instances list | jq -c '.[] | arrays | .[]'
```

```json
{"connector":"gmail","instance":"private","instance_id":"gmail.private","restart":"never","startup":"on-demand"}
```

A second account is a second instance, such as `[connectors.gmail.instances.work]`, with a
consent of its own.

## Connect

Hand the client file to `connections connect`. It listens on `127.0.0.1`, prints the consent
address, and after you consent in a browser on the same machine stores the refresh token in
custody; the client file itself is not kept. Consent has 240 seconds.

```sh
connectors connections connect --connector gmail --instance private \
  --profile google.oauth --credential-file ~/Downloads/client_secret_1234.json
```

Over SSH, forward the port the consent address names. A consent granting only one of the two
Gmail scopes fails validation as insufficient scope.

## A first read

A read takes the instance's one connection and its current schema and revision. Validation lasts
60 seconds after connect; later, `connections revalidate` renews it without a credential.

```sh
connectors invoke gmail/private users.getProfile \
  --input-json '{"userId":"me"}'
```

The answer's `historyId` is the baseline for `users.history.list`. `userId` is required on every
read and is `me`.

## A first guarded write

Every write is a required-approval mutation. Once per instance, create its signing key with
`connectors approvals key-init --connector gmail --instance private` and add an
`[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)).
Then publish a policy naming the writes:

```sh
printf '{"operations":["users.drafts.create","users.drafts.send"]}\n' > policy.json
connectors approvals policy-set --connector gmail --instance private --input-file policy.json
```

Create a draft. `draft.json` holds the whole message, RFC 5322 text encoded base64url:

```json
{"userId": "me", "body": {"message": {"raw": "<base64url RFC 5322 message>"}}}
```

Take the connection, and the schema and revision of `users.drafts.create` from `invoke --describe`,
prepare the approval, read the subject it names, issue the proof and invoke with it:

```sh
connection=$(connectors --output json connections list --connector gmail --instance private \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json invoke gmail/private users.drafts.create --describe \
  | jq -r .result.schema)
revision=$(connectors --output json invoke gmail/private users.drafts.create --describe \
  | jq -r .result.revision)
connectors --output json approvals prepare --connector gmail --instance private \
  --connection "$connection" --operation users.drafts.create \
  --schema "$schema" --revision "$revision" --input-file draft.json
connectors approvals clock-check --connector gmail --instance private
connectors approvals issue --connector gmail --instance private \
  --connection "$connection" --operation users.drafts.create \
  --schema "$schema" --revision "$revision" --input-file draft.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-draft-1.json"
connectors invoke gmail/private users.drafts.create \
  --input-file draft.json --approval-file "$HOME/private/proof-draft-1.json" \
  --idempotency-key draft-1
```

`$subject_sha256` is the `subject_sha256` of the prepare answer. The approval binds the whole
input by digest, so decode `body.message.raw` and read it before approving. The answer carries the
draft's `id` and `message.id`. Send it the same way with `users.drafts.send` and this input; a
draft whose content changed since carries another `message.id`, and the send is refused with
nothing sent:

```json
{"userId": "me", "messageId": "<the draft's message.id>", "body": {"id": "<draft id>"}}
```

## Reconnect when the connection is Stale

When Google stops honouring the refresh token (revoked, password changed, or the seven-day
*Testing* expiry), an invoke answers `service_failure` with `next_action: repair_connection` and
the connection turns `stale`. No retry renews it. Reconnect with the same client file and the
same Google account:

```sh
connectors --output json connections list --connector gmail --instance private \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector gmail --instance private \
  --expected-revision "<revision from the list>" \
  --credential-file ~/Downloads/client_secret_1234.json
```

Reconnect runs the consent again and replaces the token on the same connection. Another
account's consent is refused as `identity_mismatch`, and the connection keeps its entry.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/google_gmail.rs`); no live
  mailbox has been read and no live draft created or sent.
- An answer over 4 MiB is refused as `capacity`; a part over about 3 MiB cannot be read.
- The provider does not walk pages; a read answered `429` is sent once more when time allows,
  otherwise it is `rate_limited` with `retry_after_seconds`.
- The approval input is capped at 256 KiB, which bounds the size of a draft created under approval.
- The send's preflight and its POST are two requests: a draft edited between them is sent as
  edited.
