---
format: aep.planning-md/3
id: story:an-approved-write-survives-one-refused-clock-acquisition
kind: story
status: implemented
title: An approved write survives one refused clock acquisition
summary: A below-quorum clock acquisition gets one second attempt when one query deadline and the caller's reserve remain, in the four paths that sign or settle a write
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: contracts/service/clock.md
- confidence: cited
  path: crates/connectors-host/src/local/clock.rs
- confidence: cited
  path: crates/connectors-host/src/local/clock/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/execution.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/recovery.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: cited
  path: crates/connectors-host/tests/fixtures/clock/server.rs
- confidence: cited
  path: docs/design/approval-clock-quorum.md
- confidence: cited
  path: docs/local-clock.md
- confidence: cited
  path: website/docs/reference/contracts/clock.md
revision: 26
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T07:14:41Z", actor: "human:timo", revision: 23, decided_on: {"recorded":{"review_outcome":11}}}
- {from: "proposed", to: "active", at: "2026-11-24T07:40:09Z", actor: "human:timo", revision: 24, decided_on: {"recorded":{"review_outcome":11}}}
- {from: "active", to: "implemented", at: "2026-11-24T10:25:05Z", actor: "human:timo", revision: 26, decided_on: {"recorded":{"test_result":1,"review_outcome":13}}}
---
## What happened

2026-11-24, 03:58 to 04:42 CET, on the workstation: Slack posts through an approved write
(`approvals issue`, then `invoke chat.postMessage`, connectors 0.74.0) failed on and off before
any provider request. 12 of 12 attempts of a 3-minute loop failed from 03:58:34 to 04:16:59, with
single successes and failures after it. The channel was the operator's only one that night.

```
{"step":"issue","code":"unavailable","kind":"operational","next_action":"retry_status","stage":"readiness"}
{"step":"invoke","code":"unavailable", ... "classification":"not_attempted" ... "stage":"admission"}
```

## Cause (measured and read from the code)

- The approval clock asks every configured Roughtime source once, in parallel, under one
  two-second deadline, and does not repeat (`crates/connectors-host/src/local/clock.rs:28`,
  `:373-378`; `contracts/service/clock.md:94-99`).
- `approvals issue` acquires the clock once (`owner/approval_issuance.rs:391-393`). The owner's
  write path acquires it again through `time()` (`owner/mutation/execution.rs:84-97`), which the
  path calls twice per write (`:297`, `:433`). One approved write needs three acquisitions in a
  row, one in the `issue` command and two in the `invoke` command.
- Of the four sources of `docs/local-clock.md`, two did not answer at all: Cloudflare at wire
  version `roughtime-0x8000000c`, and `time.txryan.com`, whose address had moved from
  148.113.255.71 to 149.56.214.17 (same key). `quorum = 2` was therefore two of two.
- The two answering sources each dropped out for some seconds at a time: 20 checks 4 s apart,
  04:40:10 to 04:41:28 CET, gave 18 ok and 2 `below_quorum`; 8 rounds of 3 parallel checks gave
  0 of 3, 1 of 3, then 6 rounds of 3 of 3. Ping to both showed no loss.
- The dropouts follow the host's uplink: 90 checks 4 s apart with three answering sources,
  04:44:45 to 04:50:48 CET. While the uplink sent 128 to 138 Mbit/s (a copy to a build server),
  23 of 117 source answers were missing and 6 of 39 checks were refused; under 62 Mbit/s, 2 of
  153 answers were missing and 1 of 51 checks was refused. Host load was the same in both
  (median 42 and 41).
- The refusal the caller sees names no source and no `clock_refusal`: `acquire()` drops the
  detail that `acquire_explained()` has (`clock.rs:377`), and `Error::clock`
  (`owner.rs:261-272`) answers a bare `unavailable`.
- None of 0.74.1 to 0.77.1 changed this code.

Done already: the callers' post script retries the two refusals that carry no write; the seven
local configurations and, since 0.77.1, `docs/local-clock.md` and `docs/catalog-slack.md` hold
the address 149.56.214.17 for `time.txryan.com`.

## What changes

1. A new entry point of the approval clock, `acquire_within(until, reserve)`, on
   `clock::Configuration` (beside `acquire_explained`, `clock.rs:381`) and on `ApprovalClock`
   (beside `:506`). It returns `Result<BoundedClock, Refused>` like `acquire_explained`; a caller
   keeps today's mapping, `Error::clock(selected, refused.failure)`.
   - It runs one attempt, which is `acquire_explained()` unchanged.
   - It runs one second attempt when all three hold: the first ended with the refusal
     `BelowQuorum` (`clock.rs:166-174`); no source report of it is `unverified`; and after the
     first attempt `until - now >= query deadline + reserve`. The query deadline is 2 s
     (`clock.rs:28`). An attempt never follows the second.
   - The decision of the third condition is one function of `(now, until, query deadline,
     reserve)` with no clock read inside, so a test checks it with constructed instants.
   - The second attempt connects new sockets and sends new nonces (`clock.rs:386`, `:544`). The
     answer or the refusal is that of the second attempt alone. No reply to the first attempt
     is counted in the second.
   - Not repeated: `Split`, `TooImprecise`, a `BelowQuorum` with an `unverified` source, a local
     timer, continuity or calendar refusal, `invalid_configuration`.
2. The reserve is what the caller still does inside `until` after the acquisition. It keeps a
   refusal of the second attempt, and the work after an answer, inside the caller's deadline:
   without it the second attempt can end at `until`, and the caller's next `check(until)`
   (`owner/approval_issuance.rs:26-31`) turns today's refusal into `Timeout`. In `issue` and the standing
   reading a refusal returns at once (`:391-393`, `:632-634`), so there the reserve covers the
   work after an answer only. Two reserves in `clock.rs`:
   - `RESERVE_AFTER_CLOCK`, a constant of 1 s.
   - the reserve before the second reading, a function: `query_timeout() +
     RESERVE_AFTER_CLOCK`, computed at the call (3 s in production, 1.5 s in a test that set
     500 ms). It is one attempt of the write's second reading plus the constant.
3. Four callers (five call sites) take the entry point, each with the `until` it already holds:

   | caller | lines | reserve | what follows the acquisition |
   |---|---|---|---|
   | `approvals issue` | `owner/approval_issuance.rs:391-393` | `RESERVE_AFTER_CLOCK` | resolve, policy and key lease, signing (`:394-423`) |
   | reading of the standing commands | `owner/approval_issuance.rs:626-635` | `RESERVE_AFTER_CLOCK` | `standing::reading`, `check(until)`, the signed grant file |
   | `time()` at `owner/mutation/execution.rs:297` | `:84-97` | the reserve before the second reading | native preflight, the second reading, dispatch |
   | `time()` at `owner/mutation/execution.rs:433` | `:84-97` | `RESERVE_AFTER_CLOCK` | resolve, ledger preparation, dispatch |
   | recovery of a `Prepared` write | `owner/mutation/recovery.rs:44-55` | `RESERVE_AFTER_CLOCK` | policy lease, resolve, `recover`, the final `observe` (`:57-74`) |

   `time()` gets the reserve as a parameter. The reserve is no budget for dispatch: a write
   whose provider call needs more than what is left ends as it does today.
4. Two callers keep one attempt: `approvals clock-check` (`apps/connectors/src/local.rs:873`),
   which exists to show a dropout (`docs/local-clock.md:91-92`, "asks every source once"), and
   the maintenance scan (`owner/maintenance.rs:106-111`), which drops a refusal and scans again
   later. `acquire()` and `acquire_explained()` do not change.
5. Texts:
   - `contracts/service/clock.md:98-99`: the sentence "There are no automatic network retries."
     is replaced by one sentence on the same two lines: one acquisition makes one attempt, and
     the callers named under `[second attempt](#second-attempt)` make at most one more. The
     sentence holds the words "second attempt". The file keeps its line numbers up
     to its last line (235), because `ess/domains/cli.yaml:189` cites `clock.md:163` and
     `docs/design/approval-clock-quorum.md` cites its lines.
   - `contracts/service/clock.md`, a new section `### Second attempt` after the last line of
     the file (inside `## CLI and verification`; the place is the price of unchanged line
     numbers): which callers repeat, the three conditions, the two reserves, in words that hold "second attempt" in lower case, that the network
     wait of one acquisition is at most two deadlines, that attempts are not merged and each
     uses its own sockets and nonces, and that `approvals clock-check` and the maintenance scan
     make one attempt.
   - `website/docs/reference/contracts/clock.md`: the committed rendering of that file
     (`website/publication.json:6`; its line 114 holds the replaced sentence). It is written by
     `cargo run --locked -p connectors-docs -- generate`, not by hand.
   - `docs/design/approval-clock-quorum.md:133-134` ("There are no retries. The total network
     wait is therefore 2,000 ms whatever the number of sources, the same bound as today.":
     both sentences are replaced; the wait is 4,000 ms at most for the four callers) and `:346`
     (the network-attacker row: one drop per source refuses today, two after the change). Line 18
     is the row of the single-source design this document replaced; it stays.
   - The doc comment `clock.rs:373-375` ("or retry occurs"): it names the new entry point.
   - `docs/local-clock.md`: lines 91-92 speak of `clock-check` and stay. A new sentence after
     the refusal table says that "automatic retry" at line 92 speaks of `clock-check` alone,
     and that `approvals issue`, the standing commands and a write make a second attempt
     themselves.
   - `CHANGELOG.md`: one entry under `## Unreleased`, `### Fixed`. The story creates either
     heading only where the base lacks it.
6. Readers of the changed contract text, none of which changes:
   `contracts/cli/v1alpha1/semantics.md:123-124` (`clock-check` "performs the bounded
   authenticated exchange specified in the clock binding": still one attempt),
   `contracts/service/local-mutations.md:161`, `:190`, `:287` (a fresh reading and a settlement
   clock: a second attempt is a fresh reading), `ess/domains/clock.yaml:73` ("no reply within
   the shared deadline": true of each attempt), `docs/local-clock.md:91-92` and
   `apps/connectors/tests/local_cli.rs:325`. No CLI answer member changes. `contracts/cli/**`,
   `ess/**` and `apps/connectors-cli-contract/**` are not touched.

## What the repeat costs

- One acquisition waits at most 4 s on the network (2 s today).
- `approvals issue` and a standing command hold one acquisition: at most 4 s of their deadline.
- An `invoke` of a write holds two: at most 8 s (4 s today) of its deadline, which is 20 s from
  the CLI. At least 12 s remain for native preflight and dispatch.
- A source receives at most two requests per acquisition.

## Why a repeat gives an attacker on the network path nothing

Ruling of the design critic, round 1, with the lines it rests on:

- Every attempt sends a new nonce (`clock.rs:544`; `contracts/service/clock.md:92-93`) over a
  new socket (`clock.rs:386`). The counted set and the answer come from one attempt
  (`clock.rs:432-449`; `clock.md:117-118`), so a reply to the first attempt cannot serve the
  second.
- An injected or wrong-nonce datagram ends an exchange `unverified` (`clock.rs:585-587`), and
  an attempt with an `unverified` source is not repeated.
- An attacker who can only drop packets refuses the write today with one drop per source and
  needs two after the change.
- A response is no larger than the request (`clock.md:98`): no amplification; a source gets at
  most two requests.

## Acceptance

Fixture (`crates/connectors-host/tests/fixtures/clock/server.rs`): one new constructor,
`Server::dropping(fixture)`. It is warmed like `Server::fixture` (`:76-80`), so the test's
deadline never pays for the serving thread's cold start. It carries a counter the test sets;
while the counter is over 0 the source counts the request (`requests`), stays silent and lowers
the counter by 1. "Drops its next request" below means: the test sets that counter to 1.

Every test below sets the test query deadline (`set_test_query_timeout`, `clock.rs:36-48`, a
thread-local read on the calling thread at `clock.rs:387-389`) to 500 ms on its own thread.
Every acquisition under test runs on the test's thread.

In `crates/connectors-host/src/local/clock/tests.rs` (`until` is 30 s away unless stated;
`reserve` is 300 ms):

1. `a_below_quorum_acquisition_is_repeated_once`: two new sources, each drops its next request,
   quorum 2. `acquire_within` returns a clock and each source's `requests` is 2. Red before
   the change: the entry point does not exist.
2. `one_attempt_refuses_where_two_answer`: two other new sources of the same kind.
   `acquire()` is `Unavailable` and each source's `requests` is 1. Green before and after: it
   pins what `acquire()` keeps doing.
3. `sources_that_never_answer_end_after_two_attempts`: two silent sources. The refusal is
   `BelowQuorum` and each source's `requests` is 2.
4. Same test: the call returns within 2 × 500 ms + 1 s.
5. `an_attempt_with_an_unverified_source_is_not_repeated`: one source answers with a bad
   signature, one is silent. The refusal is `BelowQuorum`, the first source's report is
   `unverified`, and each source's `requests` is 1.
6. `a_split_is_not_repeated`: two sources whose intervals do not meet. The refusal is `Split`
   and each source's `requests` is 1.
7. `the_second_attempt_needs_one_deadline_and_the_reserve`: the decision function of change 1,
   with constructed instants and no wait: false at `query deadline + reserve − 1 ms` left, true
   at `query deadline + reserve` left, false when `until` has passed.
8. `no_second_attempt_starts_inside_the_reserve`: two silent sources, `until` 1 s after the
   call (after the first attempt about 500 ms are left; 800 ms are needed). The refusal is
   `BelowQuorum`, each source's `requests` is 1, and the call returns before `until`.
9. `the_second_attempt_sends_new_nonces`: a source records the `NONC` field of request 0 and
   request 1; they differ.
10. `a_late_reply_to_the_first_attempt_is_not_counted_in_the_second`: quorum 2. Source A
    answers request 0 after 600 ms and never answers request 1. Source B drops its next request
    and answers after it. The refusal is `BelowQuorum` and A's report is `unreachable`.

Through the owner. Each test sets the 500 ms deadline itself, in `owner/mutation/tests.rs`
after `site()`, which sets 50 s (`:718`). `until` is 50 s (`WAIT`) in `owner/mutation/tests.rs`
and 120 s (`until()`) in `owner/approval_issuance/tests.rs`, unless stated. `write()`
(`owner/mutation/tests.rs:932`) or a sibling of it takes an `until` and an optional revoke
step and keeps the credential seam; where `site()` gains a parameter, its two callers
(`:1049`, `:1069`) are edited:

11. `owner/approval_issuance/tests.rs`,
    `issue_asks_again_when_every_source_drops_its_first_request`: `issue` runs in two sites
    built the same way (`setup`, `fake_issuer()`, a policy, `prepare`), both on
    `Server::dropping` sources. In the first no request is dropped; in the second each source
    drops its next request. Both calls end the same way (both `Ok`, or both `Err` with the
    same `code`), and in the second site each source's `requests` is 2. Red before the
    change: the second call ends `Unavailable` and each `requests` is 1. The test does not ask
    for a written proof: signing needs platform custody
    (`crates/connectors-host/src/local/approval_keys.rs:641-655`), and the one test that signs is `#[ignore]`
    (`owner/approval_issuance/tests.rs:596-598`).
12. `owner/approval_issuance/tests.rs`,
    `the_standing_reading_asks_again_when_every_source_drops_its_first_request`: the test calls
    `standing_reading` (`owner/approval_issuance.rs:617`, private; the `tests` module is its
    child) with the configuration of `setup` and a copy of its adapter whose `standing_grants`
    names a path in the test's directory (`setup` sets `None`, `tests.rs:106`). The answer is
    `Ok` and each source's `requests` is 2. Red before the change: `Unavailable`.
13. `owner/mutation/tests.rs`, module `recheck`,
    `a_granted_write_is_dispatched_once_when_the_sources_drop_the_first_request_of_the_write`:
    the standing-grant route of that module (`proof: None`, `tests.rs:986`). `Site` gets its
    sources from `Server::dropping` (a parameter of `site()` or a sibling constructor) and the
    write runs without a revoke step. After `site()` each source drops its next request. The delivery is admitted.
    Red before the change: `unavailable` with `not_attempted`.
14. Same test: `count(&site, "send") == 1` (`tests.rs:1004`).
15. `owner/mutation/tests.rs`, module `recheck`,
    `recovery_settles_a_prepared_write_when_the_sources_drop_the_first_request`: the test
    prepares one attempt with the two functions recovery itself uses, `candidate` and `store`
    (`owner/mutation.rs:380-388`; `recovery.rs:26-27`), and `ledger::Store::prepare` or
    `prepare_approved` (`mutations.rs:123-138`). Each source then drops its next request. The
    test calls `recover_observation` on its own thread. `Quiescent` has private fields
    (`owner/supervisor.rs:332-335`), so `supervisor.rs` gets a constructor for it under `#[cfg(all(test, unix))]`, the gate
    `supervisor.rs:583` uses (module `recheck` is `#[cfg(unix)]`, `tests.rs:668-669`); the test
    passes `&mut None` for the child. The answer is `Ok`, its `mutation.cause` is not `unavailable` at stage `observation`, the
    attempt's state in the ledger is no longer `Prepared`, and each source's `requests` is 2.
    Red before the change: the cause is `unavailable` at stage `observation`
    (`recovery.rs:83-86`), the state is `Prepared` and each `requests` is 1.

Guards for the reserve, green before and after, red when the reserve of the call site is 0:

16. `owner/mutation/tests.rs`, module `recheck`,
    `a_write_refused_inside_the_reserve_answers_as_today`:
    silent sources after `site()`, `until` 2 s after the call (about 1.5 s are left after the
    first attempt; 2 s are needed). The delivery is `unavailable` with `not_attempted`, not an
    `Err(Timeout)`, and each source's `requests` is 1.
17. `owner/mutation/tests.rs`, module `recheck`,
    `recovery_refused_inside_the_reserve_answers_as_today`: the
    prepared attempt of statement 15, silent sources, `until` 1.7 s after the call (about
    1.2 s are left; 1.5 s are needed). The answer is `Ok` with the cause `unavailable` at
    stage `observation`, and each source's `requests` is 1.

Guards for the callers that keep one attempt:

18. `apps/connectors/tests/local_cli.rs:325` (`clock_check_below_quorum_refusal`): one silent
    source serves the test's rows; after each row that names it, its `requests` has grown
    by 1. Command:
    `cargo nextest run --locked -p connectors -E 'test(clock_check_below_quorum_refusal)'`.
19. `rg -n 'acquire_within' crates/connectors-host/src/local/owner/maintenance.rs
    apps/connectors/src/local.rs` finds no line.
20. `git diff --name-only <base>` lists no file under `contracts/cli`, `ess` or
    `apps/connectors-cli-contract`, and under `website/docs/reference/contracts` only
    `clock.md`.

Texts:

21. `rg -n 'no automatic network retries' contracts/service/clock.md
    website/docs/reference/contracts/clock.md` finds no line.
22. `rg -c 'second attempt' <file>` is at least 1 for each of `contracts/service/clock.md`,
    `website/docs/reference/contracts/clock.md`, `docs/local-clock.md`,
    `docs/design/approval-clock-quorum.md`, `crates/connectors-host/src/local/clock.rs` and
    `CHANGELOG.md`.
23. `rg -n 'There are no retries|same bound as today' docs/design/approval-clock-quorum.md`
    finds no line.
24. `sed -n '163p' contracts/service/clock.md` prints the same line before and after the
    change, and `rg -n '^### Second attempt' contracts/service/clock.md` finds one line, with
    a number over 235.
25. `cargo run --locked -p connectors-docs -- generate --check` exits 0.

Budget:

26. `cargo nextest run --locked -p connectors-host -E 'test(/repeated_once|one_attempt_refuses|two_attempts|not_repeated|second_attempt|inside_the_reserve|late_reply|asks_again|drop_the_first_request/)'`
    runs the 15 new tests of the change and prints each one's time; each is under 8 s. A test
    over 5 s is named with its time in the implementor's report. Since the review the filter
    also selects the review cases, whose module paths hold `second_attempt`
    (`second_attempt_adversary_tests`, `second_attempt_security_tests`).
27. `cargo clippy --locked -p connectors-host -p connectors --all-targets --target
    x86_64-pc-windows-gnu -- -D warnings` exits 0 with the new tests: the file-level
    `#![allow(dead_code)]` of the fixture (`server.rs:4`) covers its new constructor in the
    three crates that include it (`clock.rs:6`, `apps/connectors/tests/local_cli.rs:2`,
    `adapters/catalog/tests/main.rs:9`), and the `Quiescent` constructor of statement 15 is
    not compiled for Windows.

## Decided

- The repeat is a new entry point, not a change of `acquire()` or `acquire_explained()`:
  `clock-check` must show a dropout, and its guide says "once" (`docs/local-clock.md:91`;
  design critic, round 1).
- All four callers that sign or settle a write take it; the first draft named two.
- The second attempt starts only with one deadline plus the caller's reserve left (design
  critic, round 2): a repeat must not turn a refusal into `Timeout`.
- `issue` and the standing reading are checked by the request count, not by a written proof or
  a `grant_id`: neither can be produced in the test step (acceptance critic, round 2).
- One acquisition per write is not in this story (see below): the second reading of a write is
  taken after native preflight on purpose (`execution.rs:427-433`), and a bounded clock refuses
  any reading 30 s after its sources answered (`clock.rs:817-818`; `clock.md:167-168`).

## Not in this story

- story:a-clock-refusal-of-issue-and-invoke-names-its-sources (draft): `clock_refusal` and
  `clock_sources` on the refusals of `approvals issue` and `invoke`. It changes the CLI
  contract and its generated files.
- story:an-approved-write-acquires-the-clock-once (draft): the write path reads its first clock
  again where it is still inside the 30 s window.
- Admitting RFC 10049's wire version (`roughtime-1`), which the Cloudflare source may need.
- A clock kept by the owner across writes.
- A second attempt of the write's second reading in its own test: statement 13 covers `time()`,
  which both readings call.

## Shares files with

- story:standing-grant-binds-a-pattern-or-a-value-set (branch `story/pattern-grant`, read at
  its head `d45310985` plus uncommitted changes, 2026-11-24 08:10 CET; the source to read again
  is that story's section "Order and shared files"):
  - `CHANGELOG.md`: that story creates `## Unreleased` / `### Added`. The second to merge
    keeps one `## Unreleased` heading and adds its entry to the `### Fixed` section the base
    has, or creates that section where the base has none.
  - `owner/mutation/tests.rs`: that story's diff of this file is one line (`:577`,
    `Bound::Value`) at that head. Statements 13 to 17 edit `site()` and `write()`; a new test
    of that story which calls either is the case that rebases. The base of statements 13 to
    17 is the tree after that story's merge when it merges first.
  - No shared change in `owner/mutation/execution.rs`, `owner/approval_issuance.rs`,
    `clock.rs`, `contracts/service/clock.md`, `docs/local-clock.md` or `local_cli.rs`.
- story:connections-list-answers-through-the-owner (branch `story/connections-through-owner`):
  `CHANGELOG.md` (it creates `## Unreleased` / `### Changed` / `### Fixed`; same rule).
- story:a-clock-refusal-of-issue-and-invoke-names-its-sources (draft): it edits `clock.rs`,
  `Error::clock` (`owner.rs:261-272`) and with it the five call-site expressions of change 3,
  and `CHANGELOG.md`. It has a `depends_on` edge to this story and is built after it, on
  `acquire_within`.
- story:the-local-gate-lints-the-windows-target: no shared file; statement 27 is its lint.
