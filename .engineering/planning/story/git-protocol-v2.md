---
format: aep.planning-md/1
id: story:git-protocol-v2
kind: story
status: implemented
title: Broker bounded Git protocol v2
tags:
- coding-workspace-speed
relations:
- derived_from: story:broker-read-only-git-fetch-sessions
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
  path: crates
- confidence: cited
  path: crates/integration-gitlab/Cargo.toml
- confidence: inferred
  path: crates/integration-gitlab/src/git_fetch.rs
- confidence: inferred
  path: crates/integration-gitlab/src/git_fetch_advertisement.rs
- confidence: cited
  path: crates/integration-gitlab/src/git_fetch_v2.rs
- confidence: cited
  path: crates/integration-gitlab/tests/support
- confidence: cited
  path: crates/server/src/egress.rs
- confidence: inferred
  path: crates/server/src/hosted/git_fetch.rs
- confidence: inferred
  path: crates/service/src/git_fetch.rs
- confidence: inferred
  path: docs/design
revision: 10
---
## Acceptance

A real Git v2 client discovers only the admitted branch and HEAD then fetches the exact authorized commit through the streaming proxy at depth 50, while legacy clients remain supported and upstream HTTP connections are reused without caching authority.

## Implementation

Design 20 (`docs/design/20-bounded-git-protocol-v2.md`) records the additive protocol before implementation. Strict bounded packet framing and capability/argument grammar cover discovery, ls-refs and fetch. The proxy requires actual upstream v2 negotiation, binds it to the source-capability generation, rewrites upstream discovery prefixes and verifies the exact returned branch/HEAD identities. Reference commands retain the final fetch; a complete pack spends it and interrupted or refused transfers revoke the capability. The existing control contract and all expiry/request/byte/depth limits remain.

`ConnectionEgress` retains at most 64 reusable clients per immutable policy, separated by authority, origin and the complete currently admitted address set, with 60-second idle expiry. Every request still resolves and validates addresses; credentials and timeouts remain request-specific. Independent project and branch admission reads run concurrently with both validations and project-first refusal precedence preserved.

## Scope

- Cited: crates/integration-gitlab/src/git_fetch.rs and git_fetch_v2.rs implement the broker and bounded parser; crates/integration-gitlab/tests/support holds unit and real HTTP interoperability fixtures.
- Cited: crates/server/src/hosted/git_fetch.rs validates headers before reading bodies; crates/server/src/egress.rs owns bounded connection reuse.
- Cited: docs/design records the protocol and performance design.
- Cited: Cargo manifests and lockfiles, catalog, connectors.lock and crates/catalog-reader/catalog.pack prepare the coordinated 0.6.4 source identity using repository-owned catalog generation.

## Validation

Observed real Substrate gix materialization through the Connectors production HTTP router/broker over local TLS, followed by Git's HTTP backend: exact detached commit, 50 usable commits, no tags, no persisted source authority. The generic fixture has 4,000 unrelated branches plus a matching-prefix branch: provider legacy discovery is 280,511 bytes and v2 capabilities plus targeted reference rows are 354 bytes. These are local fixture byte counts, not deployed service latency measurements.

Focused regression tests cover strict requests/responses, reference filtering, capability generation, state consumption, interrupted streams, authority and budget refusals. Egress tests observe one reused socket across distinct credential-bearing requests, separate changed destinations and authorities, enforce the cache bound/idle expiry, and refuse a currently denied DNS answer despite a warm cache. Full repository gate evidence is recorded separately when observed.

## Delivery

Consumer order is Connectors 0.6.4, Substrate 0.7.3, then Workspace and Devcenter pins. This work prepares source identity only; commits, publishing and release/deployment evidence belong to the coordinated integration run.
