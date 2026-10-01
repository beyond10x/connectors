# Google Calendar through the catalog provider

The catalog provider reads Google Calendar — the signed-in user's calendar list,
the events of a calendar and one event — and creates, changes and deletes
events, from the pinned Calendar API v3 Discovery document. Nothing here is
Calendar-specific code: the Discovery document is projected into OpenAPI, the
projection is compiled into a bundle, a reviewed selection set exposes three
reads and three guarded writes, and the engine described in
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
exposes three reads, each `effect: read`, and three event writes, each
`effect: write` (see [Writes](#writes)); no other method of the document is
selected. A selection id is the Discovery method id without its `calendar.`
prefix. `adapters/catalog/tests/google_calendar.rs` pins this exact id list,
each id's Discovery id, method and path, and each write's guard, so a renamed or
dropped id, a method that moved or a changed guard fails the gate. The bundle
refuses at load any `operation_id` the projection lacks.

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
- **`fields` and the end conditions.** `fields` selects the parts of the answer
  Google returns, and a token it leaves out is not returned. The end conditions
  above hold only when `fields`, if given, includes both `nextPageToken` and
  `nextSyncToken`. Without them the first page looks like the last, and a walk
  ends with no sync token. A walk meant for sync tokens also keeps the fields
  that mark a removal in a delta: `status` for events (a deleted event is
  `cancelled` and may carry only its `id`), and `deleted` and `hidden` for
  calendar list entries. For example
  `nextPageToken,nextSyncToken,items(id,status,summary,start,end)` for
  `events.list`, and `nextPageToken,nextSyncToken,items(id,summary,deleted,hidden)`
  for `calendarList.list`.
- **`maxAttendees`** on `events.list` and `events.get` has a minimum of 1 in the
  pinned document and no maximum. A selection bound needs a maximum, so none is
  declared: `maxAttendees` 0 is sent as given, and Google answers `400`
  (`invalid_input`).

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
a walk meant to be continued with sync tokens is a full walk without them: a
walk windowed by `timeMin`, `timeMax` or `updatedMin` cannot be continued by a
delta, and a delta repeats every other parameter of the full walk. The pinned
document also does not allow `showDeleted` set to `false` beside `syncToken`,
nor, for `calendarList.list`, `showHidden` set to `false`; leave both unset (or
`true`) on a walk meant for sync tokens, since its delta repeats them. The
provider does not check these combinations: it sends them, and Google's answer
decides.

**Reset rule.** A sync token expires. Google then answers `410` with the reason
`fullSyncRequired`, which reaches the caller as `not_found`. A `calendarId`
that does not exist or is no longer shared is answered `404`, which reaches the
caller as the same `not_found` with the same message. So a `not_found` on an
`events.list` sync-token request means either the sync token expired or the calendar is gone.
On that refusal, walk the list again from the first page without `syncToken`.
If that walk also answers `not_found`, the calendar is gone: drop its mirror.
Otherwise discard the stored token and everything derived from it, rebuild the
mirror from that walk, and keep its new `nextSyncToken`. `calendarList.list` is
no test of a gone calendar: it never lists `primary` by that id, and it leaves
hidden calendars out unless `showHidden` is `true`.

## Writes

Three writes create, change and delete events. An event write reaches people
other than the caller: guests are invited, emailed about changes and sent
cancellations, and a body can give them rights over the event. Each is a
required-approval mutation: it runs only through the host's approval, audit and
mutation coordinator, needs `private_protocol = "connectors-private/2"` and an
approval policy naming it, and is prepared, approved and invoked as
[the guarded merge guide](local-gitlab-merge.md) describes. Without a proof the
write is refused before the provider is asked anything.

The approval binds the whole input by its digest: the subject's `input_sha256`
is the SHA-256 of the canonical input — `calendarId`, `eventId`, the pinned
`etag`, `sendUpdates` and every other query parameter, and every `body`
member — so a proof issued for one input is refused for any other. A proof for
an input with `sendUpdates: none` does not permit the same write with
`sendUpdates: all`. The subject shows only that digest; the presentation
`approvals prepare` returns names the instance, operation, connection and
descriptor revision, not the input
(`crates/connectors-host/src/local/owner/approval_issuance.rs`). Read the input
file itself before approving — the whole input, not only the members named
below. Writes run on a separate write instance with the write scope; see
[The write scope](#the-write-scope).

| id | Discovery id | request | guard | beyond the event's own fields |
|---|---|---|---|---|
| `events.insert` | `calendar.events.insert` | `POST /calendar/v3/calendars/{calendarId}/events` with an Event `body` | none; the approval binds the whole body | every guest in `body.attendees` is invited; `body.guestsCanModify` lets guests change the event; `body.guestsCanInviteOthers` and `body.guestsCanSeeOtherGuests` default to true; `body.visibility` `public` shows its details to every reader of the calendar; with `conferenceDataVersion` 1, `body.conferenceData.createRequest` creates a new conference; `body.recurrence` makes a series; an `autoDeclineMode` other than `declineNone` in `body.outOfOfficeProperties` or `body.focusTimeProperties` declines conflicting invitations |
| `events.patch` | `calendar.events.patch` | `PATCH /calendar/v3/calendars/{calendarId}/events/{eventId}` with a partial Event `body` | preflight `events.get`: `/status` is `confirmed` and `/etag` equals the input `etag` | a given field replaces the stored one, so `body.attendees` replaces the whole guest list, removing a guest left out and inviting one added; the guest rights, `body.visibility`, `body.conferenceData` and `body.recurrence` as for insert; the id of a recurring event changes the whole series |
| `events.delete` | `calendar.events.delete` | `DELETE /calendar/v3/calendars/{calendarId}/events/{eventId}` | preflight `events.get`: `/status` is `confirmed` and `/etag` equals the input `etag` | the id of a recurring event deletes the whole series; guests may be sent a cancellation |

- **`sendUpdates`** decides whom Google emails about the write: `all`,
  `externalOnly` (guests not on Google Calendar) or `none`. Name it in every
  write input. Each write's selection requires it, so an input without it is
  refused as `invalid_input` before any request. The engine checks only that it
  is present: the bundle records no list of allowed values, so any string or
  integer, such as `""`, `NONE` or `0`, is sent to Google as given, and what
  Google does with it is not documented. The issuer checks that it is one of
  `all`, `externalOnly` or `none` before approving. The pinned document gives
  no default for patch and delete; for insert it says the default is false and that some emails might
  still be sent, and it warns that `none` "can have significant adverse
  effects, including events not syncing to external calendars or events being
  lost altogether for some users". The deprecated `sendNotifications` is
  accepted too, and nothing refuses an input that names both: both are sent,
  and the pinned document does not say which one Google obeys when they
  disagree. Do not send both.
- **`events.insert` carries no guard.** Nothing exists to compare before a
  create, and a guard compares a value read before the write for equality only.
  The approval, bound to the whole body, is its only check. The same input
  approved and sent again makes a second event. A supplied `body.id` must be
  unique per calendar, but the pinned document says Google cannot guarantee
  that a collision is detected when the event is created, so an id does not
  make a repeated create safe.
- **`events.patch` and `events.delete` are pinned to `etag`.** Their input is
  `calendarId`, `eventId`, `etag` — the event's current `etag`, from
  `events.get` or an `events.list` item — `sendUpdates`, and for a patch the
  `body`:

  ```json
  {"calendarId": "primary", "eventId": "<id>", "etag": "\"<etag>\"", "sendUpdates": "none",
   "body": {"summary": "Moved review", "start": {"dateTime": "2026-10-06T10:00:00+02:00"}, "end": {"dateTime": "2026-10-06T11:00:00+02:00"}}}
  ```

  Google's event ETags are quoted strings, and the quotes are part of the
  value: pin it exactly as read. The `etag` is compared, never sent to Google.
  Before the write the provider sends one `events.get` for `calendarId` and
  `eventId`, and refuses before any write unless the answer's `status` is
  `confirmed` and its `etag` equals the input's; the attempt is then
  `not_attempted`. An input without `etag`, or with an object, an array or
  `null` as its `etag`, is refused as `invalid_input` before any request.
- **Only confirmed events are written.** The pinned document says a deleted
  event keeps being answered with `status` `cancelled` ("The get method always
  returns them"), and `events.list` returns such events on an incremental sync
  or with `showDeleted`, each with an `etag` a caller could pin. The `status`
  check refuses a patch or delete of a cancelled event with no write sent. It
  refuses a `tentative` event too: an event whose `status` is `tentative`
  cannot be changed or deleted through these selections, and a cancelled one
  cannot be restored by a patch. A preflight Google answers `404` or `410` is
  refused with no write sent as well.
- **Patch semantics.** The pinned document says `events.patch` "supports patch
  semantics": the fields given in `body` change and the others stay. Google's
  [performance guide](https://developers.google.com/workspace/calendar/api/guides/performance)
  documents that an array given in a patch replaces the stored array, so
  `body.attendees` must list every guest who should remain. That is Google's
  stated behaviour, not verified here against a live calendar.
- **A truncated guest list.** `events.get`, `events.list`, `events.insert` and
  `events.patch` take `maxAttendees`; the pinned document says that when an
  event has more attendees than that, "only the participant is returned", and
  the answer sets `attendeesOmitted`. So an insert or patch sent with
  `maxAttendees` answers a truncated guest list as well: pin the next `etag`
  from that answer if you like, but do not build the next patch's
  `body.attendees` from it. A `body` copied from a truncated read lists only
  the caller and carries `attendeesOmitted: true`, which the pinned document
  says, for an update, "can be used to only update the participant's
  response". What Google does with such a patch — with that flag, or with a
  truncated list and without it — has not been verified here, and the guard
  does not look at it: the preflight reads the event in full and compares only
  `status` and `etag`. Before patching `attendees`, read the event without
  `maxAttendees`, check that `attendeesOmitted` is not `true`, and send every
  guest who should remain.
- **Recurring events.** The id of a recurring event names the series: a patch
  or delete of it changes or deletes every instance. An instance returned by
  `events.list` with `singleEvents` has its own id, and a write to that id
  touches that instance only (Google's
  [recurring events guide](https://developers.google.com/workspace/calendar/api/guides/recurringevents);
  not verified here against a live calendar).
- **Conference data and attachments.** `body.conferenceData` is ignored unless
  `conferenceDataVersion` is `1`; with it, `body.conferenceData.createRequest`
  asks Google to create a new conference, which the pinned document says is
  generated asynchronously. `body.attachments` is changed only with
  `supportsAttachments` set to `true`.
- **The answers.** Insert and patch return the event Google stored, with its
  new `etag` to pin next; a `fields` value narrows that answer and nothing
  else, because no postflight check reads it, and `maxAttendees` truncates its
  guest list as described above. A delete's answer has no body and
  is returned as `null`.
- **Race boundary.** The `etag` is checked in preflight only: the provider
  does not ask Google to check it again with the write, so a change between the
  preflight and the write is not detected, and the guard compares nothing
  afterwards. This is the accepted boundary described under `guard` in
  [the catalog provider guide](local-catalog-provider.md), which also gives the
  `refused`, `applied` and `unknown` classification of the answer.

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
    "authorize_url": "https://accounts.google.com/o/oauth2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/calendar.readonly"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-calendar/operations.json"
}
```

### The write scope

The read-only scope covers no write. The writes need
`https://www.googleapis.com/auth/calendar.events`, which Google describes as
"View and edit events on all your calendars" (the pinned Discovery document).
The pinned document accepts it for `events.insert`, `events.patch`,
`events.delete` and the `events.get` each guard reads, and not
`calendar.readonly` for any of the three writes.

Writes use a separate instance. Configure a second instance id, such as
`google-calendar-write`: the read configuration above with that instance id and
`calendar.events` added to both `minimum_scopes` and `requested_scopes`, as its
own adapter entry in the host configuration, with the writes in its operation
permissions and `private_protocol = "connectors-private/2"`. The read scope
stays in it, so `calendarList.list`, which `calendar.events` does not cover,
still answers on it:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "google-calendar-write",
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
    "minimum_scopes": ["https://www.googleapis.com/auth/calendar.readonly", "https://www.googleapis.com/auth/calendar.events"],
    "token_url": "https://oauth2.googleapis.com/token",
    "authorize_url": "https://accounts.google.com/o/oauth2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/calendar.readonly", "https://www.googleapis.com/auth/calendar.events"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-calendar/operations.json"
}
```

Then `connections connect` that instance with the Google client file. A stored
refresh token granted only the read-only scope fails its validation as
insufficient scope. An existing read-only connection cannot be widened in
place: `connections repair` cannot add a scope, because the configuration
revision and the profile, whose `minimum_scopes` the scope changes, are part of
the connection binding. Changing the scopes of the read instance itself leaves
its connection bound to the old configuration, and a new connection on that
same instance is refused while the old one exists.

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/google_calendar.rs`): the guide's configuration with
  the fixture as API and token host, one token exchange, the exact request of
  each read with the exchanged bearer, the returned body as JSON, a two-page
  walk of each list to the page carrying `nextSyncToken`, a delta read with
  `syncToken`, an expired token's `410` as `not_found`, `eventTypes` sent as two
  pairs, and the `maxResults` bounds of both lists. No live calendar has been
  read.
- The writes are verified against the same fixture under the guide's write
  configuration, through the host's prepare/commit exchange: the exact request
  and body of each write with `sendUpdates` in the query, the `events.get`
  preflight, a stale, missing or unquoted `etag` refused with no write sent, a
  `404` or `410` preflight, or a cancelled or tentative event, refused with no
  write sent, a write without `sendUpdates` refused before any request, a
  delete's empty answer returned as `null`, and a read-only grant refused by
  the write configuration's validation. The approval binding, `sendUpdates`
  included, is verified with the host's approval signer and verifier against a
  subject built from the provider's descriptor, not through the CLI and owner.
  No live
  event has been created, changed or deleted, and no email to a guest has been
  observed.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Google's exact bytes.
- The provider does not walk pages itself and does not retry. On a read, and
  on the preflight read of a patch or delete, a `429`, or a `403` whose reason
  is `rateLimitExceeded`, `userRateLimitExceeded` or `dailyLimitExceeded` (each
  selection names all three in `rate_limit_reasons`), is returned as
  `rate_limited`, and no write is sent; every other `403` is `forbidden`. A
  write that Google answers `429` is reported `unknown`, as
  [the catalog provider guide](local-catalog-provider.md) says: only its
  documented definite refusals are refused, and among them a `403` naming one
  of those reasons is a `rate_limited` refusal.
