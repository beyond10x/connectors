---
format: aep.planning-md/3
id: story:google-consent-on-the-oauth2-core
kind: story
status: draft
title: The Google consent flow uses the OAuth2 core's PKCE and loopback receiver
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
- depends_on: story:oauth2-grant-core
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/connectors-host/Cargo.toml
- confidence: cited
  path: crates/connectors-host/src/local/oauth.rs
revision: 4
---
## Outcome

The Google consent flow (`crates/connectors-host/src/local/oauth.rs`) uses `connectors-oauth2` for PKCE,
`state` and the loopback receiver, so the repository holds one implementation of each. Its behaviour,
messages and stored triple are unchanged.

## Model

No model change: the Google profile stays `static_entry`, OAuth-acquired non-rotating variant
(`architecture-decision-record:oauth-material-as-static-entry`); the core's types are
`connectors.oauth2` (`ess/domains/oauth2.yaml`).

## Design

- Replace `challenge`, `random`, `Loopback`, `receive`, `Drain`, `redirect` and `answer`
  (`oauth.rs:128-530`) with the core's equivalents, configured `loopback_any_port`, path `/`,
  confidential client, `form` encoding.
- Keep in `oauth.rs` only the Google rules: installed-client JSON, endpoint equality with the profile,
  `openid` plus `access_type=offline` and `prompt=consent` (now `authorize_parameters`), the triple.

- Adds `connectors-oauth2` to `crates/connectors-host/Cargo.toml`, as `story:owner-rotating-refresh` does;
  whichever lands second rebases onto the first's manifest line.

## Acceptance

- Every existing `oauth.rs` unit test and `oauth_adversary_pass2_tests.rs` passes unchanged in name and
  assertion (diff shows no assertion removed or relaxed).
- `rg -n "fn challenge|struct Loopback|struct Drain" crates/connectors-host/src/local/oauth.rs` prints
  nothing.
- The ignored CLI journeys `oauth_connect_journey` and `oauth_repair_journey`
  (`adapters/catalog/tests/local_runtime/cli_journey.rs`) pass when run with a disposable Secret Service,
  as before.

## Out of scope

Moving Google to `oauth2_grant`.
