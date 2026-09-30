---
format: aep.planning-md/3
id: story:catalog-oauth2-refresh-profile
kind: story
status: implemented
title: The catalog provider refreshes an OAuth access token from a stored refresh token
relations:
- decomposes: epic:google-workspace-reads
- informed_by: architecture-decision-record:oauth-material-as-static-entry
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/Cargo.toml
- confidence: inferred
  path: adapters/catalog/src/local.rs
- confidence: inferred
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/kubernetes/src/local.rs
- confidence: cited
  path: adapters/sql/src/local.rs
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: inferred
  path: crates/connectors-host/src/local/runtime.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/process.rs
- confidence: inferred
  path: docs/local-catalog-provider.md
- confidence: cited
  path: ess/domains/cli.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T13:14:00Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-09-30T13:14:01Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:32Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":5,"verification":1}}}
---
## Outcome

A catalog configuration can declare `auth.scheme: "oauth2_refresh"`. Its protected entry is
`{client_id, client_secret, refresh_token}`; the child exchanges it for an access token and sends
`Authorization: Bearer <access token>`. Decision: `architecture-decision-record:oauth-material-as-static-entry`.

## Design (verified against code 2026-09-30)

- `adapters/catalog/src/local.rs`: new `Scheme` variant (today `Token | Basic`, `:46-52`);
  `AuthConfig` (`:63-81`) gains `token_url` (https only) and `identity.source: api | id_token`
  (default `api`, skipped when serialized so existing configuration revisions keep their bytes);
  new `OAuthEntry` with `deny_unknown_fields`, each field checked by the existing token validator
  (`:121-129`); the profile advertises the three fields (`:306-325`) and stays
  `http_bearer` / `http-bearer` on the wire (`crates/connectors-host/src/local/runtime.rs:348-354`).
- Token client: a second `ScopedHttp` with `ca = None` (a configured `ca_file` becomes
  `tls_certs_only`, `crates/connectors-host/src/http.rs:128-135`), base = scheme + host of
  `token_url` with a trailing `/`, path segments from `token_url` (`canonical_base` appends `/`,
  `http.rs:45-47,99-101`).
- `crates/connectors-host/src/http.rs`: new form-encoded POST with a zeroized body, no credential
  header, bounded like the existing JSON writes (`:263-287`).
- Token trust: `auth.token_ca_file` (optional; default absent = platform roots). Tests set it to the
  fixture CA so the TLS fixture's token route is reachable; production configurations omit it.
- Acquisition metadata for the host: `AuthConfig` also carries `authorize_url` (https only) and
  `scopes` (non-empty list). The adapter-to-host `Profile` (`adapters/catalog/src/local.rs:335-345`;
  the type in `crates/connectors-host/src/local/runtime.rs`) gains an optional
  `acquisition: {authorize_url, token_url, scopes}`, present only for `oauth2_refresh` and skipped
  when absent, so existing profiles keep their bytes. `story:cli-oauth-loopback-acquisition` reads it
  from the capture reply.
- Cache: in-memory, keyed by a sha2 digest of the entry bytes (`sha2` moves from
  `[dev-dependencies]` to `[dependencies]` in `adapters/catalog/Cargo.toml:32`), valid until `expires_in - 60 s`, behind a `Mutex`
  (`Adapter: Send + Sync`, `runtime/server.rs:12`). Evicted when an invoke returns
  `InvalidCredential`. `validate` always exchanges fresh.
- `validate` with `identity.source: id_token`: subject from the `id_token` payload of the token
  response, after checking `iss` is `https://accounts.google.com` or `accounts.google.com` and
  `aud == client_id`; granted scopes from the space-separated `scope` string, so `minimum_scopes`
  (`local.rs:514-516`) keeps working. Fallback when no `id_token` is returned: GET
  `https://oauth2.googleapis.com/tokeninfo?access_token=…` on the token host, whose `sub` and
  `scope` fields give the same values. `credential_expires_at_ms` stays `None` (`:526`).
- Errors: token `invalid_grant` / `invalid_client` → `Failure::InvalidCredential`
  (`runtime.rs:43`); token 429 → `ProviderRateLimited`; 5xx → `Unavailable`; a refresh response with
  a different `refresh_token` → `InvalidCredential` (ADR point 3).
- `apps/connectors/src/local.rs:114`: a `ServiceFailure` whose `service_code` is `Unauthorized`
  reports `("dispatch", "repair_connection", false)`; `repair_connection` exists
  (`ess/domains/cli.yaml:252`).

## Acceptance

Fixture tests over the TLS fixture in `adapters/catalog/tests/local_runtime.rs`, extended with a
token route, each named in the test file:

- `oauth_exchange_sends_form_and_uses_bearer`: the token request is `POST` form-encoded with
  `grant_type=refresh_token`, `client_id`, `client_secret`, `refresh_token`, and the API request
  carries `Bearer <fixture access token>`.
- `oauth_cache_reuses_token_until_skew`: two invokes inside `expires_in - 60 s` make one token
  request; an invoke after it makes a second.
- `oauth_invalid_grant_reports_repair`: the CLI reports `repair_connection` for a token
  `invalid_grant`.
- `oauth_rate_limit_and_unavailable`: token 429 and 503 map to `ProviderRateLimited` and
  `Unavailable`.
- `oauth_rotated_refresh_token_refused`: a response with a different `refresh_token` is refused
  `InvalidCredential` and nothing is stored.
- `oauth_validate_identity_from_id_token` and `oauth_validate_identity_from_tokeninfo`: subject and
  scopes come from each source; a wrong `aud` is refused.
- `oauth_validate_wrong_iss_refused`: an `id_token` whose `iss` is neither accepted value is refused.
- `oauth_token_url_must_be_https` and `oauth_authorize_url_must_be_https`: an `http://` URL is refused
  when the configuration loads.
- `oauth_cache_evicted_on_invalid_credential`: after an API 401 mapped to `InvalidCredential`, the
  next invoke makes a new token request.
- `oauth_profile_carries_acquisition`: the profile of an `oauth2_refresh` instance carries the three
  acquisition fields; a `token` instance's profile serializes with no `acquisition` key.
- `oauth_entry_unknown_field_refused`: an entry with a fourth field is refused.
- Existing configurations produce identical configuration revisions (`token`, `basic` fixtures).
- No secret value appears in any log, error or output (the existing value-freedom checks).

## Out of scope

Acquisition (`story:cli-oauth-loopback-acquisition`); rotating refresh tokens; persisting access
tokens.
