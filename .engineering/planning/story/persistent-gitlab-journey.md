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
  path: crates/connectors-conformance
- confidence: cited
  path: crates/connectors-host
- confidence: cited
  path: crates/connectors-sdk
- confidence: cited
  path: docs
- confidence: cited
  path: ess/domains
revision: 11
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

## GitLab runtime continuation — 2026-09-10

The active thread goal selects GitLab, Kubernetes and PostgreSQL usable through the CLI, in that order, while the initiative retains its full selected workflow scope. The previous goal turn updated authoritative scheduling; it did not implement runtime behavior. This story remains the first delivery.

First close the GitLab-owned static-entry profile and baseline identity/grant/expiry validation in adapters/gitlab/contracts/auth/v1alpha1/semantics.md, adapters/gitlab/spec/ess and adapters/gitlab/src/auth.rs. The provider interprets its own token document and bounded provider responses through the existing narrow AuthenticatedHttp capability; generic host/CLI code gains no GitLab branch. Existing shared AuthProfile, Connection, Acquisition, CustodyVersion, CredentialGeneration and evidence declarations own their lifecycle and relations. The native auth document introduces bounded values, not another connection or custody entity.

Then bind protected source preflight, durable custody, metadata publication and supervised invocation. The generated Sources seam runs before Handler, so production source admission must establish current configuration/profile/target and owner before consuming protected bytes. Do not use the demonstration source implementation or bypass the generated parser. Persisted metadata and keyring availability alone still establish no connection readiness.

Validate the pinned shared/generated ESS baseline, author and validate the native values before dependent runtime implementation, verify provider interpretation with deterministic failure cases, and run the required repository gate on final changed inputs. The dedicated live GitLab sandbox has been requested again; its missing evidence remains recorded independently of ongoing implementation. No acceptance is closed by this note.

## Native GitLab auth implementation checkpoint — 2026-09-10

The new adapter-owned auth contract/model and adapters/gitlab/src/auth.rs implement strict protected PAT parsing plus native user/token identity, scope, expiry and freshness interpretation through the supplied scoped HTTP capability. Six tests pass, including a disposable server using production HTTP transport. SDK Secret and header assembly now clear their owned buffers; legacy/federation/conformance conversions are updated to retain compatibility. crates/connectors-conformance was added to the machine-readable cited scope for that affected consumer.

The full repository gate with Rust 1.88, generation, conformance, tests/Clippy and boundaries passes on these inputs. Documentation drift checks pass. Exact commands, hashes and limits are retained in docs/evidence/gitlab-auth-20260910/README.md. This evidence covers a native helper and its boundaries, not CLI acquisition or the persistent restart journey.

GNOME Keyring 50.0 source inspection finds file fsync followed by rename in its save path; the complete durability barrier and exact storage/service binding still need qualification before an acknowledged write can publish a connection. No existing keyring secret was accessed or changed. The private bootstrap binding, source admission, custody coordinator, metadata publication and supervised CLI invocation remain required. The dedicated sandbox request is still unanswered. This story and the three-provider goal remain active; no runtime milestone is marked complete.

## 2026-09-10 Secret Service custody checkpoint

The host now implements scoped immutable Secret Service writes and exact-version reads, with explicit file/directory synchronization for the audited GNOME Keyring 50.0 daemon artifact on the admitted ext filesystem family. docs/local-secret-service.md records the private layout, process/executable binding, encryption checks, acknowledgement boundary and limits. Existing auth_bindings.CustodyVersion semantics own the scoped tuple; this adds no provider entity or new planning decomposition.

Four tests passed in an explicit native qualification run, including two disposable DBus/GNOME daemon fixtures. Evidence covers exact-version reuse after SIGKILL and unlocked restart, locked restart refusal, dead-owner capability refusal, failed native writes, uncertain synchronization, immutable concurrent writes, scope and metadata denial, unsafe/changed backing files, and physical deletion surviving restart. The repository gate passed, including Rust 1.88, ESS, generation, conformance, workspace tests, Clippy and boundaries. Website reference drift check passed. Exact inputs and results are retained in docs/evidence/gitlab-custody-20260910/README.md.

Physical deletion is test-only until the host's retirement fence, 24-hour retention and no-valid-use/recovery checks are implemented. Metadata publication and protected input/adapter bootstrap/supervision remain unfinished; setup's persistent custody prerequisite remains failed. This story stays active and has not passed its CLI/sandbox acceptance. Next bind the SQLite acquisition/publication/retirement records and the private adapter runtime so the validated GitLab credential can actually be saved and reused through the generated CLI. The existing sandbox and AEP driver blockers remain unchanged. The initiative still has one decomposing child, so no new critic panel is required for this implementation checkpoint.

## 2026-09-10 SQLite registry and CLI management checkpoint

Migration two binds the existing connection/acquisition/generation/custody/evidence semantics to SQLite WAL/FULL with a persistent clock floor. Static-entry completion, custody acknowledgement and publication retain separate boundaries. Repair preserves external identity and public semantic revision; competing publication, terminal revoke and final read dispatch share a private fence. Retirement requires the acknowledged fence, terminal acquisition, full 24-hour retention and no live use under the physical writer lock. No business-write approval, audit, idempotency or transport is claimed by these read guards.

The shared EvidenceSnapshot now explicitly retains optional native granted scopes and known credential expiry, validated before runtime implementation. contracts/auth/evidence/v1alpha1/semantics.md section 4.5 owns the additive internal observations and 64-scope/256-byte limits; provider interpretation remains native. No public CLI projection changed and no new entity or governed decomposition was introduced. The GitLab auth implementation now refuses implication overflow beyond the retained bound.

Production generated-parser handlers implement connections list/describe/status/revoke without launching an adapter or reading credential material. Missing authority remains unavailable; it never becomes an empty connection list. setup check distinguishes a merely available service from the exact qualified custody binding. docs/local-connection-registry.md records the model-to-table mapping, short transactions and distinct acknowledgement groups. docs/evidence/gitlab-registry-20260910/README.md retains the full Rust 1.88 gate, fifteen passing local/native tests, exact input hashes and documentation checks. Native fixture evidence covers real private keyring/SQLite restart, CLI acquisition status/revoke, unknown write/delete acknowledgement and guarded cleanup, using fictional credentials.

Protected connect/repair entry, explicit baseline revalidation, private adapter bootstrap/owner supervision and the allowed GitLab read/restart journey remain unfinished. Dedicated GitLab sandbox access is still unavailable under credential-blocker:gitlab-runtime-sandbox; independent implementation continues. The story, initiative and three-provider goal remain active. Next bind the private adapter/bootstrap and protected source admission, then execute GitLab connect/read/restart reuse. Continue Kubernetes and PostgreSQL before MCP, followed by the remaining providers. No sandbox acceptance or full runtime milestone is closed by this checkpoint.
