---
format: aep.planning-md/3
id: story:list-commands-page-with-a-cursor
kind: story
status: implemented
title: adapters list and operations list page with a cursor instead of refusing with capacity
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/Cargo.toml
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/cursor.rs
- confidence: cited
  path: apps/connectors/src/local/operations.rs
- confidence: cited
  path: apps/connectors/tests/list_paging.rs
- confidence: cited
  path: apps/connectors/tests/list_paging_adversary.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T14:19:49Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-09-30T14:19:50Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-09-30T16:53:26Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Observed
Found by the black-box CLI surface test of release 0.18.0 on 2026-09-30 (raw output under the tester's sandbox, outside the repository).
`adapters list --limit=2` with 3 entries answers `capacity` / `retry_explicitly` (exit 1) and never issues
`next_cursor`; `operations list --limit=1` the same at stage `admission`. A retry can never succeed.
Contradicts contracts/cli/v1alpha1/semantics.md:300 and :303-304; apps/connectors/src/local.rs:285-293 says "No cursor owner yet".
## Acceptance
- A list with more entries than `--limit` returns a page and a `next_cursor`; following it returns the rest and no cursor.
- A stale or foreign cursor is refused as `stale_cursor`.

## Decided (coordinator, 2026-09-30)

- The CLI side already declares `cursor`/`next_cursor` and `stale_cursor` (`ess/domains/cli.yaml:402-412,545-558`,
  generated binding). The missing part is issuing and checking cursors for these two lists.
- The cursor is self-contained: base64url of `{version, selection digest, offset, issued_at}`, where the digest
  covers the list's source (configuration revision for `adapters list`; descriptor revision and permitted
  operations for `operations list`) and the page limit. It is refused as `stale_cursor` when the digest no longer
  matches, when it is older than 300 s (semantics.md:300-304), or when it does not decode. No stored entity and
  no ESS entity: altering a cursor can only reveal entries the caller may already list.
- The `capacity` refusal on a long list goes away; the stage of `adapters list` refusals is aligned with
  `operations list` where the contract says so.

## Also here (moved from story:cli-surface-minor-findings-0-18-0, D11)

`operations list --limit=0` answers `description_unavailable`: `owner::cached` (`apps/connectors/src/local/operations.rs:25`)
runs before the bounds check (`:31-37`). Check `--limit` before reading the cached description.
