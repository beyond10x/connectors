# Changelog

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
