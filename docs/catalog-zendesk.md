# Zendesk Support through the catalog provider

The catalog provider reads Zendesk Support tickets, their comments, users and
organizations from the pinned Zendesk Support API OpenAPI document. Nothing
here is Zendesk-specific code: the pinned document is compiled into a bundle, a
reviewed selection set exposes seven reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work exactly as described
there; this page covers what differs for Zendesk.

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
unsupported: `deepObject` query parameters this pass does not expand, each sent
as one value as given (34 `page` parameters that also take an offset page
number, and one nested `filter`); no shipped read is among them.
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
`page[after]`, `page[before]` and `page[size]`.

## The shipped selection set

[`adapters/catalog/providers/zendesk/operations.json`](../adapters/catalog/providers/zendesk/operations.json)
exposes seven reads and nothing else. Each is `effect: read`; there are no
writes. `adapters/catalog/tests/zendesk.rs` pins this exact id list and each
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
| `ticket.comments` | `ListTicketComments` | `GET /api/v2/tickets/{ticket_id}/comments` | `page[size]`, `page[after]` | `meta.has_more: false` | none; deltas come from `tickets.incremental` |
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
  `page[after]`. The walk ends on the page whose `meta.has_more` is `false`.
  The pinned response schema declares only `comments`; `meta` and `links` come
  from Zendesk's pagination guide, which the operation's description links.
  Offset paging (`per_page`, `sort_order`) is withheld by the selection, so an
  input carrying either is refused before any request. The engine sends the
  bracketed names percent-encoded: `page%5Bsize%5D=2&page%5Bafter%5D=…`.
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
client-credentials token to the user who created the client. Then declare:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "zendesk-support",
  "provider": "zendesk",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://your-subdomain.zendesk.com",
  "auth": {
    "profile": "zendesk.oauth",
    "scheme": "oauth2_client_credentials",
    "header": "Authorization",
    "bearer": true,
    "label": "OAuth client secret",
    "identity": {"path": "api/v2/users/me", "kind": "zendesk.user", "subject_pointer": "/user/id"},
    "token_url": "https://your-subdomain.zendesk.com/oauth/tokens",
    "requested_scopes": ["read"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/zendesk/operations.json"
}
```

`connections connect` takes `{"client_id": "<unique identifier>", "client_secret":
"<secret>"}` once. Every access token is requested with
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
against a live account here). Declare the basic profile `zendesk.basic`. The identity
read is `GET /api/v2/users/me` (`ShowCurrentUser`), whose `user.id` names the
credential's subject. The pinned document allows that read to anonymous users;
an answer without a user id refuses the connection. Zendesk API tokens expose no
scope list, so the profile declares no `scopes` read and no `minimum_scopes`.
The incremental exports are allowed to admins only (pinned "Allowed For").

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "zendesk-support",
  "provider": "zendesk",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://your-subdomain.zendesk.com",
  "auth": {
    "profile": "zendesk.basic",
    "scheme": "basic",
    "header": "Authorization",
    "bearer": false,
    "account_label": "Account email followed by /token",
    "label": "API token",
    "identity": {"path": "api/v2/users/me", "kind": "zendesk.user", "subject_pointer": "/user/id"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/zendesk/operations.json"
}
```

`connections connect` takes the credential document
`{"account": "<email>/token", "token": "<API token>"}`; every request then
carries `Authorization: Basic base64(<email>/token:<API token>)`.

## Limits

- Verified against a local HTTPS fixture (`adapters/catalog/tests/zendesk.rs`)
  with synthetic records only: the exact request of each read, including the
  `start_time` filter and the basic header, the returned body, a two-page walk
  of each list to the end condition above, and the identity read. Nothing has
  been run against a live Zendesk account.
- The engine parses and re-serialises the body. The fixture's bodies are
  compact JSON with sorted keys, and the test asserts the returned body is
  those exact bytes; a provider body with other spacing or key order is
  returned as equal JSON, not as its exact bytes.
- The provider does not walk pages itself, does not bound `per_page` or
  `page[size]`, and does not retry on `429`; a rate-limited read is returned as
  a refusal.
