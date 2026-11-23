---
format: aep.planning-md/3
id: story:invoke-admission-queues-under-load
kind: story
status: active
title: 'A read on invoke names its deadline: --deadline-seconds, 1 to 120, default 30'
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: adapters/catalog/tests/local_runtime/invoke_definition.rs
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: cited
  path: apps/connectors/src/help.rs
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/invoke.rs
- confidence: inferred
  path: apps/connectors/src/local/operations.rs
- confidence: cited
  path: apps/connectors/src/local/session.rs
- confidence: inferred
  path: apps/connectors/src/walk.rs
- confidence: cited
  path: apps/connectors/tests/invoke_short.rs
- confidence: cited
  path: apps/connectors/tests/invoke_walks.rs
- confidence: cited
  path: apps/connectors/tests/owner_slow_start.rs
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/generated/metadata-emissions
- confidence: cited
  path: crates/connectors-build/generated/metadata-emissions/inputs.json
- confidence: cited
  path: crates/connectors-core/generated/operation-types
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/contracts/cli.md
- confidence: cited
  path: website/docs/reference/contracts/model
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-cli.md
revision: 38
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T04:15:21Z", actor: "human:timo", revision: 31, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "proposed", to: "active", at: "2026-11-23T04:15:23Z", actor: "human:timo", revision: 32, decided_on: {"recorded":{"review_outcome":8}}}
---
## Outcome

A caller of `connectors invoke` that passes `--deadline-seconds <n>` (1 to 120) lets a read wait
up to that long for its turn in the owner's queue. A caller that does not pass it is refused at
30 s as before: 30 s stays the default, as `contracts/cli/v1alpha1/semantics.md:1493` states
("30-second default/120-second maximum business deadline").

## Observed

Consumer: dev-operator (session bf-runbook), run `refresh-read-20261123-032902`, connectors
0.65.2, host load 70-80: `issues.search` answered `metadata_unavailable` at `observation`, then
`timeout` at `admission` twice; 4 parallel `issue.comments` calls refused 14 of 16 with `timeout`
at `admission`. Called as `connectors --output json invoke jira/<instance> <op> --input-json ...`.
The main store was 650 MB (issue #267), which every CLI call replays.

Measured 2026-11-23 on connectors 0.74.0, one catalog instance, host load 126 to 154
(https://github.com/beyond10x/connectors/issues/267#issuecomment-7091114772):

- 16 parallel reads on a 12 MB store: 5 answered in 22 to 46 s, 11 were refused `timeout` at
  `admission` after about 34 s. 4 parallel reads: all answered in 7 to 11 s.
- One read took 2.5 s at a 3 MB store and 9.9 s at 14 MB.

Cause separation: the one-read times at 3 MB and 14 MB show the store size (issue #267) and the
parallel runs show the fixed deadline; a further fresh-store run is not needed. This story is
the deadline. The store is Timo's pick D3 of 2026-11-23 (record format in Entity Runtime, then
compaction).

## What the code does today

Read by the scoper and the design critic on 2026-11-23 at 138b6cf7540:

- One worker thread per instance takes reads one at a time
  (`crates/connectors-host/src/local/owner/supervisor.rs:698-725`, `:1080`); the queue is a
  first-in first-out channel of 16 (`:703`), and a 17th waiting read is refused `capacity`.
- The 30 s is set only in the CLI, on two clocks that share no state: `prepare` starts the owner
  with `Instant::now() + READ_DEADLINE` (`apps/connectors/src/local/invoke.rs:52`, `:1029`), and
  the generated runner later sets `session.deadline = now_ms() + 30_000`
  (`apps/connectors/src/local/session.rs:600`). A `--walk` re-enters per page.
- The owner, the registry and the adapter channel admit an invoke deadline up to 120 s
  (`owner/transport.rs:2700`, `registry.rs:56`, `runtime/process.rs:1100-1102`).
- On the executable-changed path the capture lease is capped at 30 s
  (`crates/connectors-host/src/local/owner.rs:1135-1141`), while validation and its retries run
  to the read deadline (`supervisor.rs:1516-1546`). Today both are 30 s.

## Design

- `invoke` gains `--deadline-seconds <n>`, an integer 1 to 120.
- One value: `prepare`, and each page of a walk, takes the cutoff once as absolute epoch
  milliseconds. The owner's start derives its `Instant` from it and `session.rs:600` reads it
  instead of `now_ms() + 30_000`. Without the flag the value is 30 s from that instant. Needed
  because the flag would otherwise allow two full windows, start then read. `prepare` returns
  rewritten arguments and the runner builds its own `Session` (`local.rs:68-72`), so the value
  needs a carrier across that boundary; the implementer picks it and names it in the report.
- The consumer's change: dev-operator passes `--deadline-seconds 120` on its reads.
- With `--walk` the flag is each page's deadline, as the 30 s is today: a walk of many pages
  could not fit one 120 s cutoff.
- Reads only, decided from the arguments: beside `--idempotency-key`, `--approval-file` or
  `--describe` the flag is refused `invalid_input` at `arguments` with `field =
  deadline_seconds`, before an owner is started or anything is sent. A value outside 1 to 120 or
  not an integer is refused the same way. With `--definition` the flag sets the read's deadline
  as for a named instance.
- Executable-changed path: validation and its retries end at the earlier of the read deadline
  and the capture lease's expiry, so a deadline above 30 s cannot produce a validation that
  succeeds and is then refused `Expired`.
- At the cutoff a read still waiting is refused `timeout` at `admission`, as today. With a large
  store 16 serialized reads can need more than 120 s (9.9 s each at 14 MB), so some are refused
  even with the flag at 120 until issue #267 and entity-runtime#94 shrink the store; the caller
  splits the burst (4 at a time answered in 7 to 11 s).
- `contracts/cli/v1alpha1/semantics.md` gains the flag's rule; the line citations of that file
  in `ess/domains/cli.yaml` (`:1946`, `:2106`) are renumbered to land on their rules.

Unchanged, and the existing tests for each pass unchanged:

- The queue of 16 and its `capacity` refusal. A `capacity` refusal in a burst above 17 is the
  queue's answer, not a failure of this story.
- The owner exchanges that run before the deadline exists keep their fixed 30 s and 40 s
  (`owner/transport.rs:1037`, `:1052`, `:1068`): a deadline below 30 s is not honoured by them.
- The 30 s clamp of a recollection (`supervisor.rs:1584`) and the store lock's 30 s
  (`metadata.rs:51`).
- `lapses_early` (`supervisor.rs:1498-1503`): with a longer deadline more reads run an identity
  probe first; the measurement below shows the cost.
- A read use never outlasts its credential evidence.
- The refusal's members: no new member (see Out of scope).

## Acceptance

Tests assert on events, not on elapsed time against a bound a loaded runner crosses.

1. `apps/connectors/tests/owner_slow_start.rs`, provider delay 1.5 s (`invoke_ms`), 3 parallel
   reads on one instance, `--deadline-seconds 2`: at least one exits with `timeout` at
   `admission`.
2. Same run: a read refused at `admission` was never sent. The fixture provider records a read
   before its delay, so a read refused at `dispatch` has a line: the lines of
   `/work/invocations` number at least the reads that answered and at most those plus the
   reads refused at `dispatch`.
3. Same setup with `--deadline-seconds 20`: all 3 answer. One read with `--definition` and
   `--deadline-seconds 20` answers, in the ignored journey of
   `adapters/catalog/tests/local_runtime/invoke_definition.rs`, which CI runs in its job
   "ignored CLI journeys".
4. One cutoff: with `--deadline-seconds 2`, an owner start delayed 1.5 s
   (`CONNECTORS_TEST_OWNER_START_DELAY_MS`) and a read of 1.5 s, the call is refused `timeout`
   (the test names the stage it observes). With two separate windows it would answer.
5. With `--deadline-seconds 2` and an owner start delayed 600 s (it never greets within the
   test), the call exits non-zero with `timeout`. A hang is the runner's 60 s kill, a failure.
6. `apps/connectors/tests/invoke_short.rs`: `--deadline-seconds 0`, `121` and `x` are each
   refused `invalid_input`, `field = deadline_seconds`, at `arguments`; so is the flag beside
   `--idempotency-key`, beside `--approval-file` and beside `--describe`. In each case no owner
   is started and `/work/invocations` is empty. `1` and `120` are accepted.
7. A unit test in `apps/connectors/src/local/invoke.rs` asserts that the cutoff built from
   arguments without the flag is 30,000 ms after its instant. (Line 4 checks that the owner
   start and the read share it.)
8. A `--walk` with `--deadline-seconds 8` over 3 pages of 3 s each answers every page
   (`apps/connectors/tests/invoke_walks.rs`; its fixture gets a per-page delay). Page 1 follows
   the owner start of `prepare`, as today. (With pages of 1.5 s and a deadline of 2 the first
   page timed out at `dispatch` in 4 of 4 runs at host load 65 to 120.)
9. Executable-changed path: the lease length is a parameter of `follow_executable_within` (30 s
   in production; `follow_executable` keeps its signature for the write path). A test in the
   `connectors-host` lib tests uses a 3 s lease, a 10 s read deadline and a validation that
   pauses 3.5 s: the read is refused once with `timeout` (the registry's `Expired`), and no
   validation is sent after the lease expiry. (A 1 s lease made the test depend on two sqlite
   commits finishing within 300 ms.)
10. `connectors invoke --help` and the bash completion contain `--deadline-seconds`, and the help
    names `1` and `120`. `semantics.md` contains `--deadline-seconds`. `owner.md` no longer
    contains "the read's own 30 seconds". The checks `AGENTS.md` lists after an edit to
    `ess/domains` pass: `connectors-build metadata-entities --check`, `ess-boundary`, `cli
    --check`, `mcp-bindings --check`, `service-metadata --check`, `connectors-docs generate
    --check` and `cargo test -p connectors-build --tests`.
11. Measurement, outside the test step, on the instance of the 2026-11-23 comment, with the
    released binary that carries the flag (the instance's own credential setup, no second one):
    16 parallel reads with `--deadline-seconds 120`. Recorded on issue #267 and as evidence on this story:
    version, store size, host load, and per read the outcome and time. Pass: no read is refused
    `timeout` before 120 s of its own wall time; each read answers, is refused `capacity`, or is
    refused `timeout` at `admission` at 120 s; and more than 5 of the 16 answer (baseline: 5 of
    16 on 0.74.0). Pass is not 16 of 16 on a large store.

Each new test's time is reported; budget 60 s per test, 120 s for the test step.

## Shares files with

Not to be worked beside this story (same source, same whole regenerated files, or the
`semantics.md` line numbers): story:invoke-exits-promptly-on-sigterm,
story:missing-state-database-says-initialize,
story:setup-check-names-missing-provider-with-instances,
story:definitions-install-requires-a-valid-configuration, story:tool-contract-for-any-instance,
story:definition-files-in-user-config, story:standing-write-approval-for-unattended-agents,
story:cli-option-value-hint, story:docs-generated-reference-readable,
story:guarded-write-credential-refusal-says-repair, story:openrouter-inference-key-profile,
story:mcp-cli-journey-discovery-contract, story:mcp-outbound-stdio-runtime,
story:admission-checks-token-scopes, story:timeout-refusal-says-how-long-it-waited, and the
Entity Runtime 0.33.0 pin (regenerate
`crates/connectors-build/generated/metadata-emissions/inputs.json` after whichever lands second).

After this story lands, story:invoke-exits-promptly-on-sigterm runs its blocked-admission test
with `--deadline-seconds 120` too.

## Out of scope

- The store size and per-read cost: issue #267 and beyond10x/entity-runtime#94 (D3 of
  2026-11-23).
- `waited_ms` on a `timeout` refusal: story:timeout-refusal-says-how-long-it-waited. It changes
  the shared `connectors.cli.Failure`; the flag does not depend on it.
- A retry delay on a `timeout` refusal, which the first version of this story offered as an
  alternative: not delivered. The caller names its own deadline instead.
- `metadata_unavailable` at `observation` under load: cause not shown; assumed to be the 650 MB
  store (issue #267). Re-checked when the consumer reads through a small store.
- The write path (`crates/connectors-host/src/local/owner/mutation/execution.rs`) calls
  `follow_executable` and validates up to its own deadline; the implementor reported that this
  can pass the 30 s lease. Not checked and not changed here.
- More than one worker per instance, and a longer queue than 16: no artifact carries it. It is
  raised only if bursts are still refused after entity-runtime#94.
- A deadline flag for writes, `connections` commands and the MCP projection: not requested.
- The hand-written invoke flag lists in
  `adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json`, its `.md` and
  `docs/local-mcp-cli.md:147`: story:mcp-cli-journey-discovery-contract.
