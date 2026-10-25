---
format: aep.planning-md/3
id: story:chatgpt-subscription-connector
kind: story
status: active
title: A ChatGPT plan connects through the Codex sign-in and issues current access tokens
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
- depends_on: story:connect-runs-an-oauth2-grant
- depends_on: story:connections-token-issues-an-access-token
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: adapters/catalog/providers/chatgpt
- confidence: inferred
  path: apps/connectors
- confidence: cited
  path: website/docs/connectors/chatgpt.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:56Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

A catalog connector `chatgpt` declares the ChatGPT sign-in as an `oauth2_grant` profile, so a person
connects their ChatGPT plan once (`connections connect`, browser or `--device`) and a caller gets a current
access token with `connections token`. The client id comes from the instance's configuration. Its guide
says what the token is for and which client id to use.

## Provider facts

S = `https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs` (Codex CLI,
Apache-2.0, read 2026-10-25).

| Fact | Value | Source |
|---|---|---|
| issuer | `https://auth.openai.com` | S/login/src/server.rs#L77 |
| authorize / token | `{issuer}/oauth/authorize`, `{issuer}/oauth/token` | S/login/src/server.rs#L576-L605, #L708 |
| Codex CLI client id | `app_EMoamEEZ73f0CkXaXp7hrann` (registered to Codex; not shipped as a default) | S/login/src/auth/manager.rs#L1784 |
| authorize extras | `id_token_add_organizations=true`, `codex_cli_simplified_flow=true`, `originator=codex_cli_rs` | S/login/src/oauth/authorization.rs#L20-L36; S/login/src/auth/default_client.rs#L42 |
| scope | `openid profile email offline_access api.connectors.read api.connectors.invoke` | S/login/src/server.rs#L597-L601 |
| redirect | `http://127.0.0.1:1455/auth/callback` (fixed port; Codex falls back to 1457) | S/login/src/server.rs#L78-L80, #L629-L670 |
| PKCE | S256; verifier 64 random bytes base64url | S/login/src/oauth/pkce.rs#L15-L29 |
| code exchange | form: `grant_type, client_id, code, redirect_uri, code_verifier` | S/login/src/oauth/client.rs#L79-L95 |
| refresh | **JSON** `{grant_type, client_id, refresh_token}` (+ Codex's `refresh_reason`); no `scope` | S/login/src/oauth/client.rs#L51-L63, #L97-L112 |
| rotation | each returned field overwrites the stored one → `rotation: rotating` | S/login/src/auth/manager.rs#L1647-L1690 |
| refresh lead | 5 minutes before the JWT `exp` → `refresh_lead_seconds: 300` | S/login/src/auth/manager.rs#L214-L215, #L3300-L3328 |
| identity | `id_token` claim namespace `https://api.openai.com/auth`, `chatgpt_account_id` | S/login/src/token_data.rs#L42-L56, #L92 |
| use | `Authorization: Bearer`, `ChatGPT-Account-ID`, base `https://chatgpt.com/backend-api/codex` | S/model-provider/src/bearer_auth_provider.rs#L31-L47; S/model-provider-info/src/lib.rs#L82 |
| device grant (not RFC 8628) | POST `{issuer}/api/accounts/deviceauth/usercode` JSON `{client_id}` → `device_auth_id, user_code, interval`; poll `/api/accounts/deviceauth/token` JSON `{device_auth_id,user_code}` (403/404 = pending, 15 min max) → `authorization_code, code_verifier`; exchange with `redirect_uri={issuer}/deviceauth/callback`; user page `{issuer}/codex/device`; the user enables it in ChatGPT security settings | S/login/src/device_code_auth.rs#L66-L218 |

Terms: <https://developers.openai.com/siwc/quickstart> (verbatim): "ChatGPT plan usage is available to all
open-source partners and selected private clients." The registered-client route is
`credential-blocker:chatgpt-oauth-client`.

## Design

- `adapters/catalog/providers/chatgpt/connector.json`: one `oauth2_grant` profile `chatgpt.subscription`
  with the facts above, `client_id` absent (configuration `oauth2_client_id`), each value's source in
  `sources`.
- The device dialect is an adapter-local implementation of the core's device trait (it returns a code
  and verifier for the ordinary exchange); composition in `apps/connectors` pairs it with the profile.
  The core stays RFC 8628 only.
- The ChatGPT account id is surfaced in `connections describe` as the connection's external identity.
- Guide `website/docs/connectors/chatgpt.md` with the eight onboarding sections
  (`crates/connectors-docs/src/onboarding.rs`), the README support-table row, CHANGELOG entry.

## Acceptance

- Journeys against a fake server that answers the shapes above (form exchange, JSON refresh with rotation,
  the deviceauth endpoints): `chatgpt_connect_browser_journey`, `chatgpt_connect_device_journey`,
  `chatgpt_token_refreshes_with_json_and_rotates`; each ends with `connections token --reveal` printing the
  fake server's current token.
- `chatgpt_profile_without_client_id_is_invalid_configuration`.
- `connectors-docs vocabulary` and `every_support_table_connector_has_an_onboarding_page` pass.
- No live call to the provider in any test (the fake server's root is the only trusted root).
- The journeys run in `cargo run --locked -p connectors-build -- ignored --required`, whose report names
  each of them as passed (an ignored or skipped journey does not count).

## Out of scope

Operations against `chatgpt.com/backend-api`; shipping a default client id; a live login.
