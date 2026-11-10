---
format: aep.planning-md/3
id: story:catalog-guard-optional-preflight-values
kind: story
status: active
title: A guard preflight can forward an optional input value when the caller gives it
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: adapters/catalog/providers/google-drive/operations.json
- confidence: cited
  path: adapters/catalog/spec/ess/domains/guard.yaml
- confidence: cited
  path: adapters/catalog/src/closed.rs
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/engine.rs
- confidence: cited
  path: adapters/catalog/tests/google_drive.rs
- confidence: cited
  path: adapters/catalog/tests/google_drive_writes_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/guard_optional_values_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/guard_optional_values_adversary_pass2.rs
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-catalog/src/definition.rs
- confidence: cited
  path: crates/connectors-catalog/src/import.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/google-drive.yaml
- confidence: cited
  path: docs/catalog-google-drive.md
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 20
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:57:50Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:57:50Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Source

Adversary pass 1 on Google Drive writes (review-result:adversary-google-drive-writes-pass-1,
finding 1): the `files.update` preflight cannot carry `supportsAllDrives`, because guard `values`
refuse any absent input value (`adapters/catalog/src/lib.rs:475-480`), so a shared-drive update's
preflight reads the file without it; inferred effect: Drive answers 404 and the update is refused
(not observed live).

## Acceptance

- A preflight value may be marked optional; an absent optional value is omitted from the preflight,
  a present one is forwarded.
- `files_update_preflight_reads_under_the_shared_drive_mode_of_the_patch` in
  `adapters/catalog/tests/google_drive_writes_adversary.rs` asserts the forwarded value (flipped
  from today's state).

## Scope

Landed on, from `git diff a64f6396b...impl/catalog-guard-optional-preflight-values` (merged into wave/zero-issues at 29450b8c7); the earlier scoper lines marked inferred are replaced by this list. Generated reference pages are left out.

- `adapters/catalog/providers/google-drive/operations.json`
- `adapters/catalog/spec/ess/domains/guard.yaml`
- `adapters/catalog/src/closed.rs`
- `adapters/catalog/src/lib.rs`
- `adapters/catalog/tests/engine.rs`
- `adapters/catalog/tests/google_drive.rs`
- `adapters/catalog/tests/google_drive_writes_adversary.rs`
- `adapters/catalog/tests/guard_optional_values_adversary_pass2.rs`
- `adapters/catalog/tests/guard_optional_values_adversary.rs`
- `contracts/catalog/v1alpha1/semantics.md`
- `crates/connectors-catalog/src/definition.rs`
- `crates/connectors-catalog/src/import.rs`
- `crates/connectors-catalog/tests/fixtures/definitions/connectors/google-drive.yaml`
- `docs/catalog-google-drive.md`
- `docs/local-catalog-provider.md`
