---
format: aep.planning-md/3
id: review-result:connections-through-owner-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: needs-revision

1. Acceptance line 1 (`connections describe` without `--connection`) cannot pass with the Design as written. `connections::select` (`apps/connectors/src/local/connections.rs:417-450`) runs for `connections-describe` (`:403`) before any handler. It is called at `apps/connectors/src/local.rs:233` and calls `Registry::held` (`:434`). `held` goes through `transaction_observation` (`crates/connectors-host/src/local/registry/lifecycle.rs:356`), which opens the store in the CLI process. That fresh open logs `open` and `snapshot`. `select` also writes `input["connection"]`, so the Design's `describe` selector "none for the held one" is never sent. Add to the Design and line 1 that for `connections-describe` the unnamed connection is resolved by the owner, not by `select`. Then reword the last Design bullet to "`connections::select` is unchanged for revalidate, revoke, launch and token; `connections describe` skips it when an owner of the same build answers".

2. Acceptance lines 1 and 3 ("the log … of the CLI process"). `cold_owner_start.rs` cannot identify the CLI process as written. Each log line is `<pid> open` or `<pid> snapshot` (`er.rs:2899-2905`). `Jail::output` returns only an `Output`, and there is no PID namespace (`fixture.rs:249` unshares user and mount only). `passes()` (`cold_owner_start.rs:104-126`) counts all pids together. Say how the CLI is told apart. The existing warm-read check asserts zero lines in total (`:215-220`), and the owner answers from its pooled handle without logging. So write "the log, drained after the owner is started by a first cold read, holds no `open` and no `snapshot` line at all after each of the five commands". Alternatively take the owner's pid from that cold read's single log line and assert every line after it carries that pid. Also say how the owner is started and drained: a cold read, then `passes()`, as `verifies_once` does.

3. Acceptance line 1 (`connections status --acquisition <id>`). The `<id>` has no source in the test file. `connected()` keeps only `connected["connection"]["summary"]["connection"]` (`:48-51`). The connect result's `acquisition` is `Optional<String>` (`ess/domains/cli.yaml:1540`), and nothing says the mounted fixture returns one. Add "the acquisition id read from the `connections connect` result in `connected()`". If the fixture returns none, add that the line gets one another way, for example a pending acquisition.

4. Acceptance line 2. "Except `observed_at_ms` and `valid_until_ms`" names fields at three depths. They are top level for list (`connections.rs:120-121`), inside `connection` for describe (`:481-482`), and `acquisition.observed_at_ms` for acquisition status (`:154-155`). The line does not say how the comparison is made, so it can pass or fail on the normalisation chosen. Name that the test removes those two keys at every depth of `result` before comparing. Name the order: the no-owner answers are taken first, after `setup owner-stop` (line 4 shows none is started), then the owner is started and the same commands are run.

5. Acceptance line 5. "As `listed_through_owner` does" conflicts with the Design. `listed_through_owner` returns `None` for a refusal too (`owner.rs:679-683`: "a refusal included"), and the Design says a refusal is an answered exchange with no fallback. The line does not say how the wrapper separates "no usable owner or failed exchange" (→ `None`) from "owner refused" (→ not `None`). Also name the fixture. `admission_tests.rs:140-183` (another-build owner, `OwnerBuildMismatch`) and `:191` (`answer_once`, a scripted owner) are the precedents in `connectors-host`. Reword to "the wrapper returns `None` for an owner of another build and for a greeting or frame failure, and returns the owner's refusal for a refusal; the host test names the two fixtures it uses".

6. Acceptance line 6. The pass rule exists (at most half), but the measurement cannot be repeated from the text. It names no harness or command. The existing `grow` (`cold_owner_start.rs:273`) writes filler descriptions with `remember`, not read invokes. The existing ignored test is run as `cargo test --release … --ignored` with `CONNECTORS_COLD_START_STORE_MB`. Name which of these is used (for example "extend the ignored test with the three commands, `CONNECTORS_COLD_START_STORE_MB=10`"). Name the statistic compared (median of three, or worst-with-owner against best-without), since "the time" of three runs is undefined. Say that the release build takes a pool job with `slot --timeout`, while the timed runs start without the `slot` prefix (`~/beyond10x/AGENTS.md`). Also say what counts as "host load named" (for example the `slot status` header and `uptime` line pasted), or drop it as a condition.

7. Acceptance line 7 holds three statements and cannot fail. "Stay true as written, or are corrected where they are not" is satisfied by editing nothing. `semantics.md:1378-1381` and `local-er-metadata.md:269-271` name only `cached` and `invoke`'s admission, so after the change they are incomplete. Replace the clause with "`semantics.md:1378-1380` and `docs/local-er-metadata.md:269-271` name the new request beside `cached`". Split the three statements: `owner.md` names the ESS type beside `connections` (the check is `git grep` of the type name in `owner.md`), the two prose sentences, and the CHANGELOG entry.

8. Acceptance line 8 names no check. "Current" is shown by the drift commands `AGENTS.md:149-153` lists: `connectors-build metadata-entities --check`, `ess-boundary`, `cli --check`, `mcp-bindings --check`, `service-metadata --check`, `connectors-docs generate --check`, and `cargo test -p connectors-build --tests`. The last runs the line-citation tests that read `cli.yaml`. Name them and require exit 0. "Each its own pool job with `slot --timeout 30m`" is a how, not an outcome, so move it to the notes. "`cli.yaml` and `binding.json` do not change" is checkable with `git diff --stat` and should say so.

9. Acceptance line 9. "Filtered to `cold_owner_start`, the new host test and the `owner` unit tests" is not a filter. Give the nextest filter expression: `binary(cold_owner_start) | test(<new host test name>) | test(owner::)`. With `-p connectors-host -p connectors` the build of all test binaries of both packages is a pool job. Add `SLOT_TIMEOUT=30m` for it, or the 10-minute limit kills it. The existing nextest `slow-timeout` (`.config/nextest.toml:5`: 30 s × 2) enforces the 60 s bound per test. The 120 s step bound has no stated measurement, so name "wall time of the run step, excluding the build".

Read: story:connections-list-answers-through-the-owner (1 of 1) in full, plus `connections.rs:1-200` and `:200-540`, `local.rs:215-260`, `lifecycle.rs:356`, `er.rs:2385-2440` and `:2880-3010`, `transport.rs:1055-1145`, `:1675-1700` and `:3118-3160`, `owner.rs:675-695`, `cold_owner_start.rs` in full, `admission_tests.rs`, `owner.md:225-262`, `semantics.md:68-100` and `:1370-1390`, `scenarios.md:60-68`, `local-er-metadata.md:265-280`, `Taskfile.yml`, `AGENTS.md:140-170` and `.config/nextest.toml`.

Not established: whether the owner's `catch_up` ever logs `snapshot` for these commands (the warm-read precedent suggests not); whether the mounted fixture's connect returns an acquisition id; whether the owner's `Registry::list` on a grown store is fast enough for the half-time rule at 10 MB. Out of my lane: the `Shares files with` section and the Design's choice of one `observe` kind with a selector.

Note of the session that recorded this review: the critic wrote the block below with the keys
`artifact`, `line` (the acceptance line), `citation` and `reason`. The keys are rewritten to the
ones the store reads; each entry keeps its words in `message`. The critic gave no severity and
no origin: `warning` and `introduced` were set by the recording session.

```findings
- file: apps/connectors/src/local/connections.rs
  line: 403
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 1: describe without --connection opens the store in the CLI through connections::select, so line 1 fails while select is unchanged (apps/connectors/src/local/connections.rs:403,434)'
- file: apps/connectors/tests/cold_owner_start.rs
  line: 104
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 1,3: the log cannot tell the CLI pid from the owner pid; say zero lines in total or the owner pid from the cold start (apps/connectors/tests/cold_owner_start.rs:104-126)'
- file: ess/domains/cli.yaml
  line: 1540
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 1: the acquisition id for status --acquisition has no source in the test (connect result acquisition is Optional) (ess/domains/cli.yaml:1540)'
- file: apps/connectors/src/local/connections.rs
  line: 120
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 2: the two time keys sit at three depths and the order of the no-owner and owner runs is not stated (apps/connectors/src/local/connections.rs:120-121,154-155,481-482)'
- file: crates/connectors-host/src/local/owner.rs
  line: 679
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 5: "as listed_through_owner does" returns None on refusal, which the Design forbids; fixtures not named (crates/connectors-host/src/local/owner.rs:679-683)'
- file: apps/connectors/tests/cold_owner_start.rs
  line: 273
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 6: no harness or command, growth is not by read invokes, statistic over three runs and load not defined, release build needs a named pool time (apps/connectors/tests/cold_owner_start.rs:273,313-316)'
- file: contracts/cli/v1alpha1/semantics.md
  line: 1378
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 7: "stay true as written, or are corrected" passes with no edit; three statements in one line (contracts/cli/v1alpha1/semantics.md:1378-1381)'
- file: AGENTS.md
  line: 149
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 8: names no drift check command; the pool-job clause is process, not outcome (AGENTS.md:149-153)'
- file: .config/nextest.toml
  line: 5
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'acceptance line 9: no filter expression, no slot timeout for the two-package test build, 120 s bound has no stated measurement (.config/nextest.toml:5)'
```
