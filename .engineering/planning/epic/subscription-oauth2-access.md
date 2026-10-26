---
format: aep.planning-md/3
id: epic:subscription-oauth2-access
kind: epic
status: active
title: Subscription access through regular OAuth2, with an always-current access token
summary: Provider-neutral OAuth2 grants, rotating refresh in custody and a verb that issues a current access token; ChatGPT and Claude as the first providers.
relations:
- serves: vision:independent-contract-adapters
- informed_by: architecture-decision-record:oauth-material-as-static-entry
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:57Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:57Z", actor: "human:timo", revision: 5}
---
## Outcome

A person connects a subscription account once through a regular OAuth 2.0 grant (authorization code with
PKCE S256, or a device grant where the provider offers one). Connectors keeps the access token and the
refresh token in its existing custody, refreshes a rotating grant under the host's refresh ledger, and
hands a caller a currently valid access token through the `connectors` CLI, a Rust library entry or a
consumer launch. No command prints a token unless the caller asks for it by name.

## Operator request

Timo, 2026-10-25, relayed by the directing session (verbatim): "it must be possible in connectors lib, to
do a regular oauth2 thing, for anthropic claude code AND for codex/chatgpt. this way a token and refresh
token would be stored in connectors, and by using connectors lib/sdk or CLI we would be able to get an
always up-to-date (refreshed) token to be used".

Brief constraints from the same relay: provider-neutral OAuth2 core and the storage/refresh verb first; no
live login against either provider; tests use a fake authorization server on loopback; provider facts are
researched from public documentation and the providers' open-source CLIs and cited in the story bodies.

## Model (ESS first)

- New domain `ess/domains/oauth2.yaml` (`connectors.oauth2`): `AuthorizationServer` (the declaration a
  profile carries: endpoints, client kind, request encodings, redirect kind, scopes, rotation, grant
  types, sources) and `AccessTokenIssue` (one hand-out: connection, generation, optional refresh attempt,
  expiry, scopes, delivery; never the token). Validated with the pinned `ess` 0.68.0
  (`ess specify validate --path ess`: "connectors v1 — 26 file(s), valid").
- Reused, not duplicated: the grant run is one `connectors.auth_bindings.Acquisition`
  (`ess/domains/auth_bindings.yaml:248`); rotating refresh is `connectors.refresh.RefreshAttempt`
  (`ess/domains/refresh.yaml`), specified in `contracts/auth/acquisition/v1alpha1/semantics.md` §4.1 and
  not implemented today (no component owns `connectors.refresh`, `ess/components.yaml:5-17`); each
  refreshed material is a new `connectors.credentials.CredentialGeneration`.

## What exists (origin/main d2842557d)

| Concern | State | Source |
|---|---|---|
| Loopback + PKCE consent | Google-only: installed-client JSON, confidential client, form encoding, any-port redirect on `/`, stores `{client_id, client_secret, refresh_token}` | `crates/connectors-host/src/local/oauth.rs:52,76-94,170,474,598` |
| Refresh | catalog child refreshes a **non-rotating** grant in memory and refuses a rotated `refresh_token` | `adapters/catalog/src/local.rs:765-876,838-848`; `architecture-decision-record:oauth-material-as-static-entry` rule 3 |
| Replacing material without a person | none on a writable binding | `crates/connectors-host/src/local/owner/transport.rs:2067-2290` |
| Credential hand-out | only `connections launch` (stored document on fd 3) | `contracts/cli/v1alpha1/consumer-launch.md` |
| Predecessor (v1, `compat/claude-oauth-0-7-source`) | Claude subscription PKCE, manual `code#state`, JSON exchange, refresh with rotation, scope carry-forward | `crates/subscription-custody/src/lib.rs:31-33,627-637` on that branch |

## Provider facts and their limits

Researched 2026-10-25 from Codex CLI source at `openai/codex` commit `970111f6cb62cc02fe36a82ea7d71b37a2889c97`
and the published Claude Code 2.1.310 binary; cited per fact in `story:chatgpt-subscription-connector`
and `story:claude-subscription-connector`.

- Anthropic, <https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use>
  (verbatim): "Anthropic does not permit third-party developers to offer Claude.ai login into their own
  applications, or to route requests through Free, Pro, or Max plan credentials on behalf of their users.
  Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens —
  sign-in to a Claude account must complete through Anthropic's own flow." The operator decided (option B of
  `decision-blocker:claude-subscription-terms`, 2026-10-25): built for the operator's own single-user use.
- OpenAI, <https://developers.openai.com/siwc/quickstart> (verbatim): "ChatGPT plan usage is available to
  all open-source partners and selected private clients." and the open-source flow "registers your client
  and issues OAuth credentials". The Codex CLI's own client id is registered to Codex; the ChatGPT
  connector takes the client id from configuration (`credential-blocker:chatgpt-oauth-client`, cleared).

## Stories

| Story | Delivers | Depends on |
|---|---|---|
| `story:oauth2-grant-core` | provider-neutral OAuth2 client library | — |
| `story:oauth2-refresh-metadata-authority` | `connectors.oauth2` and `connectors.refresh` in the local metadata authority | — |
| `story:owner-rotating-refresh` | owner refreshes every `oauth2_grant` connection through `RefreshAttempt`, new custody version per refresh | core, authority, connect |
| `story:connect-runs-an-oauth2-grant` | `connections connect`/`reconnect` run the grant for an `oauth2_grant` profile and store the token response | core, authority |
| `story:connections-token-issues-an-access-token` | `connections token` and the Rust library entry | owner refresh, connect |
| `story:launch-hands-an-oauth2-access-token` | `connections launch` gives a consumer the access token only | token |
| `story:google-consent-on-the-oauth2-core` | the Google consent flow uses the core's PKCE and receiver | core |
| `story:chatgpt-subscription-connector` | ChatGPT (Codex sign-in) declaration, device dialect, guide | connect, token |
| `story:claude-subscription-connector` | Claude subscription declaration, guide, for the operator's own use | connect, token |

Decision: `architecture-decision-record:oauth2-grants-and-access-token-issue`.

## Out of scope

Live login against either provider; registering client ids; hosted (server) deployments; routing catalog
operations through a subscription token; the coordinator's trusted UI channel of `auth.acquisition`.
