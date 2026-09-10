# Changelog

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
