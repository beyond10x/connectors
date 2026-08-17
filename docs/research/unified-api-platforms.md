# Domain models of unified-API / embedded-iPaaS platforms

**Compiled 2026-08-13** from official documentation of Nango, Merge.dev, Composio, Apideck Unify
and Paragon (priority order), with brief looks at Pipedream Connect and Arcade.dev. All claims are
from official docs unless marked as third-party opinion; source URLs at the end. Kong/Apigee were
confirmed out of category: they are API management gateways for **your own** APIs (publishing,
rate-limiting, monetizing), not brokers for end-user-authorized access to third-party vendor APIs.

Companion artifacts: the primary sources mined for [catalog-precedents.md](catalog-precedents.md)
are vendored under [vendor/](vendor/) with provenance.

## 1. Comparative entity table

| Concept | Nango | Merge.dev | Composio | Apideck Unify | Paragon |
|---|---|---|---|---|---|
| Customer org | Account → **Environments** (dev/prod; per-env secret keys; all state env-scoped) | "Your organization" (Test/Production API keys, Bearer) | Project (dashboard) | **Application** (`x-apideck-app-id` + API key) | **Project** (JWT `aud` = `useparagon.com/{project-id}`) |
| End user | End-user **tags** on connection: `end_user_id`, `end_user_email`, `organization_id` | **End user** (`end_user_origin_id`, org name, email at link-token creation) | **`user_id`** (your ID; formerly "Entity") | **Consumer** (`x-apideck-consumer-id`, your ID, upserted; optional metadata for Vault UI) | **Connected User** (JWT `sub` claim) |
| Provider template | **Provider** (entry in `providers.yaml`, ~950+ APIs) | **Integration** | **Toolkit** (formerly "App") | **Connector** (`x-apideck-service-id`, kebab-case) | **Integration** (130+ connectors) |
| Org-level configured provider | **Integration** (provider config key + your OAuth creds) | Dashboard integration config (optional partner credentials) | **Auth Config** (`ac_…`; formerly "Integration") | Connector enabled for app (state `available`) + optional own client credentials | Integration config in dashboard (your OAuth app) |
| Per-user instance | **Connection** (+ `connection_config`, metadata) | **Linked Account** (statuses `COMPLETE`/`INCOMPLETE`/`RELINK_NEEDED`/`IDLE`) | **Connected Account** (`ca_…`; states INITIALIZING/INITIATED/ACTIVE/FAILED/EXPIRED/INACTIVE) | **Connection** (states `available`→`added`→`authorized`→`callable`) | Connection backed by a **Credential** (multi-account via `X-Paragon-Credential` header) |
| Credential object client holds | None — backend fetches creds via `GET /connections/{id}` or uses proxy | Permanent **`account_token`** (`X-Account-Token` header) + org API key | None — calls scoped by `user_id`; creds stay in Composio | API key + header triple; Vault holds vendor creds | RS256 **Paragon User Token** (JWT signed by *you*) used for SDK, Proxy, ActionKit |
| Permission scoping | OAuth scopes per integration; `allowed_integrations` per connect session | **Common Model Scopes** + **field-level scopes**; Default vs per-Linked-Account, read/write | Scopes on auth config; per-session toolkit/tool restriction | Per-connector config, unified-API scope | Workflow/feature toggles + User Settings per user (no field-level scopes) |
| Webhooks | To customer: auth/sync/forwarded events, HMAC-SHA256; from provider: `webhook_routing_script` per provider | Dual: Merge→you (`LinkedAccount.*`, `{Model}.added/changed/removed`) ; provider→Merge receivers (auto or manual) | **Triggers** (`ti_…`): provider events → webhook to project URL (Svix-style signing) or WebSocket/SDK listener | Unified webhook events per API (`x-apideck-event-type`); provider webhooks auto-registered or **polling engine** fallback | Integration Triggers / Custom Webhooks (auto-registered); **ActionKit Triggers** (one URL for all users); Event Destinations for ops events |
| Invocation | **Actions**, **syncs** (records cache), **proxy**, MCP | Unified REST per category + **(Async) Passthrough** | `tools.execute`, session **meta-tools** (e.g. `COMPOSIO_SEARCH_TOOLS`), MCP, `proxyExecute` | Unified REST + **Proxy API** (`x-apideck-downstream-url`) | **Proxy API**, **ActionKit** (`POST /projects/{id}/actions`), workflows |

Briefly: **Pipedream Connect** — project + OAuth client / `external_user_id` / **connected
accounts** (`apn_…`); invocation via Connect API (run registry components), Connect Proxy, and a
single remote MCP server that returns a connect-URL when auth is missing. **Arcade.dev** — Engine
holds tokens; tools declare `requires_auth(provider, scopes)`; `tools.authorize(tool, user_id)` →
auth URL → `waitForCompletion()` → `tools.execute`; self-hostable engine (Helm).

## 2. Auth delegation

**Common shape across all five**: backend creates a short-lived session/link token bound to an
end-user identity → frontend component or hosted URL runs the vendor auth → platform stores vendor
credentials and owns refresh forever → customer persists only a stable reference.

- **Nango**: `POST /connect/sessions` (~30-min token; payload: end_user id/email/name,
  organization_id, `allowed_integrations`); frontend SDK `openConnectUI()`; auth webhook returns
  connectionId + mirrored tags for reconciliation. Credentials retrievable server-side,
  auto-refreshed; `invalid_credentials` → **reconnect session** that re-auths in place, preserving
  connection metadata. OAuth apps: Nango-provided shared apps (test only, branded, fixed scopes)
  vs your own client_id/secret; custom callback domains via 308 redirect to
  `api.nango.dev/oauth/callback`.
- **Merge**: 4-step exchange — `POST /create-link-token` → Merge Link component →
  `onSuccess(public_token)` → `GET /account-token/{public_token}` → permanent `account_token`.
  **Magic Link** = hosted URL variant (up to 7 days). Default: Merge's own vendor OAuth apps;
  optional customer partner credentials. Merge owns refresh; failure → `RELINK_NEEDED`.
- **Composio**: `connected_accounts.initiate(user_id, auth_config_id)` → `redirect_url` →
  `wait_for_connection()` polls to ACTIVE; Connect Links can be surfaced by the agent at runtime.
  Composio-managed OAuth apps for dev, own credentials recommended for production; auto token
  refresh, `EXPIRED` only after refresh fails.
- **Apideck**: server-side `POST /vault/sessions` → `session_token` (JWT for embedded Vault JS) +
  `session_uri` (Hosted Vault); connection states advance to `callable`; `auto_redirect` returns
  users to your app. Apideck default OAuth creds with per-connector "use your client credentials"
  flip. Vault = "Managed OAuth", owns refresh.
- **Paragon**: inverted default — **you** register the vendor OAuth app and give Paragon the
  client ID/secret; additionally **User-Configured OAuth** lets each end user bring their *own*
  OAuth client. Identity via RS256 JWT you sign (Paragon never stores your private key); the same
  JWT authenticates frontend SDK, Proxy, and ActionKit.
- **Arcade**: same dual model (Arcade-provided default apps, own provider config "takes
  precedence", recommended for prod).
- **Pipedream**: default pre-approved OAuth clients, but custom clients are **required** for
  sensitive operations (retrieving raw credentials).

Implication for a three-tier product: personal/local and org self-hosted tiers are inherently
BYO-OAuth-app (there is no platform to share apps from). Nango's custom-callback-domain redirect
trick shows how a SaaS tier can still offer shared apps while keeping catalog entries portable.

## 3. Webhook / event architecture patterns

**Provider → platform (ingestion)**
- Nango: one platform webhook URL registered in the vendor portal; a per-provider
  `webhook_routing_script` (declared in providers.yaml, code in repo) does two jobs: verify
  origin/signature and attribute the payload to a connection; then forward to customer and/or
  trigger webhook functions/syncs.
- Merge: provider webhooks as a **sync accelerator** — auto-created in supported vendor APIs on
  initial sync, or manually configured by the end user with a per-Linked-Account
  `webhook_listener_url` + signature key.
- Apideck: auto-registration where supported, manual paste for some (QuickBooks documented), and a
  **proprietary polling engine** that synthesizes unified webhook events for providers without
  webhooks.
- Paragon: Custom Webhooks auto-register with the provider when a user enables the feature, using
  a Paragon-managed `{{settings.webhookURL}}`.

**Platform → client (delivery)** — all push webhooks; only Composio offers a pull-ish channel
(WebSocket/SDK listener for dev, CLI forwarding to localhost). Signature schemes observed:

| Platform | Header(s) | Scheme |
|---|---|---|
| Nango | `X-Nango-Hmac-Sha256` (legacy `X-Nango-Signature`) | HMAC-SHA256 over raw body, dedicated webhook signing key; 3 attempts, exponential backoff, 20s timeout; 2 URLs/env + per-connection `webhook_url_override` |
| Merge | `X-Merge-Webhook-Signature` | HMAC-SHA256 over raw bytes, dedicated regenerable key, Base64url, constant-time compare prescribed |
| Composio | `webhook-id`, `webhook-timestamp`, `webhook-signature` | Svix-convention: HMAC-SHA256 over `{id}.{timestamp}.{rawBody}` — id enables dedup, timestamp defeats replay |
| Apideck | `x-apideck-signature` | HMAC-SHA256 with **your API key as secret** over a payload with recursively alphabetically sorted keys — two anti-patterns (key coupling, canonicalization fragility) |
| Paragon | ActionKit Triggers: one URL "on behalf of all of your users"; Event Destinations separate operational events (workflow/credential failures) from data events | — |

Dedup/replay: none of the five documents a first-class customer-facing replay/redrive API; Merge
explicitly advises a polling fallback "because webhooks can fail"; delivery logs exist (Merge,
Nango). A documented gap worth exploiting.

## 4. Catalog-as-text precedents

**Nango `providers.yaml`** (vendored: [vendor/nango-providers.yaml](vendor/nango-providers.yaml),
957 entries at the pinned commit): one public YAML file declares every provider.

- Identity: `display_name`, `categories`, `docs`, `alias` (inherit/extend another provider).
- Auth template: `auth_mode` ∈ {API_KEY, APP, APP_STORE, BASIC, NONE, OAUTH1, OAUTH2, OAUTH2_CC,
  CUSTOM, TBA, JWT, BILL, TWO_STEP, SIGNATURE, AWS_SIGV4, MCP_OAUTH2, MCP_OAUTH2_GENERIC,
  INSTALL_PLUGIN}; `authorization_url`, `token_url`, `refresh_url`, `authorization_params`,
  `token_params`, `default_scopes`, `scope_separator`, `disable_pkce`, `authorization_method`,
  `body_format`, `token_expiration_buffer`.
- Per-connection inputs: `connection_config` (typed, regex-validated user fields, e.g.
  `instance_url`), `connection_configuration` (auto-populated post-connection), `credentials`
  (field defs for API_KEY/BASIC), `token_response_metadata` (extract extra OAuth response fields
  into `connection.metadata` — e.g. Slack's `incoming_webhook.url`, `bot_user_id`).
- Runtime metadata: `proxy.base_url` (interpolates `${connectionConfig}`), `proxy.headers`,
  `proxy.retry` (rate-limit headers `after`/`at`), `proxy.paginate` (cursor / offset / link),
  `proxy.verification` (a probe endpoint to validate credentials).
- Eventing hooks: `webhook_routing_script`, `webhook_user_defined_secret`,
  `post_connection_script`.

Key observation: auth + proxy metadata are pure data; only webhook routing and post-connection
steps escape to named scripts. **Operations/endpoints are NOT in providers.yaml** — Nango's
actions/syncs are TypeScript functions in a separate templates repo. Our catalog goes further by
declaring operations as text too.

**Airbyte declarative manifest** (vendored:
[vendor/airbyte-source-pokeapi-manifest.yaml](vendor/airbyte-source-pokeapi-manifest.yaml)): the
strongest precedent for *operations*-as-text — `version`/`definitions`/`streams`/`spec`/`check`;
each stream declares a requester (`url_base`, authenticator, error handler), record_selector,
paginator, incremental_sync cursor, partition_router, transformations; interpreted by a generic
runtime (`source-declarative-manifest`). Proof that a YAML-interpreted connector runtime scales to
hundreds of maintained connectors — read-side only.

**Others**: Apideck's Connector API (vendored:
[vendor/apideck-connector-api.yml](vendor/apideck-connector-api.yml)) exposes the catalog as
queryable data (connector list, per-resource schemas, coverage matrices) rather than contributable
text. Pipedream's public registry is **real Node.js code** (vendored:
[vendor/pipedream-github-create-issue.mjs](vendor/pipedream-github-create-issue.mjs)) — a
counter-example showing the cost: code components need their serverless runtime, so self-hosting
the catalog means self-hosting an execution platform. Arcade's tool SDK is Python decorators, also
code.

## 5. Design patterns worth adopting (attributed)

1. **Three-layer provider model: template → org-configured integration → per-user connection**
   (Nango provider/integration/connection; Composio toolkit/auth-config/connected-account).
   Cleanly separates what's in the shared text catalog, what a deployment owner configures, and
   per-user state. Maps 1:1 to our catalog / deployed service / user connections across tiers.
2. **Auth templates and proxy runtime metadata as pure data in one public text file** (Nango,
   extended by Airbyte for operations). Adopt the `verification` probe and
   `token_response_metadata` specifically; both remove per-provider code.
3. **Server-created short-lived connect session + reconciliation tags + embeddable/hosted/headless
   connect surfaces** (Nango, Merge link_token/Magic Link, Apideck sessions, Paragon headless).
   Keeps org API keys out of browsers; tags solve "which of my users just connected"; hosted-URL
   and headless variants cover CLI/agent contexts.
4. **Reauthorize-in-place with explicit connection states** (Apideck `added→authorized→callable`;
   Nango reconnect sessions; Merge `RELINK_NEEDED`). Model expiry as a state transition with a
   repair flow, never delete-and-recreate — connection ids referenced by clients and event
   subscriptions must be stable.
5. **Dual credential-ownership with BYO as the portable default** (all seven platforms). Catalog
   entries define the auth template; client_id/secret live in the deployment. Personal/org tiers
   are BYO-only; a SaaS tier can add shared apps later via the custom-callback-domain trick so
   connections stay portable between tiers.
6. **Svix-style webhook envelope: `webhook-id` + `webhook-timestamp` + HMAC over
   `{id}.{timestamp}.{body}` with a dedicated signing key** (Composio). Dedup and replay-attack
   protection for free. Explicitly avoid Apideck's API-key-as-secret and sorted-keys
   canonicalization.
7. **Per-provider webhook routing declared in the connector** (Nango). Provider webhook signature
   verification and payload→connection attribution are connector concerns; declaring them in the
   catalog (declarative match rules with a script escape hatch) is what makes "the platform
   terminates inbound events" scale across providers.
8. **Auth-as-tool-result for agents** (Arcade authorize→waitForCompletion; Pipedream MCP returning
   a connect URL; Composio Connect Links mid-conversation). "Not connected" should be a structured
   response containing a connect URL the agent can hand to a human — not an error. The single most
   agent-native pattern found.
9. **Meta-tools instead of loading the whole catalog into model context** (Composio sessions'
   ~7 meta-tools; Paragon ActionKit `format=json_schema`). Project search/inspect/execute
   meta-tools (and an MCP endpoint backed by the same session) rather than N tool schemas.
10. **Authenticated proxy as the universal escape hatch, with normalized downstream limits**
    (Merge Passthrough incl. async; Nango proxy with catalog-driven retry/pagination; Apideck
    `x-downstream-ratelimit-*`; Pipedream Connect Proxy). Declared operations + proxy is the
    invocation layer; normalizing rate-limit/pagination from catalog metadata is what makes the
    proxy better than curl.
11. **Separate operational events from data events** (Paragon Event Destinations vs ActionKit
    Triggers). Credential-expiry and delivery-failure events deserve their own channel.

## 6. Pitfalls to design against

1. **Per-connection/per-identity pricing distorts the category** — Merge: free 3 Linked Accounts,
   then $650/mo for 10, $65/Linked-Account/mo after; Paragon meters every Proxy/ActionKit call as
   a billable "task" with five-figure annual minimums. Competitors' marketing is built on this
   resentment. Never meter identities or connections in the open tiers; if SaaS meters anything,
   meter infra-shaped costs.
2. **Open-core self-hosting with a crippled or unclear free tier** — Nango's free self-host covers
   only auth+proxy (functions/syncs/webhooks/MCP are Enterprise; the boundary is undocumented —
   open issue #5536); Composio's token-holding backend is closed source; Paragon on-prem is
   Enterprise-sales-only. Identical feature set across tiers, differing only in operational scope,
   is the category's most conspicuous open flank.
3. **Common-model lossiness, with the escape hatches paywalled** — Merge ships three compensators
   (remote_data, Field Mapping, Passthrough), all Professional/Enterprise-gated; Apideck has ~14
   auth-only categories with no unified model. No-unified-models-v1 sidesteps the lossiness; the
   lesson: raw vendor access must be the free default, never the premium tier.
4. **Webhook reliability theater** — Apideck synthesizes "webhooks" from a polling engine; Merge
   officially advises a 24h polling fallback; none offers customer-facing replay/redrive. Design:
   durable per-client delivery queues, a replay-by-id API, honest event provenance
   (native-webhook vs polled) in the envelope.
5. **Centralized token store = concentrated blast radius** — the May 2026 Composio breach
   (attacker escalated via an internal agentic tool into the tool-execution sandbox). The tier
   model is itself the mitigation — personal/org tiers keep tokens off shared infrastructure; for
   SaaS, per-org envelope encryption and scoped execution isolation are table stakes.
6. **Terminology churn burns integrators** — Composio has three doc generations
   (entity/apps/actions → user_id/toolkits/tools → sessions/meta-tools), leaving most in-the-wild
   examples stale. Pick nouns once; version the catalog schema, not the vocabulary.

## Key sources

Nango: https://nango.dev/docs/reference/api-configuration · https://nango.dev/docs/guides/auth/auth-guide ·
https://nango.dev/docs/guides/platform/webhooks-from-nango ·
https://nango.dev/docs/implementation-guides/use-cases/webhooks-from-external-apis ·
https://nango.dev/docs/guides/platform/environments · https://nango.dev/docs/guides/platform/self-hosting ·
https://github.com/NangoHQ/nango · https://github.com/NangoHQ/integration-templates
Merge: https://docs.merge.dev/merge-unified/concepts · https://docs.merge.dev/merge-unified/merge-link ·
https://docs.merge.dev/merge-unified/reading-data/webhooks/overview ·
https://help.merge.dev/en/articles/6074068-linked-account-statuses-and-the-account-details-endpoint ·
https://www.merge.dev/pricing/unified
Composio: https://docs.composio.dev/docs/quickstart · https://docs.composio.dev/docs/authenticating-tools ·
https://docs.composio.dev/docs/using-triggers · https://docs.composio.dev/docs/sessions-via-mcp
Apideck: https://developers.apideck.com/get-started · https://developers.apideck.com/guides/vault ·
https://developers.apideck.com/guides/connection-states · https://developers.apideck.com/guides/webhooks ·
https://developers.apideck.com/apis/connector/reference
Paragon: https://docs.useparagon.com/actionkit/overview ·
https://docs.useparagon.com/getting-started/installing-the-connect-sdk ·
https://docs.useparagon.com/connect-portal/headless-connect-portal ·
https://docs.useparagon.com/resources/user-configured-oauth ·
https://docs.useparagon.com/resources/custom-webhooks
Pipedream / Arcade / Airbyte: https://pipedream.com/docs/connect ·
https://pipedream.com/docs/connect/api-proxy · https://docs.arcade.dev/en/home/auth/auth-tool-calling ·
https://docs.arcade.dev/en/home/auth-providers ·
https://docs.airbyte.com/platform/connector-development/config-based/low-code-cdk-overview
Third-party (opinions, mostly competitor-authored): https://nango.dev/blog/merge-pricing/ ·
https://nango.dev/blog/paragon-pricing/ · https://p0.dev/blog/the-composio-breach-lets-stop-blaming-the-agents/
