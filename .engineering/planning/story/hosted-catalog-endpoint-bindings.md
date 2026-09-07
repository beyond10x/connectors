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
  path: crates/connectors-config/src/hosted.rs
- confidence: cited
  path: crates/connectors-config/src/hosted_catalog.rs
- confidence: cited
  path: crates/connectors-config/src/lib.rs
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
revision: 11
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
