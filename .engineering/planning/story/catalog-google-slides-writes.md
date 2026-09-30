---
format: aep.planning-md/3
id: story:catalog-google-slides-writes
kind: story
status: implemented
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
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T15:35:20Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-09-30T15:35:21Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:43Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":9,"verification":1}}}
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
- `docs/catalog-google-slides.md` routes writes through a separate write instance connected with `connections connect`, and says `connections repair` cannot add a scope (the configuration revision and profile are part of the connection binding; decided 2026-09-30 from review-result:adversary-google-slides-writes-pass-1, F1).

Create operations carry no guard: the existing guard model needs a preflight and compares by
equality only (`adapters/catalog/src/lib.rs:34-38,308-313`), and nothing exists to compare before a
create. This story does not edit `adapters/catalog/src/lib.rs`.
