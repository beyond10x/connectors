---
format: aep.planning-md/3
id: review-result:er-0330-pin-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: needs-revision

1. Outcome and acceptance 4: the baseline is "0.75.0", but the tree is 0.74.2 (`CHANGELOG.md:3`, tag `v0.74.2`; acceptance 3 uses the same tag). No 0.75.0 tag exists. The story says it starts from `main` after the 0.75.0 `inputs.json` regeneration, which is not what HEAD b9b56f7bd19 holds. Accept: "appends at most a third of the blob bytes it appends on connectors <the base commit's version>", with the base commit named.

2. Acceptance 4: `blob_bytes_per_invoke` does not exist in the test (`store_cost_tests.rs:361` prints `appended_per_invoke` only), and "the commit before the pin" has no such line.
   - The line has to be added in a commit that precedes the pin commit. The story must say that.
   - Another draft, `story:read-cost-test-reports-bytes-per-read`, edits the same print and is listed under "Not worked beside", so the line needs one owner.
   - The divisor is unnamed. The existing `appended / invokes` at `store_cost_tests.rs:362` counts failed invokes in the divisor, so the ratio changes with the choice.
   - Accept: "`read_invoke_cost_by_store_size` prints `blob_bytes_per_invoke` = growth of `sum(length(bytes))` over `connectors_er_blobs` / `measured` invokes, on the `events=… invokes=…` line. The number at base and at the pin, both taken with `CONNECTORS_STORE_COST_EVENTS=<n>` and `CONNECTORS_STORE_COST_INVOKES=<m>` on a store each build grew itself, is in the PR and on #267, and the second is at most a third of the first."

3. Acceptance 3, "runs one read invoke that answers": the test has no invoke. `measured_invoke` is four registry calls (`store_cost_tests.rs:20-76`: `admit_read`, `capture_read`, `dispatch_read`, `release_read`). Its helpers `fixture`, `prepared`, `publish_fixture` and `binding()` are `pub(super)` in the lib (`registry/tests.rs:858,963`), unreachable from `tests/`. The previous-pin test's own `binding(instance)` (`metadata_store_previous_pin.rs:35`) takes an argument, unlike the one `measured_invoke` uses. Accept: "…runs the four calls of `measured_invoke` with the `Binding` the cost test published, and each returns `Ok`".

4. Acceptance 3, fixture layout: `save` writes `reference` (singular, `store_cost_tests.rs:165`), while the existing restore reads `references` as `instance reference` lines (`metadata_store_previous_pin.rs:59,138-148`). The story does not say which layout the fixture has or how the new test reads it. Accept: name the file layout and the loader.

5. Acceptance 3, "at most 1.5 MB" and "every event … still recorded": no event count is named, so neither can be checked. The 0.26.0 fixture holds 15 events and is 1,033,938 bytes, so the 1.5 MB cap is not derived from any size. The default smallest size is 50 events (`store_cost_tests.rs:270-272`). Accept: the README names the `CONNECTORS_STORE_COST_EVENTS` value and the event count N, and the test asserts `event_count(restored) >= N` using `event_count` (`metadata_store_previous_pin.rs:170`) after the read.

6. Acceptance 5:
   - "What connectors 0.74.2 answers" names no command, no way of getting a 0.74.2 binary (build `v0.74.2` or the release artifact), and no store sequence.
   - "the release" has no version.
   - The CHANGELOG sentence "a store is not opened by an older release" is fixed before the run. If 0.74.2 opens the store, or reads it silently, the sentence is false.
   - The "Decided after acceptance 5" trigger ("does not tell an operator the store is newer") is a judgement, not an observation.
   - Accept: "CHANGELOG under `<version>` quotes code, stage and exit code from running `<command>` of the 0.74.2 binary on the store; the not-opened sentence appears only when the exit code is non-zero. `LEVEL` goes to 10 unless the code is `metadata_schema_newer`."

7. Acceptance 6: it names no store size and no build. Peak RSS depends on both, because `peak_rss_mb` only rises (`store_cost_tests.rs:105-110`), and it is reported per size. It names no place for the report and no `slot --timeout` for a 100-invoke run (the pool kills a job at 10 minutes). Accept: "`peak_rss_mb` from `CONNECTORS_STORE_COST_EVENTS=<n> CONNECTORS_STORE_COST_INVOKES=100`, a release build, `slot --timeout <t>`, at base and at the pin, is in the PR and on #267."

8. Acceptance 2: `--check` of the emissions takes 8 to 27 minutes per part (`docs/development.md:69-74`) and is a `slow` lane, not the gate. As written it exceeds the 10-minute pool limit unless it names `slot --timeout 30m`. The gate-side observable is missing. Accept: add "`the_committed_metadata_emissions_are_current` passes", and give each `--check` its `--part` and `slot --timeout 30m`.

9. Acceptance 7: "pass unchanged or with a change named" is not checkable for two reasons.
   - `store_cost_tests.rs` is changed by acceptance 4, so it cannot pass "unchanged".
   - Its timing tests are `#[ignore]` (`store_cost_tests.rs:265`), and `cold_owner_start.rs:317` is the same. Passing does not apply to them.
   - It names no command and no budget.
   - Accept: "`slot cargo nextest run -p connectors-host -p connectors` over those seven files exits 0 within 120 s, no test over 60 s; `store_cost_tests.rs` differs only by acceptance 4; ignored tests are not run."

Read: 1 artifact, `story:entity-runtime-0330-records-name-definition-by-digest`, whole body via `aep plan artifact show`. I also read the sibling `story:read-cost-test-reports-bytes-per-read` and checked these tree lines: `Cargo.toml:23-27`, `Cargo.lock`, `metadata_emissions.rs:63`, `docs/development.md:69-74,107`, `er.rs:48`, `metadata.rs:1374`, the 0.26.0 fixture README and size, `store_cost_tests.rs`, `metadata_store_previous_pin.rs`, and release 0.33.0 (published 2026-11-23).
Not established: I did not run any test, and I did not check the 0.33.0 tag's Eventlog dependency (acceptance 1's `eventlog-core` 0.9.4) or the 4,373-byte figure. Acceptance 8's cited lines 107-109 are right.
Outside my lane: whether the fixture and `registry/tests.rs` collide with `story:test-binary-per-package` (parallel-safety).

```findings
- file: story:entity-runtime-0330-records-name-definition-by-digest#Outcome
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the baseline is named 0.75.0 but the tree and the fixture tag are 0.74.2"
- file: crates/connectors-host/src/local/registry/store_cost_tests.rs
  line: 362
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 4: blob_bytes_per_invoke does not exist, no commit before the pin adds it, its divisor and sizes are unnamed, and the same print belongs to story:read-cost-test-reports-bytes-per-read"
- file: crates/connectors-host/src/local/registry/tests.rs
  line: 858
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3: 'one read invoke that answers' is the four registry calls of measured_invoke with helpers unreachable from tests/"
- file: crates/connectors-host/tests/metadata_store_previous_pin.rs
  line: 59
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3: the cost test saves 'reference' but the existing restore reads 'references'; no event count N or layout is named for the size cap and the events-still-recorded check"
- file: story:entity-runtime-0330-records-name-definition-by-digest#Acceptance-5
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 5 names no command, no 0.74.2 binary source and no release version, fixes the not-opened sentence before the run, and the LEVEL trigger is a judgement"
- file: crates/connectors-host/src/local/registry/store_cost_tests.rs
  line: 105
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 names no store size, build, slot timeout or place for the peak_rss_mb report"
- file: docs/development.md
  line: 69
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2: the --check runs take 8-27 minutes per part and name no slot timeout or part; the fast gate observable the_committed_metadata_emissions_are_current is missing"
- file: crates/connectors-host/src/local/registry/store_cost_tests.rs
  line: 265
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7: 'unchanged' contradicts acceptance 4 for store_cost_tests.rs, ignored tests do not pass or fail, and no command or budget is named"
```
