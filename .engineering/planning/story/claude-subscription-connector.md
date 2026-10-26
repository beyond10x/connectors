---
format: aep.planning-md/3
id: story:claude-subscription-connector
kind: story
status: implemented
title: A Claude plan connects through the Claude Code sign-in, if the operator permits it
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
  path: adapters/catalog/providers/claude-subscription
- confidence: cited
  path: crates/connectors-build/tests/oauth2_declaration_mirrors_ess.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: inferred
  path: crates/connectors-oauth2
- confidence: cited
  path: ess/domains/oauth2.yaml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:56Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:57Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-26T08:39:57Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

A catalog connector declares the Claude subscription sign-in as an `oauth2_grant` profile, so the operator
connects their own Claude plan once and a caller gets a current access token with `connections token`.
Built for the operator's own single-user use: `decision-blocker:claude-subscription-terms`, option B
(Timo, 2026-10-25). The profile ships no client id; `oauth2_client_id` is the instance's configuration.

## Provider facts

CC = the published Claude Code 2.1.310 binary (bundled JavaScript read with `grep -a`, 2026-10-25); the
grep target is quoted.

| Fact | Value | Source |
|---|---|---|
| client id | `9d1c250a-e61b-44d9-88ed-5944d1962f5e` (registered to Claude Code) | CC `CLIENT_ID:"9d1c250a-` |
| authorize | `https://claude.com/cai/oauth/authorize` | CC `CLAUDE_AI_AUTHORIZE_URL:` |
| token | `https://platform.claude.com/v1/oauth/token` | CC `TOKEN_URL:` |
| authorize extras | `code=true` | CC `function wVr({codeChallenge` |
| scopes | `user:profile user:inference` minimum for a token used for inference (Claude Code requests also `org:create_api_key user:sessions:claude_code user:mcp_servers user:file_upload user:plugins`) | CC `var c="org:create_api_key"`; `fk="user:inference"` |
| redirect | loopback any port `http://localhost:<port>/callback`, or pasted `code#state` from `https://platform.claude.com/oauth/code/callback` | CC `MANUAL_REDIRECT_URL:`, `let[h,p]=c.split("#")` |
| exchange | **JSON** `{grant_type, code, redirect_uri, client_id, code_verifier, state}` (`state` in the body is non-standard) | CC `async function Tqo(` |
| refresh | JSON `{grant_type, refresh_token, client_id, scope}`; an omitted `refresh_token` keeps the old one | CC `async function koe(`, `{access_token:K,refresh_token:j=e,expires_in:Z}` |
| refresh lead | 5 minutes (`n+300000>=e`) | CC `function rC(e,n=Date.now())` |
| identity | `account{uuid,email_address}` in the token response; `GET https://api.anthropic.com/api/oauth/profile` | CC `formatTokens(e,` |
| use | `Authorization: Bearer` plus `anthropic-beta: oauth-2025-04-20` | CC `Bu="oauth-2025-04-20"` |
| device grant | none offered | CC: no `device_code` in the OAuth module |
| predecessor | v1 implemented this flow | `crates/subscription-custody/src/lib.rs:31-33` on `compat/claude-oauth-0-7-source` |

Terms: <https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use>
(verbatim): "developers may not collect, store, or intermediate Claude.ai credentials or session tokens —
sign-in to a Claude account must complete through Anthropic's own flow." The same page permits an end user
"signing in to the unmodified Claude Code binary with their own Claude subscription".

## Design

- `adapters/catalog/providers/claude-subscription/connector.json`: profile `claude.subscription`,
  `oauth2_grant`, `request_encoding: json`, `refresh_encoding: json`, redirect `loopback_any_port` with
  `pasted_code` as the fallback, `rotation: rotating`, `refresh_lead_seconds: 300`.
- The non-standard `state` in the exchange body is `state_in_exchange: true` (`connectors.oauth2.ServerDeclaration`, added with `story:oauth2-grant-core`).
- Guide page with the eight onboarding sections, README row, CHANGELOG entry. The guide quotes the terms
  above verbatim with their URL and the date read, and says the profile is for the operator's own plan.

## Acceptance

- Journeys against a fake server answering the shapes above: `claude_connect_loopback_journey`,
  `claude_connect_pasted_code_journey`, `claude_token_refreshes_with_json`.
- The journeys run in `cargo run --locked -p connectors-build -- ignored --required`, whose report names
  each of them as passed (an ignored or skipped journey does not count).
- `claude_profile_without_client_id_is_invalid_configuration`.
- `rg -n "may not collect, store, or intermediate" website/docs/connectors/claude-subscription.md` prints the quote.
- `connectors-docs vocabulary` and the onboarding test pass.

## Out of scope

Shipping a default client id (`oauth2_client_id` is configuration); operations against `api.anthropic.com`; a live login.
