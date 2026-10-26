---
format: aep.planning-md/3
id: story:connect-runs-an-oauth2-grant
kind: story
status: implemented
title: connections connect runs an OAuth2 grant for an oauth2_grant profile and stores the token response
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
- depends_on: story:oauth2-grant-core
- depends_on: story:oauth2-refresh-metadata-authority
- informed_by: architecture-decision-record:oauth2-grants-and-access-token-issue
scope:
- confidence: cited
  path: adapters/catalog
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: docs/catalog-google-oauth.md
- confidence: cited
  path: ess/domains/auth_access.yaml
- confidence: cited
  path: ess/domains/cli.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:54Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:54Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-26T02:55:28Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Outcome

A catalog connector can declare an auth profile with scheme `oauth2_grant` and an `oauth2` block (a
`connectors.oauth2.ServerDeclaration`). `connections connect` and `connections reconnect` on such a profile
run the grant in the CLI process through `connectors-oauth2` — authorization code with PKCE S256 over the
declared redirect kind, or the device grant with `--device` — and store the validated token response as
the connection's custody entry. Nothing is stored when the grant fails, times out or is cancelled.

## Model

- `connectors.oauth2.ServerDeclaration` (`ess/domains/oauth2.yaml`), recorded per profile record with
  `DeclareAuthorizationServer` when the profile is recorded.
- The grant run is one `connectors.auth_bindings.Acquisition` (`ess/domains/auth_bindings.yaml:248`):
  `BeginAcquisition` → `ConsumeAcquisition` → `CompleteAcquisition`, or `FailAcquisition` /
  `ExpireAcquisition`. `connectors.auth_access.AcquisitionFlow` gains `oauth2_device_code`.
- CLI: `ConnectionConnectInput` and `ConnectionReconnectInput` (`ess/domains/cli.yaml:1219,1233`) gain an
  optional `device` flag; the binding `apps/connectors/spec/cli.yaml` maps it to `--device`, and the
  generated contract is regenerated (`crates/connectors-build/src/cli.rs:9-38`).

## Design

- Profile reader: `adapters/catalog` accepts `scheme: "oauth2_grant"` with `oauth2: {…}` (field names of
  `ServerDeclaration`); a declaration without `grant_types`, with a non-HTTPS endpoint, or naming a
  `device_code` grant without `device_authorization_url` refuses the configuration.
- `client_id`: the declaration's, else the instance's configuration value `oauth2_client_id`; neither
  present refuses with `invalid_configuration` before any network step. A confidential client's secret is
  entered through the existing protected entry (`--credential-file`/`--credential-stdin`).
- Entry shape (custody, at most 64 KiB, `crates/connectors-custody/src/lib.rs:44`):
  `{client_id, access_token, refresh_token, expires_at_ms, scopes, id_token?}`.
- Validation in the owner (`Task::Validate`), never by a refresh: required scopes ⊆ granted scopes;
  identity from the `id_token` claims received directly from the token endpoint over TLS (OpenID Connect
  Core 1.0 §3.1.3.7 item 6), or from the profile's declared identity request with the access token.
- The consent address goes to `/dev/tty`, else one stderr line `connectors: consent-url <address>`
  (`docs/catalog-google-oauth.md` § Connect with OAuth); a device grant prints
  `connectors: device-code <user_code> <verification_uri>` the same way. Neither carries a token.
- `oauth2_grant` is refused on read-only custody (as `transport.rs:2137-2146` refuses OAuth acquisition)
  and for credential reuse (`transport.rs:2608-2611`).
- Device dialect dispatch: `--device` on a profile runs RFC 8628 against `device_authorization_url`
  unless composition (`apps/connectors`) has registered a device dialect for that profile id, in which
  case the registered `connectors-oauth2` device-dialect implementation runs and returns a code and
  verifier for the ordinary exchange. This story owns the registry and its lookup; the first real dialect
  is `story:chatgpt-subscription-connector`'s.
- Contract: `contracts/auth/acquisition/v1alpha1/semantics.md` gains the `oauth2_grant` row in the flow
  table (this story owns that row); `contracts/cli/v1alpha1/semantics.md` documents `--device`.

## Acceptance

- `adapters/catalog` tests: `oauth2_grant_profile_reads_its_declaration`,
  `oauth2_grant_without_client_id_is_invalid_configuration`, `oauth2_grant_http_endpoint_is_refused`.
- CLI journeys against a fake authorization server (debug-build follow hook, as `cli_journey.rs:239` uses
  `CONNECTORS_TEST_OAUTH_FOLLOW`): `oauth2_grant_connect_with_loopback_fixed_port`,
  `oauth2_grant_connect_with_pasted_code`, `oauth2_grant_connect_with_device_grant`,
  `oauth2_grant_reconnect_replaces_the_entry_on_the_same_connection`; each ends with the connection
  `live` and `connections describe` showing granted scopes and expiry, never a token.
- `oauth2_grant_state_mismatch_stores_nothing`, `oauth2_grant_device_expired_stores_nothing`,
  `oauth2_grant_missing_required_scope_stores_nothing`.
- `device_grant_uses_the_registered_dialect`: a test dialect registered for the profile id is called and
  the RFC 8628 endpoint is not; `device_grant_without_a_dialect_uses_rfc8628`.
- The journeys run in `cargo run --locked -p connectors-build -- ignored --required`, whose report names
  each of them as passed (an ignored or skipped journey does not count).
- `rg -n 'oauth2_grant' contracts/auth/acquisition/v1alpha1/semantics.md` prints the flow-table row.
- The metadata authority records `DeclareAuthorizationServer` once per profile record and the
  Acquisition's transitions for each journey (asserted on recorded events).
- `apps/connectors/tests/vocabulary_text.rs` and `crates/connectors-conformance/tests/cli_surface.rs` pass
  with the new flag; `connectors-build cli --check` and `connectors-docs generate --check` exit 0.

## Out of scope

Refresh (`story:owner-rotating-refresh`); issuing the token (`story:connections-token-issues-an-access-token`);
any provider declaration; catalog operations dispatched with an `oauth2_grant` token.
