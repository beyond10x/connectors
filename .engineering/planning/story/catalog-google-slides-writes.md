---
format: aep.planning-md/3
id: story:catalog-google-slides-writes
kind: story
status: draft
title: 'Guarded Google Slides writes: create and batchUpdate'
relations:
- decomposes: epic:google-workspace-reads
- depends_on: story:cli-oauth-loopback-acquisition
- depends_on: story:catalog-google-slides-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/providers/google-slides/operations.json
- confidence: inferred
  path: adapters/catalog/tests/google_slides.rs
- confidence: inferred
  path: docs/catalog-google-slides.md
revision: 5
---
## Outcome

Guarded writes to Google Slides through the `google-slides` provider, each `effect: write` and
refused without an approval under the existing write model (`docs/local-catalog-provider.md:141-164`).

## Operations

| Selection id | Discovery id | Guard |
|---|---|---|
| `presentations.create` | `slides.presentations.create` | none; the approval binds the whole body |
| `presentations.batchUpdate` | `slides.presentations.batchUpdate` | preflight `presentations.get` checks `revisionId` equals the input's `writeControl.requiredRevisionId`; the approval binds the whole request body |

## Acceptance

- Both ids in `adapters/catalog/providers/google-slides/operations.json` as `effect: write`;
  `adapters/catalog/tests/google_slides.rs` pins them.
- A write without an approval is refused before any request; a write with an approval for a
  different body is refused.
- Fixture test: `batchUpdate` with a stale `requiredRevisionId` fails the preflight and sends no write.
- `slides_write_config_requires_presentations_scope`: an instance whose `minimum_scopes` includes
  `https://www.googleapis.com/auth/presentations` refuses validation when the token response grants
  only `presentations.readonly` (the existing `minimum_scopes` check, `adapters/catalog/src/local.rs:514-516`).
- `docs/catalog-google-slides.md` names `connections repair` as the way to add a write scope.

Create operations carry no guard: the existing guard model needs a preflight and compares by
equality only (`adapters/catalog/src/lib.rs:34-38,308-313`), and nothing exists to compare before a
create. This story does not edit `adapters/catalog/src/lib.rs`.
