---
format: aep.planning-md/3
id: story:catalog-google-gmail-draft-writes
kind: story
status: draft
title: 'Gmail writes through drafts: create a draft, send it by id'
relations:
- decomposes: epic:google-workspace-reads
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-google-gmail-reads
- depends_on: story:cli-oauth-loopback-acquisition
scope:
- confidence: inferred
  path: adapters/catalog/providers/google-gmail/operations.json
- confidence: inferred
  path: adapters/catalog/tests/google_gmail.rs
- confidence: inferred
  path: docs/catalog-google-gmail.md
revision: 5
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
