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
Calendar API v3 document. The words connector, instance, connection and operation are those of
the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Calendar guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-calendar.md)
has every parameter, paging rule and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a Google account is shown without output.

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
   `https://www.googleapis.com/auth/calendar.events`. One instance per Google account requests
   every scope its operations need, reads and writes alike.
3. Under *Credentials*, create an OAuth client ID of type *Desktop app*, download its JSON and
   make it readable by you alone (`chmod 600`).
4. Choose the publishing status. Google expires the refresh token of an app in *Testing* seven
   days after consent, and every account that connects must be a test user. An app used only by
   its developer can be set to *In production* without Google's verification
   ([Google's exemption for personal use](https://support.google.com/cloud/answer/13464323)),
   which lifts the seven-day expiry.

## Enable the connector and add an instance

```toml
[connectors.calendar]
enabled = true

[connectors.calendar.instances.private]
```

The instance uses Calendar's default endpoint, `https://www.googleapis.com/calendar/v3`, and pins
nothing.

```sh
connectors instances list
```

```json
{"adapters":[{"adapter":"calendar.private","adapter_id":"catalog","connector":"calendar","instance":"private","instance_id":"calendar.private","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

## Connect

```sh
connectors connections connect --connector calendar --instance private \
  --profile google.oauth --credential-file ~/Downloads/client_secret_1234.json
```

Consent in a browser on the same machine within 240 seconds. A consent granting only one of the
two Calendar scopes fails validation as insufficient scope.

## A first read

```sh
connection=$(connectors --output json connections list --connector calendar --instance private \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector calendar --instance private \
  --operation events.list | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector calendar --instance private \
  --operation events.list | jq -r .result.revision)
connectors operations invoke --connector calendar --instance private --connection "$connection" \
  --operation events.list --schema "$schema" --revision "$revision" \
  --input-json '{"calendarId":"primary","timeMin":"2026-10-01T00:00:00Z","maxResults":50}'
```

`primary` is the signed-in user's calendar. The last page of a walk carries `nextSyncToken`;
send it as `syncToken` to read only what changed since.

## A first guarded write

Once per instance, create the signing key (`connectors approvals key-init --connector calendar
--instance private`) and add an `[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)).
Publish a policy naming the writes:

```sh
printf '{"operations":["events.insert","events.patch","events.delete"]}\n' > policy.json
connectors approvals policy-set --connector calendar --instance private --input-file policy.json
```

Move an event. Read it with `events.get` and pin its `etag` in `move.json`:

```json
{"calendarId": "primary", "eventId": "<id>", "etag": "<etag from events.get>", "sendUpdates": "none",
 "body": {"start": {"dateTime": "2026-10-20T10:00:00Z"}, "end": {"dateTime": "2026-10-20T10:30:00Z"}}}
```

Take the schema and revision of `events.patch` from `operations describe`, then prepare, issue
and invoke:

```sh
connectors --output json approvals prepare --connector calendar --instance private \
  --connection "$connection" --operation events.patch \
  --schema "$schema" --revision "$revision" --input-file move.json
connectors approvals clock-check --connector calendar --instance private
connectors approvals issue --connector calendar --instance private \
  --connection "$connection" --operation events.patch \
  --schema "$schema" --revision "$revision" --input-file move.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-move-1.json"
connectors operations invoke --connector calendar --instance private --connection "$connection" \
  --operation events.patch --schema "$schema" --revision "$revision" \
  --input-file move.json --approval-file "$HOME/private/proof-move-1.json" \
  --idempotency-key move-1
```

The approval binds the whole input, `sendUpdates` included: a proof for `none` does not permit
the same write with `all`. An event changed since you read it is refused before the PATCH.

## Reconnect when the connection is Stale

When Google stops honouring the refresh token, an invoke answers `next_action:
repair_connection` and the connection turns `stale`. Reconnect with the same client file and the
same Google account:

```sh
connectors --output json connections list --connector calendar --instance private \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector calendar --instance private \
  --expected-revision "<revision from the list>" \
  --credential-file ~/Downloads/client_secret_1234.json
```

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/google_calendar.rs`); no
  live calendar has been read, no live event written and no email to a guest observed.
- The engine checks only that `sendUpdates` is present, not its value: check it before approving.
- The `etag` pin is checked before the write only; Google offers no precondition on it.
- `events.insert` carries no guard: the same input approved twice makes two events.
- The provider does not walk pages; a read answered `429` or a quota `403` is `rate_limited` with
  `retry_after_seconds` when the retry does not fit.
