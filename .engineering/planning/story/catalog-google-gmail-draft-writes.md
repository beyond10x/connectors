---
format: aep.planning-md/3
id: story:catalog-google-gmail-draft-writes
kind: story
status: implemented
title: 'Gmail writes through drafts: create a draft, send it by id'
relations:
- decomposes: epic:google-workspace-reads
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-google-gmail-reads
- depends_on: story:cli-oauth-loopback-acquisition
scope:
- confidence: inferred
  path: adapters/catalog/providers/google-gmail/operations.json
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/engine.rs
- confidence: inferred
  path: adapters/catalog/tests/google_gmail.rs
- confidence: inferred
  path: docs/catalog-google-gmail.md
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T18:33:47Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":6,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-09-30T18:33:47Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":6,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:48Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":6,"verification":1}}}
---
## Outcome

Gmail writes through drafts only: a draft is created, then sent by id, so the approval for sending
binds a draft the operator can read first. `gmail.users.messages.send` is not selected.

## Operations

| Selection id | Discovery id | Guard |
|---|---|---|
| `users.drafts.create` | `gmail.users.drafts.create` | none; the approval binds the whole body; `message.raw` is base64url RFC 5322 |
| `users.drafts.send` | `gmail.users.drafts.send` | preflight `users.drafts.get` checks the draft's `message.id` equals the input's expected value |

## Acceptance

- The two ids in `adapters/catalog/providers/google-gmail/operations.json` as `effect: write`;
  `users.messages.send` absent, asserted by `adapters/catalog/tests/google_gmail.rs`.
- Approval refusals as in `story:catalog-google-slides-writes`; a changed draft sends nothing.
- Scope `gmail.compose`, documented in `docs/catalog-google-gmail.md`.

Create operations carry no guard: the existing guard model needs a preflight and compares by
equality only (`adapters/catalog/src/lib.rs:34-38,308-313`), and nothing exists to compare before a
create. This story does not edit `adapters/catalog/src/lib.rs`.
