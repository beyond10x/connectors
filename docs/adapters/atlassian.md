# Adapter design: Atlassian (Jira and Confluence)

- **Status:** design, not implemented. No `adapters/atlassian/` exists.
- **Old baseline:** `../connectors` at `81459ac4`: `providers/jira.toml` (897 lines), `providers/confluence.toml` (740 lines), `crates/integration-jira/`, repository-authored specs `specs/jira/`, `specs/confluence/`.
- **Contract index:** [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

One adapter service `connectors.atlassian` with two modules (`jira`, `confluence`) sharing auth, configuration and HTTP infrastructure (`docs/design.md:258-288`, `452`). Placement: any network with egress to `api.atlassian.com`; old operations required `public_network` (`providers/jira.toml`, `required_capabilities`). Rebuild means: the 12 Jira and 7 Confluence operations, the Jira issues datasource, and the four Atlassian credential kinds, on the new contracts, without the catalog, the `ConnectorBackend` shape, or per-integration OAuth copies.

## 2. Old surface and disposition

Jira (`providers/jira.toml`; line = declaration start):

| Old id | Line | Kind | Disposition |
|---|---|---|---|
| `jira-issue-get` | 321 | read, `GET /rest/api/2/issue/{issueIdOrKey}`, full field set | preserve → `datasource.records` `document` profile `jira-issue` |
| `jira-issue-create` | 376 | write | preserve → `operations` `mutation` |
| `jira-issue-comment-list` | 442 | read | preserve → `datasource.records` list |
| `jira-issue-comment-add` | 476 | write, v2 plain-string body (wiki markup) | preserve → mutation; v2 decision preserved (`jira.toml:18-60`) |
| `jira-issue-transitions-list` | 526 | read | preserve → records list |
| `jira-issue-transition` | 566 | write | preserve → mutation |
| `jira-issue-edit` | 620 | write, summary-only (`specs/jira/runtime-writes-2026-08-17.openapi.yaml`) | preserve → mutation |
| `jira-issue-comment-edit` | 651 | write | preserve → mutation |
| `jira-issue-link-add` | 693 | write | preserve → mutation |
| `jira-issue-search` | 799 | read, `GET /rest/api/2/search/jql`, fixed JQL `updated >= <ms>` ordered `updated ASC, key ASC`, `next_page_token`, limit ≤ 100 | change: keep the incremental profile; add a bounded free-JQL profile only if a consumer needs it (`docs/design.md:936`) |
| `jira-project-list` | 835 | read, `startAt`/`maxResults`, restricted to configured keys, follow `next_start_at` even when policy empties a page | preserve → records list |
| `jira-issue-comments-read` | 865 | read, bounded comment page, author identities omitted | preserve → records list; merge with `comment-list` into one operation with a declared projection |
| Jira issues datasource `datasource-binding:jira:issues:` | `crates/integration-jira/src/backend.rs:343-352` | preserve as the `issue-search` incremental profile's cursor binding |
| `events: false` | `backend.rs:279` | no events; webhooks deferred |

Confluence (`providers/confluence.toml`):

| Old id | Line | Kind | Disposition |
|---|---|---|---|
| `confluence-space-list` | 386 | read | preserve → records list |
| `confluence-space-get` | 432 | read | preserve → records (single item, no body) |
| `confluence-space-pages` | 461 | read, `spaceKey`, `type=page`, `limit` ≤ 100, `start`; no bodies | preserve → records list |
| `confluence-page-get` | 510 | read, `expand=body.storage,version`; storage format body | preserve → `document` profile `confluence-page` |
| `confluence-page-create` | 548 | write | preserve → mutation |
| `confluence-comment-add` | 617 | write | preserve → mutation |
| `confluence-page-search` | 708 | read, CQL, cursor, `expand=version,body.storage,space`; account-timezone dates | preserve → records list with body projection under the document byte bound |

Auth (`providers/jira.toml:140-297`, `confluence.toml` auth block): `jira.api_token` (basic, email + token), `jira.user_oauth` (bearer, user, authorization_code + refresh at `https://auth.atlassian.com`, scopes `offline_access read:jira-work read:me write:jira-work`), `jira.service_oauth` (bearer, app, client_credentials, `read:jira-work`), `jira.service_api_token` (bearer, app); `confluence.api_token` (basic), `confluence.service_api_token` (bearer, app). Old configuration: `cloud_id`, `email`, `api_token`, `oauth_client_id`, `oauth_redirect_uri`, `oauth_client_secret` (Jira); `cloud_id`, `email`, `api_token` (Confluence). Base URLs `https://api.atlassian.com/ex/jira/{cloud_id}` (`jira.toml:206`) and `https://api.atlassian.com/ex/confluence/{cloud_id}/wiki` (`confluence.toml:280`).

## 3. Contracts needed and why

| Contract | Profile / use | Why |
|---|---|---|
| `operations/v1alpha1` | base | describe/invoke for every operation |
| `operations` `mutation` | 6 Jira + 2 Confluence writes | issue creation, comments, transitions and page creation are externally visible writes needing approval binding, idempotency and unknown-outcome reporting (`contracts/operations/v1alpha1/semantics.md`) |
| `datasource.records` list | projects, comments, transitions, spaces, space pages, page search, incremental issue search | paged reads with provider continuation (`next_page_token`, `startAt`, cursor) and allowlist scoping |
| `datasource.records` `document` | `jira-issue`, `confluence-page` | bodies with a named representation (`jira.fields.v2`, `confluence.storage`), version, byte bound (`contracts/datasources/records/v1alpha1/semantics.md`) |
| `auth.profile` | 6 profiles above | four distinct Atlassian credential purposes; scopes attach to the credential (`contracts/auth/profile/v1alpha1/semantics.md`) |
| `auth.acquisition` | `oauth2_authorization_code` (user), `oauth2_client_credentials` (service), `static_entry` (API tokens, two-field entry) | replaces the Jira-specific OAuth copy (`../connectors/crates/connector-oauth/src/lib.rs` header) |
| `auth.custody` | `versioned` | access+refresh stored as one immutable set; rotating refresh also requires the host coordinator protocol below |
| `auth.capability` | `http-basic`, `http-bearer` | API token is HTTP basic with the email as user half; OAuth is bearer; GET and POST/PUT needed |
| `auth.connection` | `managed` | several users' OAuth identities per instance; stable refs across reauthorization; `cloud_id` as a connection attribute |
| `auth.evidence` | `scope_check`, `identity_check` (accessible-resources / `me`), `verify_operation` = `project-list` | operations require `read:jira-work` or `write:jira-work`; verify without effects |

Rotating-token profiles require [acquisition §4.1](../../contracts/auth/acquisition/v1alpha1/semantics.md): cross-replica source-generation reservation, a durable single exchange authorization before send, stale-owner fencing, and guarded publication serialized with revocation. The secret binding provides immutable durable versions, not a refresh lease. A lost owner may be replaced before authorization only after fencing. After authorization, a successor may recover a durably recorded validated response for publication; otherwise it requires repair/reauthorization without another exchange. Provider grace/reuse behavior is not a retry exception. No Atlassian endpoint or client-library retry may bypass this rule.

Every successful refresh creates a new private credential generation for the same expected external identity; F05 evidence validates the candidate and publication invalidates old-generation dispatch admissions. A new account must not silently replace the bound account. While refresh is unresolved, affected old-generation dispatch is withheld; a pin does not override known invalidation. A metadata or custody binding that cannot provide the required guarantees refuses rotating refresh. These are design obligations; no Atlassian adapter or runtime refresh implementation is added here.

Not needed: discovery (no observations), sessions/media, mediated routes.

## 4. Operation map

| New id | Contract / profile | Effects | Old id |
|---|---|---|---|
| `jira.projects.list` | records list `jira-projects` | read | `jira-project-list` |
| `jira.issues.search_incremental` | records list `jira-issues-incremental` | read | `jira-issue-search` |
| `jira.issue.get` | records document `jira-issue` | read | `jira-issue-get` |
| `jira.issue.comments.list` | records list `jira-comments` | read | `jira-issue-comment-list`, `jira-issue-comments-read` |
| `jira.issue.transitions.list` | records list `jira-transitions` | read | `jira-issue-transitions-list` |
| `jira.issue.create` | mutation, `idempotency: keyed`, `approval: required` | external_write | `jira-issue-create` |
| `jira.issue.edit` | mutation, keyed, approval required | external_write | `jira-issue-edit` |
| `jira.issue.transition` | mutation, keyed, approval required | external_write | `jira-issue-transition` |
| `jira.issue.comment.add` | mutation, keyed, approval required | external_write, human_visible | `jira-issue-comment-add` |
| `jira.issue.comment.edit` | mutation, keyed, approval required | external_write, human_visible | `jira-issue-comment-edit` |
| `jira.issue.link.add` | mutation, keyed, approval required | external_write | `jira-issue-link-add` |
| `confluence.spaces.list` | records list | read | `confluence-space-list` |
| `confluence.space.get` | records list (single) | read | `confluence-space-get` |
| `confluence.space.pages.list` | records list | read | `confluence-space-pages` |
| `confluence.page.get` | records document `confluence-page` | read | `confluence-page-get` |
| `confluence.pages.search` | records list `confluence-cql` | read | `confluence-page-search` |
| `confluence.page.create` | mutation, keyed, approval required | external_write, human_visible | `confluence-page-create` |
| `confluence.comment.add` | mutation, keyed, approval required | external_write, human_visible | `confluence-comment-add` |

Jira stays on REST v2 for plain-string comment bodies; ADF (v3) is a later `jira.adf` representation (`jira.toml:18-60`).

For every keyed write, the receiver owns reservation and replay under [mutation §5.1](../../contracts/operations/v1alpha1/semantics.md#51-key-namespace-fingerprint-and-replay-admission). Jira and Confluence do not supply a substitute key namespace. The same key/body used by different admitted callers or trusted origins is isolated; within one namespace, changing issue/comment/page operation, connection, input or fingerprint revision conflicts while the reservation remains live. The fingerprint includes the connection metadata revision, so an authorized change of external identity or `cloud_id` cannot replay or reuse a previous binding's result silently. Same-identity token refresh alone does not create a new namespace. Every replay requires current project/space/resource/result access, but never redeems approval again. Pending and unknown outcomes remain reserved across restart and past the known-result replay window; a known result expires 86,400 s after durable settlement, after which a new attempt needs fresh admission/approval.

## 5. Auth

| Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|
| `jira.api_token`, `confluence.api_token` | `http_basic` (email + token) | `static_entry`, two fields | `http-basic` | `verify_operation` |
| `jira.user_oauth` | `http_bearer`, user | `oauth2_authorization_code` + refresh; PKCE `none` unless the vendor page confirms support | `http-bearer` | `scope_check`, `identity_check` |
| `jira.service_oauth` | `http_bearer`, app | `oauth2_client_credentials` | `http-bearer` | `scope_check` |
| `jira.service_api_token`, `confluence.service_api_token` | `http_bearer`, app | `static_entry`, one field | `http-bearer` | `verify_operation` |

The same Atlassian API token serves Jira and Confluence but is stored per connection (old rule, `confluence.toml` auth comment). `cloud_id` is resolved at acquisition from Atlassian's accessible-resources endpoint for OAuth, or entered for API tokens; it is a connection attribute, never request input.

## 6. Configuration outline

```json
{ "service": { "$ref": "urn:connectors:config:v1:service" },
  "http": { "$ref": "urn:connectors:config:v1:http" },
  "adapter": {
    "jira": { "allowed_projects": ["ENG"], "enabled_writes": ["jira.issue.comment.add"], "page_limit": 100 },
    "confluence": { "allowed_spaces": ["ENG"], "enabled_writes": [], "max_body_bytes": 262144 },
    "auth": { "registration": { "client_id": "…", "client_secret": { "$ref": "urn:connectors:config:v1:credential" }, "redirect": "host_callback" } }
  } }
```

Writes are disabled unless listed; allowlists apply before dispatch (`contracts/service/v1alpha1/semantics.md`, GitLab pattern).

## 7. Discovery and routes

None.

## 8. Specification profile

`connectors.adapter/v2` for GET operations against repository-authored OpenAPI 3.1 sources (`specs/jira/incremental-reads.openapi.yaml`, `specs/confluence/incremental-reads.openapi.yaml`, both `openapi: 3.1.0`, provenance in `specs/*.provenance.toml`); handwritten `/v1`-style obligations for mutations until the v2 profile supports non-GET methods and bodies (`spec-kinds/adapter/v2/semantics.md:13-15`). Atlassian's full generated platform document is not imported (old rationale, `jira.toml:3-11`).

## 9. Deferred

`events` (Jira/Confluence webhooks), attachments (binary documents), ADF representation, free JQL search.

## 10. Evidence required

- Fixture: auth header placement for basic and bearer; paging for `startAt`, `nextPageToken`, CQL cursor; document truncation; every mutation with approval, replay, and lost-response scenarios; scope refusal.
- Keyed-write fixtures: reuse key/body across caller, origin, Jira/Confluence operation and connection boundaries; revoke project/space access before replay; rotate same-identity credentials; change connection/configuration revision; expire known-result retention; keep in-flight/unknown reservations and generation-bound waiters safe. These are host/adapter binding obligations, not implemented provider behavior.
- Refresh fixtures: two replicas, loss before and after authorization (including before send), response loss, durable response recovery, stale publication, publication/revocation ordering and old-generation dispatch cutoff. Observe at most one provider exchange per source generation; retain consumed-source records after repair. See the [semantic verification matrix](../../contracts/auth/acquisition/v1alpha1/verification.md); ESS compilation does not execute these fixtures.
- Live: Atlassian authentication chain (`docs/design.md:1000`): connect, bounded read, expire, refresh or repair, identity preserved; repeated with a second custody binding.
- Decoupling: adapter builds without siblings; business tests run with fake authenticated transport and no OAuth server (`docs/design.md:1012`).
