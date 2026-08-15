# Design 09: catalog curation and credential capability admission

**Status:** catalog/compiler contract implemented; Connection capability admission remains M3 work ·
**Date:** 2026-08-15

This document fixes two related boundaries exposed by the Slack connector:

1. a large vendor specification is evidence, not an agent tool list; and
2. possession of a credential is not proof that the credential can perform every operation that
   accepts its wire format.

The rules are generic. Slack is the proving provider because its Web API, Events API, Admin API,
Socket Mode, and app-management surfaces use several superficially similar bearer credentials with
different subjects and scopes.

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
| `slack.user_token` | delegated user OAuth token, commonly `xoxp-…` | user | none selected initially; reserved for explicitly reviewed act-as-user operations |
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

## 4. Capability evidence belongs to the Connection, not the catalog

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

Configuration, a human assertion, a model claim, a token prefix, or the set of scopes the
Integration requested are not granted-scope evidence.

## 5. One predicate at discovery and dispatch

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

## 6. Slack source and initial curated surface

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

## 7. Implementation boundary

Implemented here:

- exact Slack source vendoring and deterministic curated projections;
- minimal AsyncAPI 3 component-message ingestion with select-none defaults;
- credential-local scope requirements on operations, events, and channels;
- canonical document, site catalog, schema, lock, and pack propagation;
- stable `message.channels` normalization in the personal Socket Mode runtime.

Still required in the generic M3 Connection runtime before claiming scope-aware Web/Admin
invocation:

- protected multi-credential Slack acquisition, including OAuth and admin installation context;
- persisted value-free `CredentialCapabilityEvidence` tied to credential generations;
- one shared capability-admission predicate wired into discovery and dispatch;
- reauthorization/refresh invalidation and authenticated scope drift handling.

The personal Socket Mode alpha already keeps `slack.app_token` in Connector custody and uses it
only on the fixed ticket-minting call. That is implementation evidence for the transport boundary,
not a claim that generic bot/user/admin scope admission has landed.
