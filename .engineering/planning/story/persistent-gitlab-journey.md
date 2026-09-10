---
format: aep.planning-md/1
id: story:persistent-gitlab-journey
kind: story
status: active
title: Persist and reuse an admitted GitLab connection across CLI and owner restarts
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:local-cli-binding-semantics
- informed_by: story:local-cli-ess-surface
- informed_by: story:gitlab-spec-service
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/gitlab
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: contracts/auth
- confidence: cited
  path: contracts/cli
- confidence: cited
  path: crates/connectors-build
- confidence: cited
  path: crates/connectors-host
- confidence: cited
  path: crates/connectors-sdk
- confidence: cited
  path: docs
- confidence: cited
  path: ess/domains
revision: 6
---
## Acceptance

From a fresh private configuration on Linux x86_64, an owner can enter an existing GitLab sandbox credential through a protected source, save a validated connection, invoke an existing allowed GitLab read, restart both CLI and local owner and repeat the read using the retained exact credential version without re-entry, with deterministic failure evidence and the required repository gate passing on Rust 1.88.

## Existing semantic owners

contracts/cli/v1alpha1/semantics.md, contracts/auth/management.md and the connection/acquisition/custody/evidence contracts own admission, protected entry, publication and reuse. Existing ESS domains cli, auth_bindings, credentials, credential_evidence, connection_admission and declarations own these values and identities. Provider-owned GitLab native profiles and runtime bindings must validate against adapters/gitlab/spec/ess before introducing any missing profile/bootstrap semantics.

## Sequence and boundaries

Implement production generated-parser handlers and protected input sources, safe local configuration and state opening, SQLite migrations and binding metadata transactions, qualified immutable Secret Service custody, owner-checked local transport and exact configured adapter bootstrap, then the existing GitLab read journey. Provider libraries receive narrow authenticated capabilities and never the database or arbitrary keyring access. Preserve legacy describe/invoke/serve. Unsupported management and write routes refuse safely until their milestones implement them.

SQLite is one physical metadata authority, WAL with synchronous FULL and foreign keys, bounded busy waiting, transactional versioned migrations and immutable publication coordinates. Preserve separate custody acknowledgement, metadata publication, audit/spend and dispatch boundaries; a database commit does not prove custody durability or provider effects. No secret documents or credential locators appear in public results. No unavailable metadata read means absence.

Verify protected path traversal/ownership, bounded capture and redaction; unsafe/locked/missing/failed custody; crash after custody before metadata; concurrent publication/revoke/final dispatch; failed same-identity repair preservation; restarted exact-version reuse. Record actual limits and refusals. Do not mark the journey done from parser/unit/specification tests alone.

## Scope

Cited from the declared owners, supplied plan and implementation diff: Cargo.toml, Cargo.lock, README.md, apps/connectors, crates/connectors-host, crates/connectors-sdk, crates/connectors-build, adapters/gitlab, contracts/cli, contracts/auth, ess/domains and docs. The generated CLI contract package changes only through its owning generator if the authored binding changes. The build-tool scope includes its runtime-obligation diagnostic and generation/check integration.

This story is the sole decomposing child initially; the planning critic panel is skipped under planning skill section 7 until two or more decomposing children exist. Later milestone decomposition follows semantic review. One writer owns planning-store mutations; the machine-readable scope is recorded through aep plan artifact scope.

## Prerequisites and current state

Dedicated GitLab sandbox access and actual Secret Service durable write/restart/deletion qualification are still required. Initial implementation starts with the existing specified setup/protected-source and metadata foundations; neither is a claim that credential custody or the restart journey is available yet.

## 2026-09-10 implementation checkpoint

Initial production foundation is implemented and verified; this story's acceptance remains unmet. See docs/local-runtime-foundation.md and docs/evidence/local-runtime-20260910/README.md for exact support and evidence.

Implemented: generated-parser setup/inventory handlers; private no-symlink configuration and state admission; exclusive fsynced setup; SQLite authority identity and versioned migration with WAL/FULL; metadata lifetime serialization including last-close sidecar retirement; local bus/service owner-checked non-interactive keyring availability; explicit source and runtime refusals; preserved legacy describe/invoke/serve and help.

Final cargo run --locked --offline -p connectors-build -- gate --msrv passed with task-owned TMPDIR/CARGO_TARGET_DIR and two jobs, including Rust 1.88, generation, conformance, full tests/Clippy and dependency boundaries. Nine new tests include sixteen four-caller initialization races. Website reference generation/check passed after populating its missing ignored cache. The final SQLite dependency is rusqlite 0.40.2 with bundled SQLite 3.53.2; the runtime refuses overrides predating the 3.51.3 WAL fix.

No provider credential has been captured or stored, no managed connection published, and no local owner or adapter supervised. GitLab currently has generated request ESS, not an authored static-entry profile/bootstrap contract; model and review those native semantics before implementing their dependent runtime. Persistent Secret Service write/restart/deletion qualification remains required. credential-blocker:gitlab-runtime-sandbox records missing dedicated live-provider access. It does not block independent implementation, and the nine foundation tests are not the restart journey.

MCP adoption/implementation/publication, later provider milestones, complete governed decomposition/critic reviews and reproducible distribution verification remain outstanding. The initiative and this story stay active; no selected workflow is marked complete by this checkpoint.
