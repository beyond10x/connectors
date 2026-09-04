---
format: aep.planning-md/1
id: story:durable-gitlab-oauth-refresh
kind: story
status: active
title: Keep delegated GitLab reads alive across OAuth refresh
summary: Rotate and durably commit GitLab OAuth credentials before expired user connections lose repository access.
relations:
- decomposes: epic:credential-production
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: catalog
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates/catalog-reader/catalog.pack
- confidence: cited
  path: crates/connectors-console/Cargo.lock
- confidence: cited
  path: crates/connectors-console/Cargo.toml
- confidence: cited
  path: crates/connectors-console/src/auth.rs
- confidence: cited
  path: crates/connectors-runtime/Cargo.lock
- confidence: cited
  path: crates/connectors-runtime/Cargo.toml
- confidence: cited
  path: crates/integration-gitlab/src/backend.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend_tests.rs
revision: 5
---
## Context

A live delegated GitLab OAuth connection remains discoverable after its two-hour access token expires, but the first credential-backed datasource read is refused even though the client secret and refresh token remain present. Refresh-token rotation must either commit the replacement credential set durably or return a precise reconnect-required state without misrepresenting the failure as repository authorization.

## Acceptance

- A deterministic integration test expires a GitLab OAuth access token, performs the refresh grant, and proves the replacement access and refresh tokens are durably committed before a repository datasource read succeeds.
- Concurrent reads cause at most one refresh exchange and observe the committed credential generation.
- A refresh failure is distinguishable from a missing repository grant at the product boundary without exposing credential or provider response bytes.
- The released hosted Connectors runtime passes the repository gate and the live Devcenter repository read succeeds after deployment.

## Scope

- `crates/integration-gitlab/src/backend.rs`
- `crates/integration-gitlab/src/backend_tests.rs`
- Connector protocol/product refusal mapping if required by the demonstrated failure.
- Release metadata and live deployment evidence.
