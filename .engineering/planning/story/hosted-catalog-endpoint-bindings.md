---
format: aep.planning-md/1
id: story:hosted-catalog-endpoint-bindings
kind: story
status: active
title: Hosted catalog connections retain deployment-declared destinations
relations:
- informed_by: story:the-hosted-posture-connects-a-catalogued-provider
- informed_by: story:deployment-declared-destination-aperture
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: catalog/grafana.catalog.json
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates/catalog-reader/catalog.pack
- confidence: cited
  path: crates/connectors-cli/Cargo.lock
- confidence: cited
  path: crates/connectors-cli/src/error.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-cli/tests/hosted_connect.rs
- confidence: cited
  path: crates/connectors-client/Cargo.toml
- confidence: cited
  path: crates/connectors-client/src/hosted_connect.rs
- confidence: cited
  path: crates/connectors-client/src/hosted_connect_tests.rs
- confidence: cited
  path: crates/connectors-client/src/identity.rs
- confidence: cited
  path: crates/connectors-client/src/lib.rs
- confidence: cited
  path: crates/connectors-client/src/model.rs
- confidence: cited
  path: crates/connectors-config/src/hosted.rs
- confidence: cited
  path: crates/connectors-config/src/hosted_catalog.rs
- confidence: cited
  path: crates/connectors-config/src/lib.rs
- confidence: cited
  path: crates/connectors-console/Cargo.lock
- confidence: cited
  path: crates/connectors-console/src/connect.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_endpoint_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted_endpoints.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: docs/guides/administer-hosted-integrations.md
- confidence: cited
  path: providers/grafana.toml
revision: 25
---
## Outcome

A deployment may bind the catalogue's declared endpoint variables for a hosted provider, allowing the existing Connect Session credential form and declared verification operation to work for configurable origins. The caller supplies only credential bytes through normal custody and cannot change the destination. Each durable connection pins its endpoint bindings; deployment changes make incompatible old connections unavailable until reconnected and never redirect an old credential.

## Acceptance

- Unconfigured templated providers expose no acquisition profile. Invalid, unknown, non-HTTPS or undeclared endpoint configuration refuses startup.
- Configured provider acquisition shows its deployment-selected destination and verifies the submitted credential using the provider's declared operation through the ordinary catalog transport and custody paths.
- Stored endpoint bindings survive restart. Destination removal or change prevents spending an old credential, including when another provider admits the old host.
- Owner and tenant isolation, session capability validation, exact transport origin/path and denied caller endpoint overrides have executable refusal coverage.
- Public-network policy remains the default. Explicit operator network configuration admits only the resolved declared origin through the existing post-DNS transport policy. Fixed-origin providers and old stored connections remain compatible.

## Existing typed authority

Provider and Connection are already declared in ess/system/domains/catalog.yaml and ess/system/domains/connection.yaml. This adds deployment configuration and private connection metadata to those existing entities; no provider API or operation schema changes.

## Scope

- crates/connectors-config/src/hosted.rs, hosted_catalog.rs and lib.rs: deployment policy fields, structural validation and public configuration type.
- crates/integration-catalog/src/hosted.rs, hosted_endpoints.rs, hosted_endpoint_tests.rs and lib.rs: canonical endpoint validation, connection pinning, acquisition, transport selection and boundary tests.
- crates/connectors-runtime/src/composition.rs: exact origin and explicit network policy composition.
- docs/guides/administer-hosted-integrations.md: generic configuration and credential acquisition contract.

This is one bounded implementation alongside the consuming application's separate work. No decomposition or critic panel is needed for a single story.

## Validation so far

All twelve repository Cargo workspace gates pass on implementation commit 7e11cb919bd2746e8d3e04c1e74ede788409f147, including both default and no-default-features hosted-runtime configurations. The final gate passes: 65 providers and 70 artifacts verified, portable Markdown links, story index, ESS 0.18.0 specification validation and byte-identical committed clap projection. Focused final suites pass 136 tests; all-target clippy for connectors-config, integration-catalog and connectors-runtime passes with warnings denied. Formatting and diff whitespace checks pass.

The first broad run found the configuration module size fence; extracting hosted_catalog.rs restored the bound without a waiver. A later concurrent run hit an existing one-second OAuth fixture expiry during setup, which passed unchanged alone and with bounded test concurrency. A long temporary directory also exceeded the native completion transport's Unix socket path bound; the unchanged test passed with a short task-owned temporary root. The completed broad gate used RUST_TEST_THREADS=4, a short private TMPDIR and the repository-pinned ESS toolchain. Failed evidence is retained alongside successful runs.

New endpoint metadata is additive to the existing v1 state and omitted for default fixed-origin records. Backward loading and serialization are tested. A connection created for a newly configurable provider needs this updated runtime; an older runtime has no implementation for spending its endpoint-bound credential. No change to provider declarations, canonical request/response schemas, SQL storage or Secrets protocols is made.

Actual composed browser acceptance and the real provider credential remain the consuming deployment's next proof; this artifact remains active until that evidence arrives.

## Credential entry clarity

The actual hosted Grafana form still labels its single masked input only as Credential, which leads an SSO user to infer that the form asks for their browser password. The renderer must derive the label, help, authentication description and documentation link from the selected catalogue credential/config declaration. Grafana's declared service_account_token help must explicitly distinguish the API token from the SSO password and direct the owner to the Grafana service-account token page. Keep the generic renderer provider-neutral and preserve normal credential custody and endpoint pinning.

Official sources checked on 2026-09-07: https://grafana.com/docs/grafana/latest/administration/service-accounts/ and https://grafana.com/docs/grafana/latest/developer-resources/api-reference/http-api/authentication/. These describe service accounts separately from Grafana users and service account tokens as bearer credentials for the HTTP API. This is acquisition copy only; provider endpoint, operation and request/response schemas remain unchanged.

Acceptance: the real selected profile's form renders its matching credential label/help and official documentation; a second profile of the same provider renders its own copy; text is escaped and unsafe documentation links are omitted. Regenerate affected canonical artifacts and prove unchanged operation schemas, then run the required source gate. Actual deployment/credential changes remain with the consuming deployment.

## Credential form verification

The form now renders the exact selected credential's catalog label, help, canonical authentication description and safe HTTPS documentation link. Grafana asks for a Grafana service account token and explains that an SSO password is not an API token. The renderer contains no Grafana-specific selection rule. Tests also distinguish Anthropic's ordinary and administrative profiles and prove HTML escaping and unsafe-link refusal.

Validation passed on 2026-09-07: 110 integration-catalog tests, all-target clippy for integration-catalog and connectors-runtime, formatting, the complete scripts/gate.sh run across all twelve workspaces and both runtime configurations, and its final catalog/docs/ESS lane. Both catalog build/diff rounds reached the same fixed point; catalog check verified 65 providers and 70 artifacts. Comparing the Grafana canonical document with config removed proves all operations, auth, services and schemas unchanged. Only declared credential help/docs and the resulting pack/lock hashes changed.

The consuming deployment must still render the published form and complete its own real credential/read proof. This source change adds no admin credential requirement or programmatic provisioning endpoint to the generic catalog adapter.

## Owner-scoped programmatic acquisition

The owner-authorized hosted token flow can reuse Connection v1 CreateConnectSession and Status plus the existing one-use HTTP completion route. The CLI adds `connectors setup connect <provider> --target hosted --auth-profile <declared-profile> --credential-file <owner-only-file> --label <label>`, retaining local as the default. It uses the existing saved Identity session, requests only the ordinary self-connection and catalog scopes, and never requests administrative or cross-owner authority. Deployment automation with its own existing principal can use the reusable HostedClient completion method and unchanged APIs; this work does not invent workload login, create provider service accounts, or register shared administrative credentials.

Acceptance: owner-only regular non-symlink bounded credential input is read without output; returned session capabilities remain private and can be submitted only once to the selected Connector origin and exact session route with redirects refused; completion is reported only after owner-scoped Status and Describe agree on provider, profile and callable Connection. Refused/expired sessions, hostile destinations, redirects, provider verification failure, foreign Connection or profile, and unsafe files fail without secret/capability diagnostics. Multiple token providers exercise the same path. Existing personal setup, OAuth and operation routing retain their behavior. The selected provider's runtime Connect Session contract remains the authority for profile admission and custody.

Cited existing typed authority: ess/system/domains/connection.yaml ConnectSession and CreateConnectSession/FinishConnectSession; crates/protocol/src/connection.rs ConnectSessionStatus/Create request; crates/server/src/hosted/connect.rs one-use POST; crates/connectors-client/src/identity.rs scope selection; crates/connectors-cli/tests/cli_surface.rs treats setup connect as an existing multi-step Flow. No new wire entity, API schema or generated command path is introduced.

Scope adds the reusable client completion/workflow and tests, a thin CLI target branch and parsing tests, and generic setup documentation. This is one bounded story, so no parallel decomposition/critic panel is needed. The sub-agent run is non-interactive; no missing authority is treated as approval.

## Local TLS prerequisite

The consuming local deployment's public discovery route returns the valid contract with its retained CA, but the installed CLI fails Discovery because its reqwest build contains only bundled WebPKI roots. Enable native certificate roots on the client-owned reqwest dependency so the normal platform trust store and SSL_CERT_FILE can admit a private development CA while retaining full certificate and hostname verification. This is required to exercise the same hosted Identity/Connect Session path against the local TLS deployment. Preserve the user's saved login and use isolated state for verification; never add an insecure TLS flag.

## Programmatic acquisition boundary

The reusable protocol client remains independent of embedded provider declarations and accepts an already-authorized Identity bearer plus the ordinary OwnerContext. Its caller selects a declared single-secret token profile. Incompatible native flows retain their server refusal: Jira raw completion is Invalid; Slack OAuth raw completion is Refused; Slack companion input must parse exactly two tokens before any verification or custody commit. The CLI performs a conservative preflight in connectors-console, which already links the pinned catalogue: exact credential, ConnectSession acquisition, stated subject and exactly one matching secret config field. It refuses unknown, OAuth, Basic and native-only profiles before reading the credential file or acquiring Identity authority. Runtime deployment admission remains authoritative, and newer profiles require matching CLI declarations while protocol clients remain catalogue-version-independent.

## Programmatic acquisition verification

The complete scripts/gate.sh run passed on 2026-09-07: all twelve Cargo workspaces, both hosted runtime feature configurations, catalog verification (65 providers, 70 artifacts), documentation/story checks, ESS 0.18.0 validation and exact committed clap projection. The client boundary suite uses real TLS with a temporary CA and verifies native SSL_CERT_FILE trust in isolated child processes, rejection without that CA, exact origin/API-prefix/session route, ownership/permissions/type/size file refusal, redirect refusal, dropped-response no replay, malformed acknowledgement refusal and matching completed principal Connection status. Three CLI process tests pass, including unknown/OAuth/native-multifield profile refusal before Identity and file access. Formatting, whitespace checks and client/CLI all-target clippy pass.

The consuming deployment independently reports a successful fresh normal Identity CLI login through a headless browser, upstream fixture, Identity callback and CLI loopback, followed by normal scope renewal and a hosted Connection list. It used task-isolated local selection and normal keyring custody without changing the operator's existing hosted session. Private evidence is retained as hosted-cli-login-672379418819327/result.json and hosted-cli-read-1788792695405657932/connections.json. The first browser helper attempt failed for missing TMPDIR and the corrected environment passed; failed evidence remains available.

The first focused source fixture used an HTTP hosted completion URL, which the unchanged Connection protocol correctly refused. Fixtures were corrected to use TLS and required Connection initiation/actor fields rather than weakening validation. Real provider token acquisition and an actual Grafana read remain pending owner setup; no service-account token was accessed or minted, and source/boundary tests are not claimed as real-provider success. This artifact remains active for that consuming-deployment evidence.

The final all-target clippy run for connectors-console also passes with warnings denied. Its existing catalogue dependency owns CLI preflight; connectors-client gains only native-root TLS support plus test-only TLS fixture dependencies and retains no catalogue/runtime dependency.
