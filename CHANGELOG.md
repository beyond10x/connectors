# Changelog

## Unreleased

## 0.3.0 — 2026-09-12

Guarded GitLab writes, Kubernetes and PostgreSQL through the persistent local
lifecycle, Helm release reads, the catalog ingest-to-bundle track, and the first
two MCP contract records. 74 commits since 0.2.0.

### GitLab writes

- Added an approved merge through the local CLI: local approval policy with
  protected proof issuance, single-use write requests, an explicit private
  prepare/commit transport, and authenticated bounded clock checks. A pinned head
  is checked at dispatch and an uncertain outcome is never retried into a second
  effect.
- Added recovery for abandoned and keyed writes in the local owner background,
  exact audit-append acknowledgement recovery, and revoked-merge audit
  finalization with owner-crash replay verified.
- A failed terminal settlement now keeps the known provider result rather than
  replacing it, and a connection revoked after a known effect refuses disclosure
  at admission instead of answering `not_attempted`.

### Kubernetes and PostgreSQL

- Both providers reach the local CLI with saved credentials through the same
  persistent lifecycle, verified against real servers — k3s `v1.31.5+k3s1` via
  k3d and `postgres:17` — rather than fixtures.
- A backend-less EndpointSlice reads as empty rather than malformed. Kubernetes
  serialises such a slice with null members, and one Service with no ready
  backends previously failed discovery for every Service beside it.
- Added Helm release reads: revision history, deployed status, recorded values and
  rendered manifest, with bounded pages, explicit completeness and provenance
  naming the exact release Secret each observation came from. Values and manifests
  are disclosed only as redacted projections and the default is refusal.

### Catalog

- Added OpenAPI source ingest with preserved provenance, a deterministic operation
  inventory with named gaps, bundle write and load through a local index, a
  coverage report, and locally authored TOML actions expanding to the same
  template.
- Two writers into one bundle directory keep both index entries. The
  read-modify-write is held under an exclusive lock; before it, concurrent writers
  silently erased each other's rows while both were told they succeeded.

### Tooling and specification

- ESS 0.22.2 and AEP 0.55.0 are pinned by source identity with receipt-checked
  local binaries and a digest-pinned AEP findings correction. Global installations
  remain independently managed.
- The gate now re-derives every archived upstream source digest — 84 files across
  six manifests. Nothing had ever read one, so every pinned-evidence record in this
  repository was evidence by assertion.
- Corrected 88 false empty-findings warnings and preserved 51 immutable legacy
  reviews through source-bound structured transcriptions. These add no reviews,
  approvals or claims of runtime conformance.

### MCP contracts

- Pinned both MCP specification revisions this repository authors against —
  `2026-07-28` primary and `2025-11-25` for interoperability — as 54 archived
  files with source URL, uncompressed SHA-256 and byte length.
- Added the `connectors_mcp` ESS root declaring the MCP nouns and the three
  credential kinds the design keeps distinct, with every relation no source
  answers carried as an explicit marker rather than a chosen cardinality.

### Limitations

- **No dedicated GitLab sandbox evidence.** All five GitLab stories remain open on
  that credential; the writes above are verified by disposable CLI journeys and a
  private HTTPS fixture, not against a GitLab server.
- **Helm release reads are fixture-verified.** No real cluster has answered those
  four operations.
- **MCP is contracts only.** No connection is made, no server starts, no credential
  persists and no transport is selected. Ten of the epic's twelve stories are
  undrafted work.
- Helm chart rendering, linting, registry access and rollback stay outside
  Connectors pending a decision on whether it runs external provider binaries.
- Source release only. No hosted release page, binary, container image or website
  deployment.

## 0.2.0 — 2026-09-11

Persistent local GitLab CLI and engineering reads, based on implementation commit
`a5b399d4f790e993aa3ab76f6a61ac1ee25b6c7a`.

- Added local setup, adapter lifecycle, protected connection entry, repair,
  revalidation, terminal local revocation and cached operation discovery.
  SQLite stores metadata; qualified Linux Secret Service custody stores credentials.
  Saved credentials survive CLI, owner and keyring restarts.
- Expanded GitLab to eleven read operations: projects, issues, repository files,
  exact-commit pipelines and jobs, bounded traces, MR inspection and update-window
  collection, and validation of a pinned MR head and selected successful pipeline.
  Validation reports blockers and always reports `merge_performed: false`.
- Added protected approval-signing key initialization, status, rotation, recovery,
  revocation and retirement. Private mutation-ledger, execution-audit and approval
  proof/spend foundations have deterministic failure and restart tests.
- Updated website support descriptions and the saved-credential GitLab workflow,
  and documented the source-release procedure in `AGENTS.md`.
- Preserved `describe`, `invoke` and `serve`, the existing standalone Kubernetes
  and PostgreSQL reads, and Rust 1.88 compatibility.

This source release targets Linux x86_64. Build the optimized CLI and adapter for
local supervision; see the [GitLab guide](docs/local-gitlab-cli.md). Existing
installed configuration and credentials are not automatically imported.
The initial custody profile requires the exact qualified GNOME Keyring 50.0
daemon on ext4 and an already-unlocked encrypted login collection; see
[custody qualification](docs/local-secret-service.md).

The implementation passed the full gate, six production CLI journeys with
disposable HTTPS/keyring fixtures, native MR tests and local packaging checks;
[implementation evidence](docs/evidence/gitlab-mr-validation-20260910/README.md)
retains the exact inputs and limits. Release-version verification is recorded in
the [release receipt](docs/evidence/release-v020-20260910/README.md).

Dedicated GitLab sandbox acceptance remains open. Approval issuance, a qualified
production approval clock, governed GitLab writes, complete management pagination,
Kubernetes/PostgreSQL persistent local lifecycle, MCP and the remaining providers
are not completed by this release. Binary/registry distribution and website or
cloud deployment are separate from this source release.

## 0.1.0 — 2026-09-09

Initial specification baseline for the selected Kubernetes (including discovery),
GitLab and SQL scope.

- Defined shared service, operations, authentication, datasource, discovery,
  session and media semantics, with explicit compatibility and support boundaries.
- Co-located native contracts, designs and authored ESS models with their owning
  adapters so they can be extracted independently.
- Resolved all 48 original contract-review findings and retained independent
  reviews, source evidence and verification records.
- Modeled connection/profile/acquisition/custody ownership, execution audit,
  bounded discovery state, historical mediated bindings and reusable immutable
  artifact provenance alongside the existing execution/evidence models.
- Added the shared ESS ownership gate and independent validation of authored
  adapter models; pinned ESS 0.20.0.
- Kept deferred capabilities and runtime enforcement obligations explicit.

This milestone records contract and model stabilization. The repository's
existing first-slice runtime predates this hardening work; the new semantic
profiles and persistent models do not claim new runtime implementations.
Schema compilation and manual scenarios are distinct from runtime conformance.
