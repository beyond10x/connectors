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
- confidence: inferred
  path: adapters/catalog/src/lib.rs
revision: 4
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
