---
title: Google Calendar
sidebar_position: 4
description: Read Google Calendar calendars and events, and create, change and delete events under approval.
lede: One Calendar instance per Google account reads calendars and events with sync tokens, and writes events under approval, each patch and delete pinned to the event's etag.
source: docs/catalog-google-calendar.md, docs/catalog-google-oauth.md, adapters/catalog/providers/google-calendar/connector.json, adapters/catalog/providers/google-calendar/operations.json, adapters/catalog/tests/google_calendar.rs
connectors: "calendar"
---

# Google Calendar

The `calendar` connector of the provider Google is served by the catalog provider from the pinned
Calendar API v3 document. Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Calendar guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-calendar.md)
has every parameter, paging rule and refusal.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `calendarList.list`, `events.list`, `events.get` | `events.insert`, unguarded; `events.patch` and `events.delete`, guarded: each reads the event first and writes nothing unless it is `confirmed` and its `etag` is the one you pinned |

An event write reaches other people: guests are invited, emailed and sent cancellations. Every
write names `sendUpdates` (`all`, `externalOnly` or `none`), which decides whom Google emails.

## Prerequisites at the provider

1. In the Google Cloud console, select or create a project and enable the Google Calendar API.
2. Configure the OAuth consent screen and add the scopes this connector requests: `openid`,
   `https://www.googleapis.com/auth/calendar.readonly` and
   `https://www.googleapis.com/auth/calendar.events`.
3. Create the OAuth client and choose the publishing status as in
   [Google OAuth setup](../guides/google-oauth-setup.md).

## Enable the connector and add an instance

```toml
[connectors.calendar]
enabled = true

[connectors.calendar.instances.private]
profile = "google.oauth"
```

An instance that names no profile is read-only: it uses `google.oauth.readonly`, requests
`https://www.googleapis.com/auth/calendar.readonly` alone and offers no event write. The `profile = "google.oauth"` above
adds the write scope; connect it with `--profile google.oauth`. A connection saved before this default is under `google.oauth` and is never rebound: keep that profile named to keep it, or `connections revoke` it and connect again. To admit only a company's accounts,
add `allowed_domains = ["example.org"]` to the table (one to eight domains): a connect, reconnect or
revalidation whose Google account has another domain, or none, is refused as `insufficient_scope`
before anything is stored.

The instance uses Calendar's default endpoint, `https://www.googleapis.com/calendar/v3`, and pins
nothing.

## Connect

```sh
connectors connections connect --connector calendar --instance private \
  --profile google.oauth --credential-file ~/Downloads/client_secret_1234.json
```

Consent in a browser on the same machine within 240 seconds. A consent granting only one of the
two Calendar scopes fails validation as insufficient scope.

## A first read

```sh
connectors invoke calendar/private events.list \
  --input-json '{"calendarId":"primary","timeMin":"2026-10-01T00:00:00Z","maxResults":50}'
```

`primary` is the signed-in user's calendar. The last page of a walk carries `nextSyncToken`;
send it as `syncToken` to read only what changed since.

## A first guarded write

Set up the [signing key and clock](../guides/guarded-write.md#once-per-instance), then publish this policy with `approvals policy-set`:

```sh
printf '{"operations":["events.insert","events.patch","events.delete"]}\n' > policy.json
```

Move an event. Read it with `events.get` and pin its `etag` in `move.json`:

```json
{"calendarId": "primary", "eventId": "<id>", "etag": "<etag from events.get>", "sendUpdates": "none",
 "body": {"start": {"dateTime": "2026-10-20T10:00:00Z"}, "end": {"dateTime": "2026-10-20T10:30:00Z"}}}
```

[Prepare, issue and invoke](../guides/guarded-write.md#prepare-issue-and-invoke) `events.patch` on `calendar/private` with `move.json`, `--idempotency-key move-1`.

The approval binds the whole input, `sendUpdates` included: a proof for `none` does not permit
the same write with `all`. An event changed since you read it is refused before the PATCH.

## Reconnect when the connection is Stale

Google no longer honours the refresh token and the connection turns `stale`. [Reconnect](../guides/reconnect-a-connection.md) `calendar/private` with `--credential-file ~/Downloads/client_secret_1234.json`.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/google_calendar.rs`); no
  live calendar has been read, no live event written and no email to a guest observed.
- The engine checks only that `sendUpdates` is present, not its value: check it before approving.
- The `etag` pin is checked before the write only; Google offers no precondition on it.
- `events.insert` carries no guard: the same input approved twice makes two events.
- The provider does not walk pages; [a read answered `429` follows the shared rule](../guides/first-read.md#rate-limits).
