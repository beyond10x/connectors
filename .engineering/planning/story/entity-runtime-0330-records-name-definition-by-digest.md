---
format: aep.planning-md/3
id: story:entity-runtime-0330-records-name-definition-by-digest
kind: story
status: active
title: 'connectors pins Entity Runtime 0.33.0: a record names its definition by digest'
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:store-checkpoints-one-way
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: apps/connectors/tests/cold_owner_start.rs
- confidence: inferred
  path: apps/connectors/tests/cold_owner_start_security.rs
- confidence: inferred
  path: apps/connectors/tests/setup_checkpoints_enable.rs
- confidence: cited
  path: crates/connectors-build/generated/metadata-emissions
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/checkpoint_tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/binding_scope_security_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: crates/connectors-host/tests/fixtures/metadata-store-er-0.32.0
- confidence: inferred
  path: crates/connectors-host/tests/metadata_reopen_adversary.rs
- confidence: cited
  path: docs/development.md
revision: 26
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T07:21:33Z", actor: "human:timo", revision: 24, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "proposed", to: "active", at: "2026-11-23T07:21:34Z", actor: "human:timo", revision: 25, decided_on: {"recorded":{"review_outcome":8}}}
---
## Outcome

connectors builds on Entity Runtime 0.33.0, so a recorded entry names its entity definition by
digest (`er.record/5`) and a read invoke appends at most half of the blob bytes it appends on the
base commit (`main` at connectors 0.75.0, Entity Runtime 0.32.0).

Issue #267 item 2 estimated "about 10×". Entity Runtime measured 4.8× for one record (21,075 to
4,373 bytes). From the 2026-11-23 comment on #267 the read workload gives about 3× (0.6 MB of
blobs without a definition plus 673 × 4.4 KB, of 11.0 MB). The measured ratio is posted on #267.

## Source

Issue #267, wanted item 2. Timo's pick D3 of 2026-11-23 (about 04:57 CET, relayed by
host-operator): "D: A then B (Recommended)", A being this record format, B compaction of settled
read records (story:settled-read-uses-leave-the-store).

Consumer: dev-operator, whose Jira store held 650 MB on 2026-11-23. This pin does not shrink
that store: records already written keep their bytes. It slows the growth of a store from the
first write of the new release on, so a rotated state directory stays small about 3 times as
long. The shrink of an existing store is part B; the 35 s `connections list` is
story:connections-list-answers-through-the-owner.

Entity Runtime 0.33.0 (https://github.com/beyond10x/entity-runtime/releases/tag/0.33.0,
beyond10x/entity-runtime#94; its `CHANGELOG.md`, section 0.33.0):

- `er.record/5` is a decision record whose `definition` is `sha256:<64 hex>`; the definition is
  stored once per tenant as an `er.definition/1` blob, in the same append group as the first
  record naming it.
- `er.record/1` to `/4` stay readable; no rewrite.
- A missing or mismatching definition is refused `ProviderIntegrity`.
- The Eventlog store only. connectors uses it (`Cargo.toml:24`, `entity-eventlog` with `sqlite`
  and `sync-bridge`).
- A store written by 0.33.0 cannot be opened by 0.32.0. Every new record is `er.record/5`: the
  release has no switch that keeps a writer on the older form.
- It builds on Eventlog 0.9.4, as 0.32.0 does (`crates/entity-eventlog/Cargo.toml` at both tags),
  so `Cargo.toml:27` stays.
- No call site of `crates/connectors-host/src/local/metadata/er.rs` must change: the design
  critic compared the 0.32.0 and 0.33.0 sources of what it imports (read, not compiled).

## One-way for an existing store

decision-blocker:store-checkpoints-one-way (cleared 2026-10-07) refused a one-way change that a
new release applies to an existing store by itself: "it would change an existing user's store
irreversibly without an act of its owner". It chose an explicit command for existing stores.

This pin is such a change: the first record the new release writes makes the store unreadable
for connectors 0.75.0 and earlier, and connectors cannot offer an opt-in, because the record form
is the library's and 0.33.0 has no writer switch. Question D4 is with host-operator since
2026-11-23 (options: ship as is with a Breaking entry; or first a writer switch in Entity Runtime
and an explicit command here). The story is built on a branch on the default (ship as is); its
merge waits for the answer.

The store's `LEVEL` (`crates/connectors-host/src/local/metadata/er.rs:48`) stays 9. An older build
answers `metadata_unavailable` on such a store, read from the code (`er.rs:2435`, `:3004`;
`admit_writer` at `metadata.rs:1084` runs after the replay); acceptance 6 runs the store-level check.
Reasons: the durable-checkpoint change of 0.29 was one-way and kept level 9 (6995a513909);
raising it is a store migration in `metadata.rs` (`:386`, `:653`, `:701`, `:995`, `:1208`,
`:1229`, `:1277`); Eventlog appends on its own SQLite handle (`metadata.rs:1044-1047`), so the
raise cannot be atomic with the first `er.record/5`. A named refusal for this case is not part of
this story.

## What the tree holds today

Read by the scoper on 2026-11-23 at b9b56f7bd19:

- Pins: `Cargo.toml:23-26` (`entity-core`, `entity-eventlog`, `entity-executor`, `entity-store`
  at tag 0.32.0); `Cargo.lock` holds those and `entity-query`. ESS brings its own `entity-core`
  0.28.0, which stays.
- `crates/connectors-build/generated/metadata-emissions/inputs.json` carries a digest over every
  `entity-*` line of `Cargo.lock` (`crates/connectors-build/src/metadata_emissions.rs:63`), so
  the pin makes `the_committed_metadata_emissions_are_current`
  (`crates/connectors-build/src/metadata_conformance.rs:966`) red until the three parts are
  regenerated (`docs/development.md:69-74`; 8 to 27 pool-minutes per part).
- No source file parses the record format (`er.record/`, `"definition":{"create"`: not found).
- `read_invoke_cost_by_store_size`
  (`crates/connectors-host/src/local/registry/store_cost_tests.rs:266-385`) is ignored, prints
  events and times per store size (`:361-362`), asserts nothing about cost, saves the store it
  grew (`CONNECTORS_STORE_COST_STORES`, `save` at `:144-176`: `metadata.sqlite3`, `reference`,
  `registry-clock.floor`) and loads one (`load`, `:224-251`). One measured invoke is four
  registry calls (`measured_invoke`, `:20-76`: `admit_read`, `capture_read`, `dispatch_read`,
  `release_read`), with helpers that are `pub(super)` in the lib tests (`registry/tests.rs:858`,
  `:963`).
- The 0.26.0 fixture and its test (`crates/connectors-host/tests/metadata_store_previous_pin.rs`)
  hold acquisitions only, no read invoke; its README says "A later pin move adds a sibling
  fixture written by the pin being replaced".
- `docs/development.md:107-109` names Entity Runtime 0.30.3 on Eventlog 0.8.3; the pins are
  0.32.0 and 0.9.4.

## Acceptance

1. `Cargo.toml:23-26` name tag 0.33.0 and `Cargo.lock` follows; `eventlog-core` stays 0.9.4.
2. The three parts of the metadata emissions are regenerated, each as its own pool job
   (`slot --timeout 30m target/debug/connectors-build metadata-conformance emissions --part
   <component|connectors.oauth2|connectors.approval_issuers>`). In the gate
   `the_committed_metadata_emissions_are_current` passes; the `slow` lane's `--check` passes in
   CI.
3. First commit, before the pin: `read_invoke_cost_by_store_size` prints
   `blob_bytes_per_invoke` on its `events=… invokes=…` line: the growth of
   `sum(length(bytes))` over `connectors_er_blobs` during the measured invokes, divided by the
   measured invokes.
4. The number of line 3 at that first commit and at the pin commit, both with
   `CONNECTORS_STORE_COST_EVENTS=50` and `CONNECTORS_STORE_COST_INVOKES=100` on a store each
   build grew itself and `SLOT_TIMEOUT=30m` on the `cargo test` call, is in the pull request and
   on issue #267; the second is at most half of the first.
5. A fixture `crates/connectors-host/tests/fixtures/metadata-store-er-0.32.0/`: the three files
   `save` writes, made at tag `v0.74.2` (Entity Runtime 0.32.0, the pin being replaced) by
   `read_invoke_cost_by_store_size` with `CONNECTORS_STORE_COST_STORES` and the smallest
   `CONNECTORS_STORE_COST_EVENTS` that leaves at least one read invoke in the store; no network,
   no credential. Its README names the command, the event count N, and the SHA-256 and size of
   each file; the directory is at most 2 MB. A lib test beside the cost test (not ignored) copies
   the fixture, rewrites `local_authority.owner_uid` to the uid running the test as
   `metadata_store_previous_pin.rs:103-111` does, always, loads it with `load`, runs the four
   calls of `measured_invoke` with the `Binding` the cost test published, each returning `Ok`,
   and then counts at least N recorded events (`recorded_events`, `registry/tests.rs:1839-1849`).
6. What the previous pin answers on a store the new build wrote a read into, from a run outside
   the gate with a fixed input: a store saved by `read_invoke_cost_by_store_size` at the pin
   commit (`CONNECTORS_STORE_COST_STORES`, `CONNECTORS_STORE_COST_EVENTS=50`) is loaded by `load`
   in a checkout of `v0.74.2`, in a one-off test that is not committed. The report quotes `Ok` or
   the `Err` text. A run of the 0.74.2 release binary's CLI on such a store is added to the
   report if it was made; it is not required.
7. `CHANGELOG.md` has, under the release, a Breaking entry: what line 6 returned and which run it
   was; the sentence that connectors 0.75.0 and earlier do not open a store once this release
   wrote to it, only if line 6 returned `Err` (or the CLI run exited non-zero); the code such a
   build answers, `metadata_unavailable`, marked as read from the code (`er.rs:2435`, `:3004`)
   unless the CLI run was made; the way back (move the state directory aside and connect again);
   and that records already written keep their size until compaction ships. It also has the pin
   in the form of earlier pins ("Entity Runtime 0.33.0 on Eventlog 0.9.4 (from 0.32.0)").
8. `cargo nextest run -p connectors-host -p connectors -E 'test(checkpoint_tests) |
   test(store_cost_tests) | test(registry::tests) | test(binding_scope_security_tests) |
   binary(metadata_reopen_adversary) | binary(metadata_store_previous_pin) |
   binary(cold_owner_start) | binary(cold_owner_start_security) |
   binary(setup_checkpoints_enable)'` (through the cargo shim, so in the pool) exits 0 with a
   summary time of at most 120 s and no test over 60 s; ignored tests are not run; the report
   gives the host load. `store_cost_tests.rs` differs only by lines 3 and 5, and any other file
   those tests live in that changes is named in the report with the reason.
9. `docs/development.md:107-109` names Entity Runtime 0.33.0 on Eventlog 0.9.4.

Each new test's time is reported; budget 60 s per test, 120 s for the test step. The ignored
timing test stays outside the gate.

## Shares files with

Edges: story:read-cost-test-reports-bytes-per-read depends on this story (this story adds the
print, that one keeps only the bound, taken at the pin); story:settled-read-uses-leave-the-store
depends on that one (the same test function).

Not worked beside this story, by file:

- `inputs.json` through its digest inputs (`Cargo.lock`, `ess/**`,
  `crates/connectors-build/src/metadata_entities.rs`): story:spec-models-operation-admission
  (also `er.rs`, which this story does not edit), story:admission-checks-token-scopes,
  story:definition-files-in-user-config, story:standing-write-approval-for-unattended-agents,
  story:definitions-pin-covers-catalog, story:google-consent-on-the-oauth2-core,
  story:mcp-outbound-stdio-runtime, and any ESS pin. story:invoke-admission-queues-under-load
  regenerated it in 0.75.0; this story starts from `main` after it.
- `docs/development.md`: story:sustained-live-read-loop, story:definitions-pin-covers-catalog,
  story:claude-getting-started-tutorial.
- `CHANGELOG.md`: one entry added under the release heading; a text conflict with any story that
  adds one is resolved at merge.
- `crates/connectors-host/tests/`: story:test-binary-per-package moves
  `metadata_store_previous_pin.rs`; this story reads that file and adds its new test in the lib.

Unassessed, because they name no surface: story:invoke-exits-promptly-on-sigterm,
story:owner-cancel-timing-test-under-load, story:test-binary-per-package.

## Out of scope

- Rewriting old records to `er.record/5` to reclaim their bytes, and compaction (D3 part B):
  story:settled-read-uses-leave-the-store.
- The bound on bytes per read: story:read-cost-test-reports-bytes-per-read.
- The owner's memory after many reads: story:settled-read-uses-leave-the-store reports it.
- A named refusal (`metadata_schema_newer`) for an older build on a newer record form: not
  drafted; it needs a store migration.
- Entity Runtime's File, SQLite and PostgreSQL stores (beyond10x/entity-runtime#96): connectors
  does not use them.

## Decision D4, 2026-11-23

Timo, relayed by host-operator at about 10:55 CET: "A: ship now". The pin ships with the
CHANGELOG Breaking entry and the way back, in connectors 0.76.0, once the Windows host fix
(pull request 284) is merged.

## Result

Acceptance lines 1 to 7 and 9 are met (pull request 286). Line 8 is not met on the
workstation at host load 68 to 85: five tests of its filterset are killed at 60 s, four of
them run 82 to 123 s on Entity Runtime 0.32.0 too. story:store-tests-end-within-60s-under-load
carries them and the filterset.
