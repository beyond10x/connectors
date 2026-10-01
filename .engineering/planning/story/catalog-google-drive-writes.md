---
format: aep.planning-md/3
id: story:catalog-google-drive-writes
kind: story
status: implemented
title: 'Guarded Google Drive metadata writes: create, update, copy'
relations:
- decomposes: epic:google-workspace-reads
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-google-drive-reads
- depends_on: story:cli-oauth-loopback-acquisition
scope:
- confidence: inferred
  path: adapters/catalog/providers/google-drive/operations.json
- confidence: inferred
  path: adapters/catalog/tests/google_drive.rs
- confidence: inferred
  path: docs/catalog-google-drive.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T15:35:21Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-09-30T15:35:21Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:42Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":6,"verification":1}}}
---
## Outcome

Guarded metadata writes to Google Drive through the `google-drive` provider. No content upload
(upload paths are excluded by `story:catalog-discovery-projection`).

## Operations

| Selection id | Discovery id | Guard |
|---|---|---|
| `files.create` | `drive.files.create` | none; metadata body only (Google-native files, folders, shortcuts); the approval binds the whole body |
| `files.update` | `drive.files.update` | preflight `files.get` checks the file's `version` equals the input's expected `version` |
| `files.copy` | `drive.files.copy` | none; the approval binds the whole body |

## Acceptance

- The three ids in `adapters/catalog/providers/google-drive/operations.json` as `effect: write`;
  `adapters/catalog/tests/google_drive.rs` pins them.
- Approval refusals as in `story:catalog-google-slides-writes`; a stale preflight sends no write.
- The connection scope for writes is `drive.file` (files the app created or opened), documented in
  `docs/catalog-google-drive.md`.

Create operations carry no guard: the existing guard model needs a preflight and compares by
equality only (`adapters/catalog/src/lib.rs:34-38,308-313`), and nothing exists to compare before a
create. This story does not edit `adapters/catalog/src/lib.rs`.
