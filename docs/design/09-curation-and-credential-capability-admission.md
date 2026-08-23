# Design 09: catalog curation and credential capability admission

**Status:** catalog/compiler contract implemented; Connection capability admission remains M3 work ·
**Date:** 2026-08-15

This document fixes two related boundaries exposed by the Slack connector:

1. a large vendor specification is evidence, not an agent tool list; and
2. possession of a credential is not proof that the credential can perform every operation that
   accepts its wire format.

The rules are generic. Slack proves delegated-user versus bot authority across Web API, Events API,
Admin API, and Socket Mode. GitLab proves that a user OAuth/PAT Connection and several automation
token kinds can share one API surface without sharing an effective actor.

## 1. Source, catalog, exposure, and admission are separate decisions

```text
official or repository-authored source
                │ exact select + reviewed overlay
                ▼
       catalogued provider member
                │ expose = true
                ▼
       eligible agent operation
                │ Connection credential capability evidence
                │ ∩ Connection lifecycle and initiation policy
                │ ∩ Connector Grant and caller authority
                ▼
          surfaced and invocable
```

The source grants nothing. OpenAPI and AsyncAPI ingestion make exact identifiers available for
patching and select none by default. A reviewed patch may add a member to the catalog; only a
separate `expose = true` decision makes an operation eligible for model projection. At runtime,
discovery then intersects that static eligibility with the selected Connection's proven credential
capabilities and the existing authorization gates.

- *Invariant:* source refresh cannot widen the callable or agent surface without a reviewed patch.
- *Invariant:* a catalogued operation with `expose = false` remains callable by an authorized
  non-model client but is not projected to a model.
- *Invariant:* a missing, stale, or ambiguous capability observation removes a scope-gated member
  from discovery and refuses invocation. A downstream vendor `missing_scope` response is a race or
  revocation signal, never the primary authorization mechanism.

## 2. Requirements attach scopes to the credential that owns them

An auth requirement remains an OR list of AND credential mechanisms. Each mechanism may now also
declare a credential-local OR-of-AND scope expression:

```toml
auth = [{
  credentials = ["slack.app_token"],
  scopes = { "slack.app_token" = [["connections:write"]] }
}]
```

`[["a", "b"], ["c"]]` means `(a AND b) OR c`. A scope map key must name a credential in the
same mechanism. Scope sets are sorted and deduplicated before hashing so author order cannot change
the canonical artifact.

This association is load-bearing. Combining all scopes on a Connection into one bag would allow an
app-level `connections:write` grant to look like authority held by a bot token, or admin authority
held by an ordinary user token.

Operation, event, and channel requirements are independent:

- operation auth authorizes one outbound API call;
- event auth states the installed scope required for that event subscription to deliver;
- channel auth authorizes the transport itself; and
- webhook verification proves inbound bytes without authorizing an API operation.

## 3. Slack credential roles

| Connector credential purpose | Slack credential kind | Subject | Initial authority |
|---|---|---|---|
| `slack.bot_token` | bot OAuth token, commonly `xoxb-…` | app/bot | curated ordinary Web API operations and bot event subscriptions |
| `slack.user_token` | delegated user OAuth token, commonly `xoxp-…` | user | the four reviewed ordinary Web API operations, performed as the consenting user |
| `slack.admin_token` | **user OAuth token** from an Enterprise organization-wide Admin/Owner install | user | the selected read-only `admin.*` methods whose scopes are proven |
| `slack.app_token` | app-level token, `xapp-…` | app | Socket Mode ticket creation only, with `connections:write` |
| `slack.signing_secret` | app signing secret, not an API token | app | Events API HMAC verification only |

`admin token` is a Connector purpose, not a new Slack token kind or prefix. Slack Admin API scopes
are user-token scopes; an app must be installed across the Enterprise organization by an Org Admin
or Owner. Keeping a dedicated credential purpose prevents a normal delegated user token from being
silently substituted even though both are user-subject bearer tokens.

An app-level token is also not an app-configuration token. Slack configuration tokens are
short-lived, user/workspace-bound credentials for the App Manifest APIs. No App Manifest operation
is selected in the initial connector, so no configuration-token credential is declared or
collected. If app provisioning becomes a requirement later, it gets a separate
`slack.configuration_token` purpose, acquisition flow, curated operation set, and scope admission;
it is never folded into `slack.app_token` or `slack.admin_token`.

Token prefixes are useful input diagnostics, not authority. Admission never infers subject, scope,
Enterprise installation, or allowed operation from a prefix.

### 3.1 A Zwirn caller is not a Slack credential subject

“The bot does something on behalf of me” describes two separate facts. Zwirn is the authenticated
platform caller. Slack sees whichever credential belongs to the selected Connection:

```text
Zwirn caller ── Grant ──▶ tenant-shared Slack bot Connection ── xoxb ──▶ Slack sees the app bot
             └─ Grant ──▶ principal-owned Slack user Connection ─ xoxp ─▶ Slack sees that user
```

The second route is delegated execution; it is not impersonation and the bot token never becomes a
user token. Slack's dedicated user-centric flow (`/oauth/v2_user/authorize` followed by
`/api/oauth.v2.user.access`) lets one consent create one principal-owned user Connection with one
credential. The ordinary app-install flow creates the app-subject bot Connection separately. Both
use the same operator-owned Slack app registration. Slack's documented comma-separated scope wire
form is declared as `scope_separator = "comma"`; the host must not encode the list as one
space-delimited value by assumption. Its user-token response locates the actual grant at
`scope_response_pointer = "/authed_user/scope"`; this generic JSON Pointer prevents Slack-specific
token parsing in the host. The loader refuses a pointer naming obvious credential material, and the
runtime normalizes only the resulting scope list into generation-bound capability evidence.

The four ordinary operations have explicit bot and user auth alternatives with the same named
scope. Their reach still differs because Slack evaluates bot membership for the bot token and the
member's own visibility for the user token. Events remain bot-only; Socket Mode remains app-token
only; Enterprise Admin remains its dedicated org-wide user credential purpose.

## 4. GitLab credential roles

| Connector credential purpose | GitLab credential kind | Subject | Reach |
|---|---|---|---|
| `gitlab.oauth_token` | OAuth access token | user | consenting user's memberships and permissions |
| `gitlab.token` | personal access token | user | token creator's memberships and permissions |
| `gitlab.service_account_token` | PAT belonging to a service account | app | the non-human account's memberships and role |
| `gitlab.group_access_token` | group bot-user token | app | its group and projects, bounded by role |
| `gitlab.project_access_token` | project bot-user token | app | its project, bounded by role |

All five are bearer-compatible, and GitLab deliberately gives personal, service-account, group,
and project access tokens the same `glpat-` prefix. The protected Connect Session therefore asks
which credential purpose is being connected; neither prefix inspection nor a successful request may
reclassify it. OAuth requests `api` because the curated surface includes issue creation. Pasted
tokens may carry `read_api` for reads or `api` for reads and writes. Every read declares
`read_api OR api` on each exact credential purpose; the write declares only `api`.

A developer normally chooses **As myself** (OAuth, with PAT as the explicit alternative). A
platform or babelforce automation normally chooses **As automation**, then supplies a service
account, group, or project token according to the intended resource boundary. No client-credentials
grant is invented: GitLab does not offer one for this use case.

The selection is not accepted as proof of subject or scope. During the protected Connect Session,
the Connector calls `GET /user`: `bot = false` is required for a pasted personal token and
`bot = true` for every automation purpose. It observes classic PAT scopes and expiry through
`GET /personal_access_tokens/self`; an OAuth Connection instead uses GitLab's
`GET /oauth/token/info`, whose `scope` and `resource_owner_id` are bound to the granted token. The
group/project/service-account distinction is acquisition provenance, not a claim inferred from the
shared token prefix; GitLab remains the enforcement point for its resource reach. Fine-grained PATs
are not admitted by the initial classic-scope predicate: their operation/resource permission model
needs a separate reviewed requirement axis rather than pretending a granular permission is an OAuth
scope.

## 5. Capability evidence belongs to the Connection, not the catalog

The provider declares requirements. A Connection records what its current credential generation
has actually been granted, as value-free metadata:

```text
CredentialCapabilityEvidence
  credential purpose
  credential generation / lease
  subject
  granted scopes
  provider installation context (workspace and/or enterprise identifiers)
  acquisition or observation source
  observed_at and expiry/staleness bound
```

The record contains no token or credential-store address. It is replaced atomically with the
credential generation and invalidated on reauthorization, refresh, revocation, or upstream
`invalid_auth`, `token_revoked`, `token_expired`, or `missing_scope` evidence.

Evidence sources are deliberately closed:

- OAuth acquisition records the granted bot/user scopes and installation context from the
  authenticated token response. Enterprise admin capability additionally requires the
  organization-wide installation context; an `admin.*` string alone is insufficient.
- A protected pasted-token flow may probe Slack from within Connector custody. It can use
  `auth.test` for identity/context and the authenticated response's `x-oauth-scopes` header for
  granted scopes. If the required subject or Enterprise context cannot be proven, the corresponding
  members do not surface.
- The app-level token's capability is recorded only after its protected acquisition and a bounded
  `apps.connections.open` verification path; it is never treated as a Web API bot/user token.
- A pasted GitLab token is observed through `/user` and `/personal_access_tokens/self`; OAuth uses
  `/user` and `/oauth/token/info`. Subject, granted classic scopes, expiry, and resource associations
  are recorded from those authenticated responses. An unprovable actor kind or a fine-grained-only
  permission set remains connected-but-not-callable for the affected operations.

Configuration, a human assertion, a model claim, a token prefix, or the set of scopes the
Integration requested are not granted-scope evidence.

## 6. One predicate at discovery and dispatch

For each auth alternative, runtime admission requires:

1. every named credential exists at the current Connection generation;
2. every credential's required scope expression is satisfied by current evidence;
3. subject and provider installation context satisfy that credential purpose; and
4. the Connection, initiator, caller, and Grant gates also admit the member.

Search, catalog projection, tool description, event selection, and channel activation all use this
same predicate before returning a member. Invocation re-evaluates it against the same or newer
Connection generation immediately before credential placement. Cached discovery results are
generation-bound; a scope or credential change invalidates them.

This is fail-closed in both places. Filtering only at invocation leaks unusable tools into a model
and creates predictable failures. Filtering only at discovery leaves a time-of-check/time-of-use
authorization gap.

## 7. Slack source and initial curated surface

Slack's hosted Swagger 2 Web API document is retained byte-for-byte as source evidence. A
deterministic script derives two reference-closed OpenAPI 3 projections:

- four ordinary operations: post a message, read public-channel history, look up a user, add a
  reaction;
- four read-only Enterprise Admin operations: list app requests, search conversations, list
  workspaces, list users.

No Admin write is selected. All eight selected operations explicitly opt into agent exposure;
none are inferred from the other methods in the source.

Slack does not publish the selected Events API payloads as a maintained AsyncAPI artifact. The
repository therefore owns a small AsyncAPI 3 document, labels it `repository-authored`, cites the
official event references, and selects only `app_mention` and the narrowed stable event
`message.channels`. The latter maps Slack's coarse wire discriminator `message` plus
`channel_type = channel` to a name that states the subscribed event family.

Socket Mode and Events API webhook bindings carry the same two stable events, but their transport
authority differs: Socket Mode requires the app-level token with `connections:write`; the webhook
requires the signing secret for HMAC verification. Both event subscriptions independently require
the bot token scopes declared on their events.

## 8. Implementation boundary

Implemented here:

- exact Slack source vendoring and deterministic curated projections;
- minimal AsyncAPI 3 component-message ingestion with select-none defaults;
- credential-local scope requirements on operations, events, and channels;
- separate Slack bot-install and delegated-user OAuth declarations, including Slack's comma scope
  encoding, nonstandard granted-scope response pointer, and user-centric endpoint;
- explicit GitLab user and automation credential purposes with read/write scope requirements;
- canonical document, site catalog, schema, lock, and pack propagation;
- stable `message.channels` normalization in the personal Socket Mode runtime.

Still required in the generic M3 Connection runtime before claiming scope-aware Web/Admin
invocation:

- protected multi-credential Slack and GitLab acquisition, including OAuth callback custody and
  Slack admin installation context;
- persisted value-free `CredentialCapabilityEvidence` tied to credential generations;
- one shared capability-admission predicate wired into discovery and dispatch;
- reauthorization/refresh invalidation and authenticated scope drift handling.

The personal Socket Mode alpha already keeps `slack.app_token` in Connector custody and uses it
only on the fixed ticket-minting call. That is implementation evidence for the transport boundary,
not a claim that generic bot/user/admin scope admission has landed.
