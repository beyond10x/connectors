---
format: aep.planning-md/3
id: story:entity-runtime-0331-pin
kind: story
status: implemented
title: 'connectors pins Entity Runtime 0.33.1: the execute slowdown of 0.33.0 is gone'
summary: Cargo pins the four Entity Runtime crates at tag 0.33.1; emissions inputs regenerated; execute time measured on 0.32.0, 0.33.0 and 0.33.1; ships as 0.76.2
relations:
- depends_on: story:entity-runtime-0330-records-name-definition-by-digest
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/connectors-build/generated/metadata-emissions
- confidence: cited
  path: docs/development.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T11:40:49Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-23T11:40:49Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-24T06:33:43Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

connectors pins Entity Runtime 0.33.1 and ships it as 0.76.2. The time one read invoke spends in
`er.execute_batch` is back at or below what it was on Entity Runtime 0.32.0, measured.

## Source

- The `entity-runtime` session, 2026-11-23: "Entity Runtime 0.33.1 is out (tag 0.33.1), fixes the
  0.33.0 execute slowdown"; "No format or API change; pin 0.33.1 instead of 0.33.0."
- story:entity-runtime-0330-records-name-definition-by-digest (0.76.0), commit 60c9016d321:
  `er.execute_batch` 314.5 and 350.5 ms per invoke on 0.32.0, 420.2 and 419.6 ms on 0.33.0 (test
  profile, 50 recorded events, 100 invokes).
- D9 (this session, 2026-11-23): 0.76.0 ships with the 0.33.0 pin; this pin follows; first planned as 0.76.1, it ships as 0.76.2 (see Results).

## What the tree holds today

Read at e535fc5f924 (the head of pull request 286, which becomes 0.76.0).

- `Cargo.toml:23-26` pins `entity-core`, `entity-eventlog`, `entity-executor` and `entity-store`
  at tag 0.33.0; `Cargo.lock` follows.
- `crates/connectors-build/generated/metadata-emissions/inputs.json:5-9` lists the five Entity
  Runtime crates at 0.33.0, so its digest changes with the pin
  (`the_committed_metadata_emissions_are_current`).
- `docs/development.md:107` names 0.33.0. `docs/development.md:66-72`: the emissions are
  regenerated one part per pool job (`--part component`, `--part connectors.oauth2`, `--part
  connectors.approval_issuers`), a part took 8 to 27 minutes.
- Fixture `crates/connectors-host/tests/fixtures/metadata-store-er-0.32.0` and its lib test
  `a_store_the_previous_pin_wrote_a_read_into_takes_a_read_invoke`.
- Measurement test `local::registry::store_cost_tests::read_invoke_cost_by_store_size`
  (`#[ignore]`), read through `CONNECTORS_STORE_COST_EVENTS`, `_INVOKES` and `_STORES`.
- Tag `0.33.1` exists on `github.com/beyond10x/entity-runtime` (89e026924bc).

## Design

- `Cargo.toml` names tag 0.33.1 for the four crates; `cargo update` for exactly those packages
  and what they pull from the same repository (`entity-query`).
- `inputs.json` is regenerated. The three emission parts are checked byte for byte with
  `--check`; a part that differs is regenerated and the report names it.
- No fixture for 0.33.0: the format did not change between 0.33.0 and 0.33.1 (the sender's
  word). It is checked once: acceptance 6 runs the 0.33.1 build on a store
  the 0.33.0 build grew.
- No call site changes. If the build needs one, the implementor stops and reports it.

## Acceptance

Builds and tests go through the pool (`cargo` is wrapped; `SLOT_TIMEOUT=30m` for a build, `slot
--timeout 30m` for a binary). The regeneration and the measurement are not tests and run as pool
jobs of at most 30 minutes each. `<base>` is `git merge-base origin/main HEAD` (e535fc5f924
until pull request 286 is merged).

1. `rg -c 'entity-runtime", tag = "0.33.1"' Cargo.toml` prints 4; `rg -c 'tag = "0.33.0"'
   Cargo.toml` prints nothing; `rg -c 'entity-runtime\?tag=0\.33\.1#' Cargo.lock` prints 5; `rg
   -c 'entity-runtime\?tag=0\.33\.0' Cargo.lock` prints nothing; `git diff -U0 <base> --
   Cargo.lock` changes only lines of the five `entity-*` packages.
2. `cargo test --locked -p connectors-build --bin connectors-build
   the_committed_metadata_emissions_are_current` passes, and `git diff --name-only <base> --
   crates/connectors-build/generated/metadata-emissions` prints `inputs.json` alone or nothing.
   When the test passes right after the pin with no regeneration (the crates' versions or the
   recorded digest did not move), no part is regenerated and the report says so. Otherwise
   each of `--part component`, `--part connectors.oauth2` and `--part
   connectors.approval_issuers` is regenerated as its own pool job (`docs/development.md:66-72`).
3. Guard (it passes before the pin too): `cargo test --locked -p connectors-host --lib
   a_store_the_previous_pin_wrote_a_read_into_takes_a_read_invoke` passes.
4. Guard: the four lib tests and the one integration test that
   story:store-tests-end-within-60s-under-load names are each run alone from their built
   binary with `slot --timeout 30m <binary> --exact <test>`; each passes and the report gives
   each time. They are not run through nextest here, which kills them at 60 s under load.
5. Measurement, release profile. Three lib test binaries of `connectors-host`, built in this
   checkout by changing only `Cargo.toml` and `Cargo.lock`: Entity Runtime 0.32.0 (the two
   files of 60c9016d321~1), 0.33.0 (e535fc5f924) and 0.33.1 (this branch); each binary is
   copied out of `target/` before the next build. Each grows its own store once
   (`CONNECTORS_STORE_COST_EVENTS=50`, `CONNECTORS_STORE_COST_INVOKES=100`,
   `CONNECTORS_STORE_COST_STORES=<its own directory>`) and then runs
   `local::registry::store_cost_tests::read_invoke_cost_by_store_size` three times, the three
   binaries at the same time, in rotating start order. The report gives, per binary, the three
   values and the median of `er.execute_batch` ms per invoke and `blob_bytes_per_invoke`, and
   the `uptime` line of each round. Met when the 0.33.1 median is at most 1.10 times the 0.32.0
   median and below the 0.33.0 median. Not met: the pin is still made (no format change, the
   sender's word), the changelog line carries the medians as measured, and they go to
   `entity-runtime`.
6. The 0.33.1 binary of line 5 runs the same test once on a copy of the store the 0.33.0 binary
   grew: exit 0, and the report gives its `blob_bytes_per_invoke`.
7. `rg -c '0\.33\.0' docs/development.md` prints nothing and `rg -n 'Entity Runtime 0\.33\.1'
   docs/development.md` prints one line. `CHANGELOG.md` gains `## Unreleased` with a `###
   Changed` line that names Entity Runtime 0.33.1 and the three medians of line 5.
8. Before the release commit that the release script makes, `git diff --name-only <base>` lists
   only `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `docs/development.md`, files under
   `crates/connectors-build/generated/metadata-emissions/` and `.engineering/`.
9. `gh pr checks <number>` shows every required check passing, the Windows host check excepted
   while story:windows-mcp-tool-call-answers-timeout is open.
10. `git ls-remote --tags origin v0.76.2` prints one line.

Not checked by any line, known: a store written by a released 0.76.0 binary has no fixture
(line 6 reads a store the 0.33.0 test build grew); a binary with the 0.33.0 pin and one with the 0.33.1 pin on one store.

## Shares files with

- `CHANGELOG.md`: story:settled-read-uses-leave-the-store,
  story:connections-list-answers-through-the-owner, story:windows-mcp-tool-call-answers-timeout.
  This pin merges first; the others rebase.
- `inputs.json`: story:local-invoke-admits-a-keyed-write-under-a-standing-grant, which
  regenerates after this pin is on `main`.

## Out of scope

- Any change in the entity-runtime repository. A fixture store written by 0.33.0.

## Results (2026-11-23)

Release number: 0.76.1 went to story:settled-read-uses-leave-the-store alone (D5, decided
after D9). This pin ships with the release after it, 0.76.2; lines 10 and the Outcome read
that way.

Deviation from the Acceptance preamble: the measurement of lines 5 and 6 ran outside the pool.
`~/beyond10x/AGENTS.md` § "Scripts that do not build stay off slot" says a script that
measures starts without the `slot` prefix, and the pool held 83 queued jobs at 17:55 CET (the
third release build waited 1,797 s for a slot). The three builds of line 5 and the five runs
of line 4 ran as pool jobs.

Line 4, each test alone from its built binary, all exit 0: 
`store_cost_tests::a_reopen_after_its_own_write_refuses_every_blob_altered_since` 58.19 s,
`tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once` 134.78 s,
`tests::a_reconnect_below_the_refresh_retention_bound_is_not_refused_capacity` 100.80 s,
`tests::concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence` 149.11 s, and
the integration test `a_reopen_refuses_every_altered_blob_a_full_replay_refuses` 42.42 s.

Line 5, met. 50 recorded events, 100 read invokes, release profile, the three binaries at the
same time, 17:56 to 17:59 CET. Load average (1 min) at the start and end of each round:
66.83 to 64.80, 64.80 to 61.28, 61.28 to 52.05.

| Entity Runtime | `er.execute_batch` ms per invoke, rounds 1 to 3 | median | `blob_bytes_per_invoke` | median CPU ms per invoke, rounds 1 to 3 |
|---|---|---|---|---|
| 0.32.0 | 121.0, 122.9, 126.8 | 122.9 | 78,488 | 143, 140, 138 |
| 0.33.0 | 124.6, 129.5, 131.7 | 129.5 | 24,500 | 192, 188, 182 |
| 0.33.1 | 105.1, 110.3, 125.0 | 110.3 | 24,500 | 177, 175, 171 |

The 0.33.1 median (110.3 ms) is 0.90 times the 0.32.0 median and below the 0.33.0 median.
Not asked by line 5 and still open: the CPU time of one read invoke is higher with 0.33.1
(median 175 ms) than with 0.32.0 (140 ms).

Line 6, met: the 0.33.1 binary on a copy of the store the 0.33.0 binary grew ended exit 0
with `blob_bytes_per_invoke=24500`, `appended_per_invoke=3`, `failed=0` (18:01 CET, load 52 to
77, `er.execute_batch` 180.8 ms per invoke in that one run).
