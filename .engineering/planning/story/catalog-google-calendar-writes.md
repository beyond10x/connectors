---
format: aep.planning-md/3
id: story:catalog-google-calendar-writes
kind: story
status: implemented
title: 'Guarded Google Calendar event writes: insert, patch, delete'
relations:
- decomposes: epic:google-workspace-reads
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-google-calendar-reads
- depends_on: story:cli-oauth-loopback-acquisition
scope:
- confidence: inferred
  path: adapters/catalog/providers/google-calendar/operations.json
- confidence: inferred
  path: adapters/catalog/tests/google_calendar.rs
- confidence: inferred
  path: docs/catalog-google-calendar.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T16:44:52Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-09-30T16:44:52Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:47Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":5,"verification":1}}}
---
## Outcome

Guarded writes to Google Calendar through the `google-calendar` provider.

## Operations

| Selection id | Discovery id | Guard |
|---|---|---|
| `events.insert` | `calendar.events.insert` | none; the approval binds the whole body |
| `events.patch` | `calendar.events.patch` | preflight `events.get` checks `etag` equals the input's expected value |
| `events.delete` | `calendar.events.delete` | preflight `events.get` checks `etag` |

The approval binds the whole input, including `sendUpdates`.

## Acceptance

- The three ids in `adapters/catalog/providers/google-calendar/operations.json` as `effect: write`;
  `adapters/catalog/tests/google_calendar.rs` pins them.
- Approval refusals as in `story:catalog-google-slides-writes`; a stale `etag` sends no write.
- `approval_binds_send_updates`: an approval issued for input with `sendUpdates: none` refuses an
  invoke of the same event with `sendUpdates: all`, and no request is sent.
- Scope `calendar.events`, documented in `docs/catalog-google-calendar.md`.

Create operations carry no guard: the existing guard model needs a preflight and compares by
equality only (`adapters/catalog/src/lib.rs:34-38,308-313`), and nothing exists to compare before a
create. This story does not edit `adapters/catalog/src/lib.rs`.
