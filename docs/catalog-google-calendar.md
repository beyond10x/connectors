# Google Calendar through the catalog provider

The catalog provider reads Google Calendar — the signed-in user's calendar list,
the events of a calendar and one event — from the pinned Calendar API v3
Discovery document. Nothing here is Calendar-specific code: the Discovery
document is projected into OpenAPI, the projection is compiled into a bundle, a
reviewed selection set exposes three reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work as described there; this
page covers what differs for Calendar. Authentication is the `oauth2_refresh`
profile `google.oauth`, as for [Google Drive](catalog-google-drive.md); see
[Authentication](#authentication).

## Source and bundle

The pinned source is
[`adapters/google/upstream/calendar/calendar-api.json`](../adapters/google/upstream/calendar/README.md),
SHA-256 `ae11869097c421ddd12e16b6cd4710ed7500379bbf669957b7014efb5b3a2698`,
Discovery `revision` `20260708`. It is projected under the rule table of
`crates/connectors-catalog/src/discovery.rs`, and the bundle is compiled from
the projection with the derivation recorded:

```sh
cargo run --locked -p connectors-build -- discovery \
  --source adapters/google/upstream/calendar/calendar-api.json \
  --out adapters/google/generated/calendar.openapi.json
cargo run --locked -p connectors-build -- catalog \
  --provider google-calendar \
  --source adapters/google/generated/calendar.openapi.json \
  --derived-from adapters/google/upstream/calendar/calendar-api.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile google.oauth \
  --replace
```

`--replace` is needed because the committed index already carries
`google-calendar`; without it the second command is refused with "provider is
already indexed". The first command writes `calendar.openapi.json` and its
projection record `calendar.openapi.projection.json`: all 38 methods of the
document are projected and none is excluded. The projection's one server is
`https://www.googleapis.com/calendar/v3`, and the bundle records each path below
it, so Discovery's `calendars/{calendarId}/events` is recorded as
`/calendar/v3/calendars/{calendarId}/events`. The bundle carries all 38
operations and names none unsupported. `adapters/catalog/tests/bundle_drift.rs`
regenerates the projection and its record from the pinned document and the
bundle from the projection, and refuses any of them, or the index, that a fresh
run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/google-calendar/operations.json`](../adapters/catalog/providers/google-calendar/operations.json)
exposes three reads and nothing else. Each is `effect: read`; none of the
document's writes is selected. A selection id is the Discovery method id without
its `calendar.` prefix. `adapters/catalog/tests/google_calendar.rs` pins this
exact id list and each id's Discovery id and path, so a renamed or dropped id,
or a method that moved, fails the gate. The bundle refuses at load any
`operation_id` the projection lacks.

Every list returns one page per call. The provider returns `status`, `body` and
`provenance`; `body` is Calendar's answer unchanged, and the fields that decide
the end of a walk are in it.

| id | Discovery id | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `calendarList.list` | `calendar.calendarList.list` | `GET /calendar/v3/users/me/calendarList` | `pageToken`, `maxResults` (1–250) | `nextPageToken` absent; the last page carries `nextSyncToken` | `syncToken` from the last walk's `nextSyncToken` |
| `events.list` | `calendar.events.list` | `GET /calendar/v3/calendars/{calendarId}/events` | `pageToken`, `maxResults` (1–2500) | `nextPageToken` absent; the last page carries `nextSyncToken` | `timeMin`, `timeMax`, `updatedMin`; deltas with `syncToken` |
| `events.get` | `calendar.events.get` | `GET /calendar/v3/calendars/{calendarId}/events/{eventId}` | single item | n/a | none; deltas come from `events.list` |

- **`calendarId`** is a calendar's id from `calendarList.list`, or `primary` for
  the signed-in user's own calendar.
- **Following a page.** Send the first page without `pageToken`. For the next
  page, send the previous page's `nextPageToken` as `pageToken`, with the other
  parameters unchanged. A page without `nextPageToken` is the last, and it
  carries `nextSyncToken`.
- **Time windows.** `timeMin` and `timeMax` are RFC 3339 timestamps with a time
  zone offset, such as `2026-09-21T00:00:00Z`; `timeMin` bounds an event's end
  time from below (exclusive) and `timeMax` its start time from above
  (exclusive). `updatedMin` selects events modified since a timestamp.
  `singleEvents` set to `true` expands recurring events into their instances.
- **`eventTypes`** is repeated: send an array, such as
  `["default", "focusTime"]`, and each element is sent as its own `eventTypes`
  pair in the order given. The pinned document enumerates `birthday`,
  `default`, `focusTime`, `fromGmail`, `outOfOffice` and `workingLocation`. The
  engine does not check a value against that list: another value is sent, and
  Google answers it.
- **`maxResults`.** The pinned document states a minimum of 1 for both lists
  and, in each description only, a ceiling: a page is never larger than 2500
  events for `events.list` and never larger than 250 entries for
  `calendarList.list`. The selections bound `maxResults` to 1–2500 and 1–250,
  so 0, a value over the ceiling (2501, 251) or a value that is not an integer
  is refused as `invalid_input` before any request.

Every other query parameter the projection declares for an operation is accepted
by name, including the document-wide `fields`; one it does not declare is
refused before any request.

## Sync tokens

A full walk ends on the page carrying `nextSyncToken`. Keep that token. To read
only what changed since, send it as `syncToken` to the same list with the same
other parameters, and walk the pages the same way; the last page carries the
next `nextSyncToken`. Deleted events, and for `calendarList.list` deleted or
hidden entries, are always included in a delta.

The pinned document names parameters that cannot be sent beside `syncToken`.
For `events.list` these are `iCalUID`, `orderBy`, `privateExtendedProperty`,
`q`, `sharedExtendedProperty`, `timeMin`, `timeMax` and `updatedMin`; for
`calendarList.list` they are `minAccessRole` and `showOwnOrganizationOnly`. So
a walk meant to be continued with sync tokens is a full walk without them. The
provider does not check these combinations: it sends them, and Google's answer
decides.

**Reset rule.** A sync token expires. Google then answers `410` with the reason
`fullSyncRequired`, which reaches the caller as `not_found`. On that refusal,
discard the stored token and everything derived from it, walk the list again
from the first page without `syncToken`, and keep the new `nextSyncToken`.

## Authentication

Calendar uses Google OAuth: the `oauth2_refresh` scheme under the profile
`google.oauth`, as described in
[the catalog provider guide](local-catalog-provider.md). The protected entry is
`{"client_id":"...","client_secret":"...","refresh_token":"..."}` for an
installed-app OAuth client. The provider exchanges it at `token_url` for an
access token and sends that as `Authorization: Bearer <access token>`. The
identity comes from the token answer's `id_token` (`identity.source: id_token`).
`minimum_scopes` asks for the Calendar read-only scope, which the pinned
document accepts for all three reads. `authorize_url` and `requested_scopes` are
never called by the provider; they are handed to the host for obtaining the
entry by consent.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "google-calendar",
  "provider": "google-calendar",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://www.googleapis.com/calendar/v3",
  "auth": {
    "profile": "google.oauth",
    "scheme": "oauth2_refresh",
    "header": "Authorization",
    "bearer": true,
    "label": "Google refresh token",
    "identity": {"source": "id_token", "kind": "google.user"},
    "minimum_scopes": ["https://www.googleapis.com/auth/calendar.readonly"],
    "token_url": "https://oauth2.googleapis.com/token",
    "authorize_url": "https://accounts.google.com/o/oauth2/v2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/calendar.readonly"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-calendar/operations.json"
}
```

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/google_calendar.rs`): the guide's configuration with
  the fixture as API and token host, one token exchange, the exact request of
  each read with the exchanged bearer, the returned body as JSON, a two-page
  walk of each list to the page carrying `nextSyncToken`, a delta read with
  `syncToken`, an expired token's `410` as `not_found`, `eventTypes` sent as two
  pairs, and the `maxResults` bounds of both lists. No live calendar has been
  read.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Google's exact bytes.
- The provider does not walk pages itself and does not retry. A `429`, or a
  `403` whose reason is `rateLimitExceeded` or `userRateLimitExceeded`
  (each selection names both in `rate_limit_reasons`), is returned as
  `rate_limited`; every other `403` is `forbidden`.
