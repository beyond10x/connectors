---
format: aep.planning-md/3
id: story:catalog-google-drive-writes
kind: story
status: draft
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
revision: 5
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
