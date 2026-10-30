# Zendesk Support through the catalog provider

The catalog provider reads Zendesk Support tickets, their comments, users and
organizations from the pinned Zendesk Support API OpenAPI document, and updates
a ticket, adds a private note or a draft reply behind an approval. Nothing
here is Zendesk-specific code: the pinned document is compiled into a bundle, a
reviewed selection set exposes seven reads and three writes, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Connection and invocation work as described there; this page configures the
`zendesk` connector and covers what differs for Zendesk. Connector, instance
and connection are defined in [the README](../README.md#vocabulary).

## Source and bundle

The pinned source is
[`adapters/zendesk/upstream/zendesk-support.yaml`](../adapters/zendesk/upstream/README.md),
SHA-256 `5dd6cf1eda8febf02f8fb2f2cfa72d7cb6bc7d5bab188ba97b49ab55fc51dbf2`:
the document retrieved on 2026-10-05 from
<https://developer.zendesk.com/zendesk/oas.yaml> (upstream SHA-256
`3a477ea89b274f4d3731f1c7ff93dc93d4520de871ac759b06d3297798fd685d`), with
example credentials, real-looking email addresses and phone numbers redacted by
`connectors-build redact` (rule `upstream-redaction/2`, described in the
source README). The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider zendesk \
  --source adapters/zendesk/upstream/zendesk-support.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile zendesk.basic \
  --amendments adapters/zendesk/upstream/zendesk-support.amendments.json
```

It carries all 652 operations of the pinned document. 35 are reported
unsupported: 34 `page` parameters that take either an offset page number or
the cursor pairs, recorded as the pairs `page[after]`, `page[before]` and
`page[size]` without the offset number (the shipped read `ticket.audits` is one
of them), and one nested `filter` this pass does not expand, sent as one value
as given.
`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle or index a
fresh run would not reproduce byte for byte.

One parameter is added to the extracted inventory by a cited amendment, not by
the pinned bytes: `per_page` on `IncrementalTicketExportCursor`, from
[`zendesk-support.amendments.json`](../adapters/zendesk/upstream/zendesk-support.amendments.json)
(format `connectors-source-amendments/1`). The file is bound to the pinned
document's SHA-256, cites Zendesk's incremental exports reference, and may only
add an optional query parameter the operation does not declare; the bundle's
source record names it with its SHA-256. The pinned document and its digest
stay as retrieved and redacted.

The pinned document declares most parameters once under
`components.parameters` and references them; the bundle reads those references
as the parameters they name. `ListTicketComments` declares its cursor as the
`deepObject` `page`, which the bundle records as the query parameters
`page[after]`, `page[before]` and `page[size]`. `ListAuditsForTicket` declares
`page` as a `oneOf` of an offset page number and those pairs; the bundle records
the pairs only.

## The shipped selection set

[`adapters/catalog/providers/zendesk/operations.json`](../adapters/catalog/providers/zendesk/operations.json)
exposes eight reads and three writes ([Writes](#writes)).
`adapters/catalog/tests/zendesk.rs` pins this exact id list and each
id's `operationId` and path in the pinned document, so a renamed or dropped id,
or a source operation that moved, fails the gate. The bundle refuses at load any
`operation_id` the pinned document lacks.

Every list returns one page per call. The provider returns `status`, `body` and
`provenance`; `body` is Zendesk's page, and the paging fields that decide the
end of a walk are in it.

| id | pinned `operationId` | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `tickets.incremental` | `IncrementalTicketExportCursor` | `GET /api/v2/incremental/tickets/cursor` | `start_time` on the first call, then `cursor`; `per_page` (amended) | `end_of_stream: true` | `start_time`, Unix seconds: tickets changed since |
| `ticket.show` | `ShowTicket` | `GET /api/v2/tickets/{ticket_id}` | single item | n/a | none |
| `ticket.comments` | `ListTicketComments` | `GET /api/v2/tickets/{ticket_id}/comments` | `page[size]`, `page[after]` | `meta.has_more` decides when present; without it, no `meta.after_cursor` or no comments | none; deltas come from `tickets.incremental` |
| `ticket.audits` | `ListAuditsForTicket` | `GET /api/v2/tickets/{ticket_id}/audits` | `page[size]`, `page[after]`; `include_boundary_indicators: true` recommended | `meta.has_more` decides when present; without it, no `meta.after_cursor` or no audits | none; deltas come from `tickets.incremental` |
| `users.incremental` | `IncrementalUserExportCursor` | `GET /api/v2/incremental/users/cursor` | `start_time` on the first call, then `cursor`; `per_page` | `end_of_stream: true` | `start_time`, Unix seconds: users changed since |
| `user.show` | `ShowUser` | `GET /api/v2/users/{user_id}` | single item | n/a | none |
| `organizations.incremental` | `IncrementalOrganizationExport` | `GET /api/v2/incremental/organizations` | `start_time`, then the previous page's `end_time` as `start_time`; `per_page` | `end_of_stream: true` | `start_time`, Unix seconds: organizations changed since |
| `organization.show` | `ShowOrganization` | `GET /api/v2/organizations/{organization_id}` | single item | n/a | none |

- **`tickets.incremental`** and **`users.incremental`** are cursor-based
  incremental exports. Send `start_time` on the first call only; the pinned
  parameter says it is "required on the initial export request; not required
  on subsequent cursor-based pagination requests" and must be at least one
  minute in the past. Each following call sends the previous page's
  `after_cursor` as `cursor`. The walk ends on the page whose `end_of_stream`
  is `true`; that page's `after_cursor` is the cursor to resume from on the
  next run. `per_page` (1 to 1,000; Zendesk's default 1,000) sets the page
  size. The pinned document declares it for the user export only; for the
  ticket export it is the one cited amendment
  ([`zendesk-support.amendments.json`](../adapters/zendesk/upstream/zendesk-support.amendments.json)),
  because a full page of 1,000 tickets can pass the provider's 4 MiB response
  bound (`connectors-core` `RESPONSE_LIMIT`) and is then refused as
  `capacity`. `support_type_scope` (`all`, `agent`,
  `ai_agent`; Zendesk's default `agent`) selects the tickets exported.
- **`organizations.incremental`** is time-based: the pinned document has no
  cursor export for organizations. `start_time` is required on every call.
  The next call sends the previous page's `end_time` as `start_time`, which is
  also the `start_time` in the page's `next_page` URL; the walk ends on the
  page whose `end_of_stream` is `true`. Whether two pages can share an
  organization at their `end_time` boundary has not been checked against a
  live account; a consumer that keys organizations by `id` and keeps the
  latest `updated_at` is unaffected either way.
- **`ticket.comments`** takes `ticket_id` and is read by cursor. The first call
  sends `page[size]` (the pinned description states at most 100 per page); each
  following call sends the previous page's `meta.after_cursor` as
  `page[after]`. When a page carries `meta.has_more`, it decides: `false` ends
  the walk, and `true` continues even past an empty page, and is an error when
  the page has no `meta.after_cursor`. Without `meta.has_more`, the walk ends on
  a page with no `meta.after_cursor` (absent or `null`) or with no comments. A
  cursor seen before in the same walk is an error, so a walk cannot loop. The
  pinned response schema declares only `comments`; `meta` and `links` come
  from Zendesk's pagination guide, which the operation's description links,
  and the operation has no parameter that asks for `has_more`.
  Offset paging (`per_page`, `sort_order`) is withheld by the selection, so an
  input carrying either is refused before any request. The engine sends the
  bracketed names percent-encoded: `page%5Bsize%5D=2&page%5Bafter%5D=…`.
- **`ticket.audits`** takes `ticket_id` and walks one ticket's audits by
  cursor as `ticket.comments` walks its comments: `page[size]` (at most 100 per
  page), then the previous page's `meta.after_cursor` as `page[after]`, by the
  same rule: `meta.has_more` decides when present (`false` ends, `true`
  continues even past an empty page and is an error without
  `meta.after_cursor`); without it, a page with no `meta.after_cursor` or no
  audits ends the walk; a cursor seen before is an error. The pinned response
  schema (`TicketAuditsResponseNoneCursor`) declares no `meta`, and the pinned
  `include_boundary_indicators` "includes `has_more` indicator in the cursor
  pagination response meta" when true, so sending
  `"include_boundary_indicators": true` on every call is recommended: with
  `has_more` present, an empty page in the middle of a walk does not end it.
  Whether Zendesk leaves `has_more` out without it has not been checked
  against a live account.
  `sort_order` is withheld; the operation has no `per_page`, and the offset
  page number is not declared. Each audit is returned as Zendesk sends it:
  `author_id`, `created_at`, `via.channel`, `via.source.rel` and `events`, each
  event with its `type` and `id`, a field change with `field_name`, `value` and
  `previous_value`. It is not `ListTicketAudits` (`GET /api/v2/ticket_audits`),
  which lists every ticket's audits.

  ```sh
  connectors invoke zendesk/support ticket.audits \
    --input-json '{"ticket_id":101,"page[size]":100,"include_boundary_indicators":true}'
  echo '{"ticket_id":101,"page[size]":100,"include_boundary_indicators":true,"page[after]":"<meta.after_cursor>"}' \
    | connectors invoke zendesk/support ticket.audits
  ```
- **`ticket.show`**, **`user.show`** and **`organization.show`** take the
  numeric id from the path; tickets reference users (`requester_id`,
  `submitter_id`, `assignee_id`) and organizations (`organization_id`), and
  users reference organizations (`organization_id`).

Comments offer no time filter. To take deltas, export tickets since the last
run's cursor and re-read the comments of each returned ticket. Whether a given
kind of change moves a ticket into the export is Zendesk's behaviour and has not
been checked against a live account here. Every other query parameter the pinned
document declares for an operation is accepted by name; one it does not declare
is refused before any request.

## Configuration

The `zendesk` connector has no default endpoint: an instance names its
account's `https://<subdomain>.zendesk.com`. It declares two auth profiles,
`zendesk.oauth` and `zendesk.basic`
([`adapters/catalog/providers/zendesk/connector.json`](../adapters/catalog/providers/zendesk/connector.json)),
so the instance names the one it uses with `profile`. Add to the host
configuration (`~/.config/connectors/config.toml`, format
`connectors-local/5`):

```toml
[connectors.zendesk]
enabled = true

[connectors.zendesk.instances.support]
endpoint = "https://your-subdomain.zendesk.com"
profile = "zendesk.oauth"
```

Everything else is derived from the shipped connector: the bundle, the seven
reads, the profile's fields and the provider executable. The instance id is
`zendesk.support`:

```console
$ connectors instances list
{"adapters":[{"connector":"zendesk","instance":"support","instance_id":"zendesk.support","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

An installation configured before connectors were declared converts its
entries with `connectors setup migrate`, which keeps each instance id and
revision; the pinned form it writes is described in
[the catalog provider guide](local-catalog-provider.md).

## Writes

Three writes select the pinned `UpdateTicket`, each `effect: write`, so each
runs only under an issued approval: through the local CLI, `approvals prepare`,
`approvals issue` and `invoke` with the proof, and never on the read
transport. Each takes `ticket_id` and a `body` of the form
`{"ticket": {...}}`, closed to the members below
([closed body keys](local-catalog-provider.md#configure-the-provider)); any other
member, in `ticket` or beside it, is refused as `invalid_input` before any
request.

| id | pinned `operationId` | request | caller's `ticket` members | fixed by the selection | guard |
|---|---|---|---|---|---|
| `ticket.update` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `group_id` (integer), `tags` (list of strings), `priority`, `type`, at least one of them; `updated_stamp` | `safe_update: true` | preflight `ShowTicket`: `updated_at` equals `updated_stamp`, else `conflict` unsent; `safe_update`, whose 409 from Zendesk is `conflict` |
| `ticket.note` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `comment.body` (non-empty); `updated_stamp` | `comment.public: false`, `safe_update: true` | preflight `ShowTicket`: `updated_at` equals `updated_stamp`, else `conflict` unsent; `safe_update`, whose 409 from Zendesk is `conflict` |
| `ticket.draft_reply` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `comment.body` (non-empty); `updated_stamp` | `comment.public: false`, `safe_update: true`, the first line `[Draft reply - not sent to the requester]` | preflight `ShowTicket`: `updated_at` equals `updated_stamp`, else `conflict` unsent; `safe_update`, whose 409 from Zendesk is `conflict` |

- **Lost updates are refused, not applied.** Read the ticket with
  `ticket.show` and pass its `ticket.updated_at` as `updated_stamp`. Before the
  write is sent, the provider reads the ticket once more and refuses with
  nothing sent when `updated_at` differs, as `service_failure` with
  `service_code: conflict`. The write then carries `safe_update: true` and the
  stamp, so Zendesk refuses it with `409` when the ticket changed between that
  read and the write; the provider reports that as `refused` with
  `code: service_failure`, `service_code: conflict`, and never sends it again.
  Read the ticket again and prepare a new approval for the new input.
- **`ticket.update`** sets whichever of `group_id`, `tags`, `priority` and
  `type` the body carries; one with none of them is refused. `tags`
  **replaces** the ticket's whole tag set, as the pinned document says ("the set
  tag behavior is used, which overwrites and replaces existing tags"); adding
  tags without replacing them (`PUT /api/v2/tickets/{ticket_id}/tags`) is not
  selected. `priority` admits only `urgent`, `high`, `normal` and `low`, and
  `type` only `problem`, `incident`, `question` and `task`, the pinned enums;
  any other value is refused as `invalid_input` naming the field, before
  anything is sent. Clearing a field (`null`) is not admitted.
- **`ticket.note`** adds an internal note. `comment.public` is fixed to
  `false`: a body carrying `public` at all, `true` or `false`, is refused, so a
  note can never reach the requester. Nothing compares notes before an append:
  a note sent twice is two notes.
- **`ticket.draft_reply`** is a note too: the pinned document has no draft
  object or endpoint. Its text is sent behind the fixed first line
  `[Draft reply - not sent to the requester]`, which the selection adds before
  whatever the caller writes, so the marker is always the first line; a person
  copies the text into a public reply. Nothing is sent to the requester.
- A write whose answer is lost after it was sent, including by a timeout, is
  `outcome_unknown` and is never sent again; inspect the ticket before
  preparing another.

### Use the writes from a local install

The writes ship in the release that carries this page
(<https://github.com/beyond10x/connectors/releases>). They are verified with the
basic profile `zendesk.basic` (an API token of an agent allowed to update the
ticket) only; the documented `zendesk.oauth` block requests `scope=read`, which
does not cover a write. Move an existing install over in four steps:

1. Replace the binaries with the release's `connectors` and
   `connectors-catalog-provider`. An instance of the `zendesk` connector takes
   the selection, the three writes beside the seven reads, and the provider
   executable from the shipped connector, so nothing is copied into the
   configuration. An installation still configured before connectors were
   declared converts its entries with `connectors setup migrate` first.

2. Name the basic profile on the instance. In the `[connectors.zendesk.instances.support]`
   table you already have, set `profile = "zendesk.basic"`. The whole connector block then reads:

   ```toml
   [connectors.zendesk]
   enabled = true

   [connectors.zendesk.instances.support]
   endpoint = "https://your-subdomain.zendesk.com"
   profile = "zendesk.basic"
   ```

   A second `[connectors.zendesk.instances.support]` table makes the file unreadable.

3. Configure a [bounded authenticated clock](local-clock.md), initialize
   [approval-signing keys](local-approval-keys.md), and publish an approval
   policy naming the writes with the [approval commands](local-approvals.md):

   ```json
   {"operations":["ticket.update","ticket.note","ticket.draft_reply"]}
   ```

4. Connect with the [API token](#api-token-zendeskbasic) credential document
   (`connections connect --connector zendesk --instance support --profile
   zendesk.basic --credential-file ~/private/zendesk.json`) and take each write's
   `schema` and `revision` from `connectors invoke zendesk/support <write> --describe`.
   For each write, put the input in a file, `approvals prepare` it, pass the subject's digest to
   `approvals issue --approve-subject`, and invoke with the proof:

   ```sh
   connectors --output json invoke zendesk/support ticket.note --input-file note.json \
     --approval-file "$proof_file" --idempotency-key "$business_key"
   ```

   ```json
   {"ticket_id": 101, "body": {"ticket": {"comment": {"body": "Checked the billing history."}, "updated_stamp": "2025-09-16T10:00:00Z"}}}
   ```

   The approval binds the operation and the whole input, the stamp included,
   so a re-read ticket needs a new approval.

## Authentication

Use an OAuth client with the client-credentials grant. Zendesk is retiring API
tokens: accounts created on or after 2026-07-28 cannot create or use them,
existing accounts cannot create new ones after 2026-10-27, and every remaining
API token stops working on 2027-04-30 (Zendesk, "Migrating from API tokens to
OAuth access tokens", read 2026-10-05). A refresh-token grant does not fit
either: Zendesk refresh tokens are single-use and rotate on every exchange,
which `oauth2_refresh` refuses.

### OAuth client credentials (`zendesk.oauth`)

Create a confidential OAuth client in Admin Center (**Apps and integrations >
APIs > OAuth clients**), as the admin, or a dedicated service user, whose
actions the reads should be attributed to: Zendesk attributes a
client-credentials token to the user who created the client. The profile
requests tokens at `<endpoint>/oauth/tokens` with the scope `read`, and its
identity read is `GET /api/v2/users/me`. Connect the instance with the client's
credentials, `{"client_id": "<unique identifier>", "client_secret": "<secret>"}`,
in an owner-only file:

```sh
connectors connections connect --connector zendesk --instance support \
  --profile zendesk.oauth --credential-file ~/private/zendesk-oauth.json
```

The client's credentials are stored once. Every access token is requested with
`grant_type=client_credentials` and `scope=read`. Zendesk gives clients created
on or after 2026-04-30 a 30-minute default lifetime; an older client's answer
carries no `expires_in`, and the provider then uses the token for at most 24
hours or until a request refuses it. Either way the provider requests a new one
when it expires, so there is nothing to refresh and the stored entry never
changes. The request is form-encoded, as RFC 6749 requires; Zendesk's pages
show both form-encoded and JSON examples, and which it accepts has not been
checked against a live account. Zendesk also documents
per-resource scopes such as `tickets:read` and `users:read`; whether those
alone admit the incremental exports and `users/me` has not been checked against
a live account. Nothing here has run against a live account yet.

### API token (`zendesk.basic`)

The pinned document declares one security scheme, HTTP basic (`basicAuth`). With
a Zendesk API token, the user name is the account email followed by `/token`
and the password is the token (Zendesk's API authentication guide; not checked
against a live account here). An instance that uses it states
`profile = "zendesk.basic"`. The identity
read is `GET /api/v2/users/me` (`ShowCurrentUser`), whose `user.id` names the
credential's subject. The pinned document allows that read to anonymous users;
an answer without a user id refuses the connection. Zendesk API tokens expose no
scope list, so the profile declares no `scopes` read and no `minimum_scopes`.
The incremental exports are allowed to admins only (pinned "Allowed For").

Connect it with the credential document
`{"account": "<email>/token", "token": "<API token>"}` and
`--profile zendesk.basic`; every request then
carries `Authorization: Basic base64(<email>/token:<API token>)`.

## Limits

- Verified against a local HTTPS fixture (`adapters/catalog/tests/zendesk.rs`)
  with synthetic records only: the exact request of each read, including the
  `start_time` filter and the basic header, the returned body, a two-page walk
  of each list to the end condition above, and the identity read; and each
  write's preflight and exact `PUT` body, a `409` answer refused as `conflict`
  after one request, the fixed `public: false` and draft marker, the pinned
  enums refused before any request, and no write without an approval. Nothing
  has been run against a live Zendesk account; the fixture's `409` body is
  synthetic, and the provider classifies by the status alone.
- The engine parses and re-serialises the body. The fixture's bodies are
  compact JSON with sorted keys, and the test asserts the returned body is
  those exact bytes; a provider body with other spacing or key order is
  returned as equal JSON, not as its exact bytes.
- The provider does not walk pages itself. It bounds `per_page` only on
  `tickets.incremental` (1 to 1,000), and no other `per_page` or `page[size]`. It sends a read answered `429` once more after
  the delay its `Retry-After` names, when that wait leaves the second request
  time of its own before the invocation deadline, and otherwise returns it as a `rate_limited` refusal carrying that
  delay as `retry_after_seconds`
  ([Limits](local-catalog-provider.md#limits)).
