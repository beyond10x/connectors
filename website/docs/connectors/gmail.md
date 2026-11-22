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
Gmail API v1 document. Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Gmail guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-gmail.md)
has every parameter, paging rule and refusal.

[How these commands were run](../guides/first-read.md#examples).

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
   `https://www.googleapis.com/auth/gmail.compose`.
3. Create the OAuth client and choose the publishing status as in
   [Google OAuth setup](../guides/google-oauth-setup.md).

## Enable the connector and add an instance

Enable `gmail` in a `connectors-local/5` configuration and name one instance per account. The
instance needs no endpoint (Gmail's default, `https://gmail.googleapis.com`, applies) and no
pin: everything else comes from the declaration shipped with `connectors-catalog-provider`.

```toml
[connectors.gmail]
enabled = true

[connectors.gmail.instances.private]
description = "personal mailbox"
profile = "google.oauth"
```

An instance that names no profile is read-only: it uses `google.oauth.readonly`, requests
`https://www.googleapis.com/auth/gmail.readonly` alone and offers no draft write. The `profile = "google.oauth"` above
adds the write scope; connect it with `--profile google.oauth`. A connection saved before this default is under `google.oauth` and is never rebound: keep that profile named to keep it, or `connections revoke` it and connect again. To admit only a company's accounts,
add `allowed_domains = ["example.org"]` to the table (one to eight domains): a connect, reconnect or
revalidation whose Google account has another domain, or none, is refused as `insufficient_scope`
before anything is stored.

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
Gmail scopes fails validation as insufficient scope. A read-only instance asks for the
`gmail.readonly` scope alone.

## A first read

```sh
connectors invoke gmail/private users.getProfile \
  --input-json '{"userId":"me"}'
```

The answer's `historyId` is the baseline for `users.history.list`. `userId` is required on every
read and is `me`.

## A first guarded write

Set up the [signing key and clock](../guides/guarded-write.md#once-per-instance), then publish this policy with `approvals policy-set`:

```sh
printf '{"operations":["users.drafts.create","users.drafts.send"]}\n' > policy.json
```

Create a draft. `draft.json` holds the whole message, RFC 5322 text encoded base64url:

```json
{"userId": "me", "body": {"message": {"raw": "<base64url RFC 5322 message>"}}}
```

[Prepare, issue and invoke](../guides/guarded-write.md#prepare-issue-and-invoke) `users.drafts.create` on `gmail/private` with `draft.json`, `--idempotency-key draft-1`.

The approval binds the whole
input by digest, so decode `body.message.raw` and read it before approving. The answer carries the
draft's `id` and `message.id`. Send it the same way with `users.drafts.send` and this input; a
draft whose content changed since carries another `message.id`, and the send is refused with
nothing sent:

```json
{"userId": "me", "messageId": "<the draft's message.id>", "body": {"id": "<draft id>"}}
```

## Reconnect when the connection is Stale

When Google stops honouring the refresh token (revoked, password changed, or the seven-day
*Testing* expiry), the connection turns `stale`. [Reconnect](../guides/reconnect-a-connection.md) `gmail/private` with `--credential-file ~/Downloads/client_secret_1234.json`.

Reconnect runs the consent again and replaces the token on the same connection. Another
account's consent is refused as `identity_mismatch`, and the connection keeps its entry.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/google_gmail.rs`); no live
  mailbox has been read and no live draft created or sent.
- An answer over 4 MiB is refused as `capacity`; a part over about 3 MiB cannot be read.
- The provider does not walk pages; [a read answered `429` follows the shared rule](../guides/first-read.md#rate-limits).
- The approval input is capped at 256 KiB, which bounds the size of a draft created under approval.
- The send's preflight and its POST are two requests: a draft edited between them is sent as
  edited.
