---
title: Zendesk
sidebar_position: 9
description: Read Zendesk Support tickets, comments, users and organizations with an OAuth client or an API token, and update a ticket or add a private note or draft reply under approval.
lede: One Zendesk instance per account reads tickets, users and organizations through incremental exports and changes a ticket only under an issued approval; it declares two auth profiles, so the instance names the one it uses.
source: docs/catalog-zendesk.md, adapters/catalog/providers/zendesk/connector.json, adapters/catalog/providers/zendesk/operations.json, adapters/catalog/tests/zendesk.rs
connectors: "zendesk"
---

# Zendesk

The `zendesk` connector of the provider Zendesk is served by the catalog provider from the pinned
Zendesk Support API document. The words connector, instance, connection, auth profile and
operation are those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary);
the [full Zendesk guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-zendesk.md)
has every parameter, paging rule and refusal.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `tickets.incremental`, `ticket.show`, `ticket.comments`, `ticket.audits`, `users.incremental`, `user.show`, `organizations.incremental`, `organization.show` | `ticket.update`, `ticket.note` and `ticket.draft_reply`, each `effect: write` and run only under an issued approval. Each reads the ticket first and sends nothing unless its `updated_at` is the stamp you read, and then sends `safe_update`, so Zendesk refuses a ticket changed in between; both are refused as `service_failure` with `service_code: conflict`. `ticket.update` sets `group_id`, `tags` (replacing the whole set), `priority` or `type`; `ticket.note` adds a private note; `ticket.draft_reply` adds a private note whose first line marks it as a draft reply, and nothing reaches the requester |

The incremental exports take `start_time` in Unix seconds and return what changed since.

## Prerequisites at the provider

Use an OAuth client with the client-credentials grant (auth profile `zendesk.oauth`). In Admin
Center (*Apps and integrations*, *APIs*, *OAuth clients*), create a confidential client as the
admin or service user the reads should be attributed to, and keep its unique identifier and
secret. Zendesk is retiring API tokens: accounts created on or after 2026-07-28 cannot use them,
and every remaining token stops working on 2027-04-30. An existing API token still works under
`zendesk.basic`. The incremental exports are allowed to admins only.

## Enable the connector and add an instance

The connector has no default endpoint, and it declares two auth profiles, so the instance names
its account and its `profile`:

```toml
[connectors.zendesk]
enabled = true

[connectors.zendesk.instances.support]
endpoint = "https://your-subdomain.zendesk.com"
profile = "zendesk.oauth"
```

## Connect

Put `{"client_id": "<unique identifier>", "client_secret": "<secret>"}` in an owner-only file,
then:

```sh
connectors connections connect --connector zendesk --instance support \
  --profile zendesk.oauth --credential-file ~/private/zendesk-oauth.json
```

The provider requests short-lived access tokens with that client as it needs them; the stored
entry never changes. With an API token, the instance states `profile = "zendesk.basic"` and the
document is `{"account": "<email>/token", "token": "<API token>"}`.

## A first read

```sh
connectors invoke zendesk/support tickets.incremental \
  --input-json '{"start_time":1790000000}'
```

Send the answer's `after_cursor` as `cursor` until `end_of_stream` is `true`.

A ticket's history is `ticket.audits`: each audit's author, time, `via` channel and source, and its
events, field changes with their previous value included. Send `meta.after_cursor` as
`page[after]`. When a page carries `meta.has_more`, it decides: `false` ends the walk, `true`
continues even past an empty page and is an error without `meta.after_cursor`. Without
`meta.has_more`, stop on a page with no `meta.after_cursor` or no audits. A cursor seen before is
an error. Zendesk's API document says `has_more` is included when the input sets
`include_boundary_indicators`, so setting it is recommended. `ticket.comments` walks and ends the
same way:

```sh
connectors invoke zendesk/support ticket.audits \
  --input-json '{"ticket_id":101,"page[size]":100,"include_boundary_indicators":true}'
echo '{"ticket_id":101,"page[size]":100,"include_boundary_indicators":true,"page[after]":"<meta.after_cursor>"}' \
  | connectors invoke zendesk/support ticket.audits
```

## A first guarded write

The writes are verified with `zendesk.basic` only: the `zendesk.oauth` profile requests the
scope `read`, which does not cover a write. Name `profile = "zendesk.basic"` on the instance and
connect with an API token of an agent allowed to update the ticket.

Set up the [signing key and clock](../guides/guarded-write.md#once-per-instance), then publish this policy with `approvals policy-set`:

```sh
printf '{"operations":["ticket.update","ticket.note","ticket.draft_reply"]}\n' > policy.json
```

Read the ticket with `ticket.show` and pin its `ticket.updated_at` as `updated_stamp` in
`note.json`:

```json
{"ticket_id": 101, "body": {"ticket": {"comment": {"body": "Checked the billing history."}, "updated_stamp": "2025-09-16T10:00:00Z"}}}
```

[Prepare, issue and invoke](../guides/guarded-write.md#prepare-issue-and-invoke) `ticket.note` on `zendesk/support` with `note.json`, `--idempotency-key note-101`.

A ticket changed since the read is refused with `service_code: conflict`; read it again and prepare a new
approval, since the approval binds the whole input, the stamp included. A note sent twice is two
notes. A write whose answer is lost is `outcome_unknown` and is never sent again. The
[full guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-zendesk.md#writes)
lists each write's input and guard.

## Reconnect when the connection is Stale

A client secret or token Zendesk refuses makes the connection `stale`. Publish a new secret on the same connection. [Reconnect](../guides/reconnect-a-connection.md) `zendesk/support` with `--credential-file ~/private/zendesk-oauth.json`.

## Limits

- Verified against a local HTTPS fixture with synthetic records only
  (`adapters/catalog/tests/zendesk.rs`); nothing has run against a live Zendesk account.
- Whether Zendesk accepts the form-encoded token request, and whether per-resource scopes alone
  admit the exports, has not been checked live.
- The provider does not walk pages or bound `per_page`; [a read answered `429` follows the shared rule](../guides/first-read.md#rate-limits).
