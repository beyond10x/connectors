---
format: aep.planning-md/3
id: story:oauth2-grant-core
kind: story
status: implemented
title: A provider-neutral OAuth2 client library runs code+PKCE, device, exchange and refresh grants
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: crates/connectors-build/Cargo.toml
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/tests/oauth2_declaration_mirrors_ess.rs
- confidence: cited
  path: crates/connectors-oauth2
- confidence: cited
  path: website/docs/reference/crates.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:53Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:54Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-26T01:30:38Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

A new library crate `crates/connectors-oauth2` runs the client side of OAuth 2.0 for any provider whose
facts arrive as a `ServerDeclaration`: the authorization-code grant with PKCE S256, the RFC 8628 device
grant, the token exchange and the refresh exchange, each against a fake authorization server on loopback.
It holds no provider name, endpoint or client id.

## Model

`ServerDeclaration`, `Parameter`, `ClientKind`, `RequestEncoding`, `RedirectKind`, `Rotation` and `GrantType` are the
types of the same names in `ess/domains/oauth2.yaml` (`connectors.oauth2`). The crate's Rust types mirror
them field for field and variant for variant (serde names equal the ESS names).

## Design

- PKCE: verifier of 43–128 characters from a CSPRNG, `S256` challenge (RFC 7636 §4.1–4.2). `state`: 32
  random bytes, base64url; compared in constant time. Both are `Zeroizing` and never logged.
- Authorize URL: `response_type=code`, `client_id`, `redirect_uri`, `scope` (space-joined
  `requested_scopes`), `code_challenge`, `code_challenge_method=S256`, `state`, plus the declaration's
  `authorize_parameters` in declared order. A declaration may not set any of the preceding names there.
- Redirect receivers: `loopback_any_port` (bind `127.0.0.1:0`), `loopback_fixed_port` (bind the
  declared port, path from `redirect_uri`), and `pasted_code` (parse `code#state` as one line the user
  pastes). The loopback receiver keeps the guarantees of `crates/connectors-host/src/local/oauth.rs`
  (first request on the path decides, 2 s per connection, 64 KiB head bound, 64 pending connections);
  this story copies none of that file's Google rules.
- `redirect_uri`: for `loopback_any_port` the declaration's `redirect_uri` is a template whose host
  (`127.0.0.1` or `localhost`) and path are kept and whose port is the bound one; absent, it is
  `http://127.0.0.1:<port>/`. `state_in_exchange: true` adds `state` to the code exchange body (a
  non-standard requirement of one provider); `false` sends the RFC 6749 §4.1.3 fields only.
- Token and refresh requests: `form` (`application/x-www-form-urlencoded`) or `json`; `public` clients
  send `client_id` and no secret, `confidential` clients add the secret the caller supplies. HTTPS only;
  the transport is a trait the host implements, so tests can trust a fake server's root.
- Responses: bounded body (64 KiB), `access_token` required, `token_type` bearer (case-insensitive),
  `expires_in` required and positive. A refresh response that omits `refresh_token` keeps the old one; one
  that omits `scope` keeps the previous granted scopes; one that carries a new `refresh_token` returns it
  as rotated. `required_scopes` missing from the granted set is `insufficient_scope`.
- Failures, one closed enum: `invalid_grant`/`invalid_client`/401 → `reauthorization_required`; 429 →
  `rate_limited` (with `Retry-After` when sent); 5xx and transport errors → `unavailable`; `access_denied`
  → `access_denied`; a `state` mismatch → `state_mismatch`. No provider body text enters a failure.
- Device grant (RFC 8628 §3.1–3.5): request at `device_authorization_url`; poll the token endpoint at
  `interval` (default 5 s); `authorization_pending` waits; `slow_down` adds 5 s; `expired_token` and
  `access_denied` end the grant; the caller's cancellation ends it within one interval. Non-RFC device
  dialects are a trait an adapter implements; the core ships only RFC 8628.

## Acceptance

`cargo test -p connectors-oauth2` passes, and these tests exist and run against a TLS fake authorization
server on `127.0.0.1` (as `adapters/catalog/tests/local_runtime/oauth2_refresh.rs:261-330` does):

- `pkce_challenge_matches_rfc7636_appendix_b`
- `authorize_url_carries_declared_parameters_in_order`, `extra_parameter_cannot_override_a_core_parameter`
- `public_client_exchange_sends_no_secret_form`, `public_client_exchange_sends_no_secret_json`,
  `confidential_client_exchange_sends_the_secret`
- `fixed_port_receiver_answers_the_declared_path_only`, `any_port_receiver_binds_loopback`,
  `pasted_code_with_wrong_state_is_refused`, `second_request_does_not_change_the_outcome`
- `exchange_carries_state_only_when_declared`
- `refresh_returns_a_rotated_token`, `refresh_without_refresh_token_keeps_the_old_one`,
  `refresh_without_scope_keeps_granted_scopes`, `missing_required_scope_is_insufficient_scope`
- `invalid_grant_is_reauthorization_required`, `status_429_is_rate_limited_with_retry_after`,
  `status_503_is_unavailable`, `oversized_response_is_refused`, `failure_carries_no_response_text`
- `device_grant_waits_on_pending_and_slows_down_by_five_seconds`, `device_grant_expired_token_ends`,
  `device_grant_access_denied_ends`, `device_grant_cancellation_ends_within_one_interval`
- `crates/connectors-build/tests/oauth2_declaration_mirrors_ess.rs`: the crate's `ServerDeclaration`
  serde field names and each enum's variants equal `connectors.oauth2` in the compiled specification.

- Boundary: the gate gains a step (`crates/connectors-build/src/gate.rs`) that refuses any `connectors-*`
  package in `cargo tree --locked -p connectors-oauth2 -e normal` other than the crate itself; a test
  in `crates/connectors-build` shows the step refuses a manifest that adds `connectors-host`.
- Provider-neutral: `rg -n -i 'openai|chatgpt|anthropic|claude|google|auth\.openai|platform\.claude|app_[A-Za-z0-9]{20}' crates/connectors-oauth2/src`
  prints nothing.
- `website/docs/reference/crates.md` is regenerated (`connectors-docs generate`) and names the crate.

`cargo run --locked -p connectors-build -- gate` passes on the merge.

## Out of scope

The host's use of the crate (later stories); any provider declaration; changing
`crates/connectors-host/src/local/oauth.rs` (`story:google-consent-on-the-oauth2-core`).
