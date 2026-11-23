---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-revision

The tests and helpers named in the story exist at the cited lines. The file:line citations I checked (acceptance, body and `registry_uses` writers) land where the story says. The gaps are in how several lines would be checked, not in the facts.

I created two sqlite sidecar files in the 0.32.0 fixture directory while reading it. I removed both. The fixture database digest still matches its README (`3489e905…`) and `git status` is clean there.

Findings (story file is `.engineering/planning/story/settled-read-uses-leave-the-store.md`):

1. **Acceptance 1, 2, 3, 6, 7, 9 and 10: the table has no named read path.** Lines 152, 154, 165, 167, 171 and 173 count "open uses" or "ReadUse subjects" in "the table". No accessor is named, and the SQL `registry_uses` can no longer answer.
   - Write: name one `#[cfg(test)]` function, for example `open_read_uses(root) -> usize`, that tests 3, 6, 9 and 10 call.
   - Name one helper that counts recorded `ReadUse` entries by state. Model it on `recorded_clock_floor_events` (`registry/tests.rs:1853`), which reads the blob wrappers. Tests 2 and 7 call it: "no ReadUse subject" and "recorded Expired once" (the fixture holds exactly one use, per its README).
   - `blob_bytes` (`store_cost_tests.rs:259`) is private to that file. Test 1 must live in `store_cost_tests.rs` or the story must make `blob_bytes` `pub(super)`.

2. **Acceptance 1: "inside one clock interval" is undefined, and the base transition is not stated.** The interval is 60,000 ms from the recorded floor (`registry/tests.rs:1948-1957`).
   - Write: "100 `measured_invoke` calls at `now` values within 60,000 ms of the recorded registry floor append 0 events and 0 blob bytes. The same test fails on the base with 300 events."
   - Evidence for the 300: `README.md` of the 0.32.0 fixture says one read invoke appends 3 events.
   - The test runs the registry sequence only. Say that the owner path is not measured here.

3. **Acceptance 4: "the replay refusal (`registry/tests.rs:769-783`)" names lines, not a test.** The enclosing test is `lost_publication_acknowledgement_is_resolved_without_repeating_capture` (`tests.rs:738`). Write that name.

4. **Acceptance 5: the answer changes from the base and the line does not say so.**
   - On the base, a CLI revoke sets `released=1` on undispatched uses (`lifecycle.rs:469`). `dispatch_read` checks `dispatched || released` and returns `Conflict` before it checks `state == "revoked"` (`use_and_retirement.rs:299-306`).
   - With the use still in the owner's table, the same dispatch reaches the revoked check and answers `Failure::Revoked`.
   - Write: "the dispatch after a revoke from another simulated process answers `Failure::Revoked` (`revoked`, `owner.md:207`); on the base it answers `Conflict`". The report states both.

5. **Acceptance 8 has no command and is outside acceptance 15's filter.** The test is `local::metadata::tests::…` (`metadata.rs:2455`). The filter names `registry::`, `owner::governed`, `approval_issuance` and `metamorphic_tests`. Write: "`cargo nextest run -p connectors-host -E 'test(revoked_legacy_connection_retains_both_read_use_histories_without_invented_admission)'` exits 0, and `git diff 3d38f77b99f -- crates/connectors-host/src/local/metadata.rs` touches no line of `:2455-2582`."

6. **Acceptance 10 is five statements in one, and two have no pass/fail.**
   - "still fail when a use is leaked" names no proof. Write: "with the `release_read` call removed in each of the four tests, the test fails; the report names the one run".
   - `final_fence.rs:281-297` asserted `(1,0,1)`: one use, not dispatched, released. A table that removes the entry on cancel reads 0, so "cancelled" cannot be told from "never captured". Write the assertion that proves the capture happened, for example the dispatch refusal code.
   - "get their rows from a source that still records (named in the report)" has no outcome. Write the source (a direct `INSERT INTO registry_uses` inside `Metadata::update` + `persist`, as `expiry_batch_tests.rs:117` does for its DELETE), and "`expiry_batch_tests` and `metamorphic_tests` pass".
   - Split `grown_stores` into its own line: `git grep -n measured_invoke` shows no call inside `grown_stores`, and `read_invoke_cost_by_store_size` still runs. `seed_recorded_events` is `fn(root, events)` at `tests.rs:1746`.

7. **Acceptance 11: the command is incomplete and before/after are measured on different stores.**
   - The test is `#[ignore]` and documented for release builds. Write the full command: `CONNECTORS_STORE_COST_STORES=<dir> CONNECTORS_STORE_COST_EVENTS=50 CONNECTORS_STORE_COST_INVOKES=100 cargo test --release -p connectors-host --lib read_invoke_cost_by_store_size -- --ignored --nocapture`.
   - Story acceptance 10 swaps the store from read-invoke-grown to seeded. The base build with `grown_stores` unchanged would measure a different store. Write: the after-build saves the stores in `<dir>`; the base build reads the same `<dir>`, as `store_cost_tests.rs:140-143` intends ("measured against the same bytes").
   - "the `uptime` line" is undefined. Write "the output of the shell command `uptime` taken before each run".

8. **Acceptance 12: `<base>` is a placeholder and the second half has no command.** Write `3d38f77b99f`. Write the test command: `cargo test -p connectors-build --lib the_committed_metadata_emissions_are_current`. The test is in `crates/connectors-build/src/metadata_conformance.rs:966`.

9. **Acceptance 13 holds several unrelated statements, and parts of it cannot be checked.**
   - "`docs/local-er-metadata.md:118-123`, `:129-132` and `:287-292` … say where an open use lives" does not fit those lines. They hold the M7 canonicalization history, the rationale for a concrete ReadUse, and expiry batching of due uses. They say nothing about where an open use lives.
   - `owner.md:210` ("not written by any step but its read use") does not say a read use is recorded, so "no longer say" is vacuous there.
   - Write, per file and line, the sentence that replaces the old one, plus one grep that must print nothing: `git grep -n "read use recorded\|use recorded dispatched" -- contracts docs website/docs`.
   - The line guard is already a test. Name it: "`cargo test -p connectors-build --tests` passes (`the_corrected_semantics_citations_land_on_their_rules`, `cli_spec_mapping_adversary.rs:202`)". Reading `git diff` for line counts is weaker.
   - The mirror is `website/docs/reference/contracts/cli.md:2109` for `semantics.md:2094`. `owner.md` has no mirror.

10. **Acceptance 15: the filter leaves out tests that call the changed code.**
    - `keyring/custody/tests.rs:359` and `metadata/er.rs:8407` call `capture_read`.
    - The owner production paths (`owner/supervisor.rs:1484` and `owner/transport.rs:3427`) are covered only by `owner::governed` and `approval_issuance`.
    - `apps/connectors/tests/local_cli.rs:846` runs a CLI revoke.
    - Write: add `test(keyring::custody) | test(metadata::er) | test(owner::)`, and say whether `apps/connectors` `local_cli` runs.
    - The 60 s bound per test is stated. State the 120 s step bound for this filtered run, or its time on the base.

11. **Acceptance 14 names no heading.** The top `CHANGELOG.md` section is `## 0.76.0 — 2026-11-23` (`CHANGELOG.md:3`), and there is no Unreleased section. Write the heading the entry goes under (a new `## 0.77.0` section, or 0.76.0's Changed if it ships unreleased).

What I read: 1 artifact, `story:settled-read-uses-leave-the-store` (`aep plan artifact show`, full body, revision 26). I checked the tree at `3d38f77b99f` with `git grep`, `sed` and `sqlite3 -readonly`. That covered all 15 acceptance lines, the named tests and helpers, the cited lines in the tests, docs and contracts, the `registry_uses` writers, and the 0.32.0 fixture. I ran no tests.

What I could not establish:
- Whether a read invoke appends other blob bytes besides the `ReadUse` entries, so that `blob_bytes_per_invoke=0` holds. The 24,500 figure comes from an issue comment I cannot reach. The base 0.32.0 fixture's 3 events per read are consistent with it.
- Whether the table keyed by state directory and simulated process survives `exit_process`, and whether capture check plus insert is atomic. Both are design, outside my lane (plan-critic-design).
- The "one owner process per state directory" question the scoper left open. Outside my lane.

```findings
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 152
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptances 1, 2, 3, 6, 7, 9 and 10 count open uses or ReadUse subjects in 'the table' but no read path is named; name one cfg(test) accessor for the table and one helper counting recorded ReadUse entries by state, and make blob_bytes (private in store_cost_tests.rs:259) reachable from the new test"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 152
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'inside one clock interval' is undefined and the base transition is unstated; write: 100 measured_invoke calls within 60,000 ms of the recorded registry floor append 0 events and 0 blob bytes, and the same test fails on the base"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 159
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the replay refusal is named by lines registry/tests.rs:769-783, not by a test; the enclosing test is lost_publication_acknowledgement_is_resolved_without_repeating_capture (tests.rs:738)"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 163
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 5 leaves the answer unstated although it changes: on the base dispatch_read answers Conflict (released checked before revoked, use_and_retirement.rs:299-306), with the use in the owner's table it answers Failure::Revoked; write the expected code and that the base answers Conflict"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 169
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 8 names no command and its test (local::metadata::tests, metadata.rs:2455) is outside acceptance 15's filter; write the nextest command for it and that metadata.rs:2455-2582 is not in git diff against 3d38f77b99f"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 173
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 10 holds five statements; 'still fail when a use is leaked' names no proof, final_fence.rs:281-297 loses the distinction between cancelled and never captured, 'a source that still records (named in the report)' has no outcome, and grown_stores is a separate check; split and give each a pass/fail"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 180
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 11 omits --release and -- --ignored --nocapture, compares median_ms and peak_rss_mb across different stores (grown_stores changes to seeded), and leaves 'the uptime line' undefined; write the full command, share CONNECTORS_STORE_COST_STORES between base and after, and name uptime as the shell command"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 184
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 12 uses the placeholder <base> and names no command for the emissions test; write 3d38f77b99f and cargo test -p connectors-build --lib the_committed_metadata_emissions_are_current"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 188
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 13 mixes documents, a mirror and a line guard; 'say where an open use lives' does not fit local-er-metadata.md:118-123, :129-132 and :287-292, and owner.md:210 does not say a use is recorded; write the replacement sentence per line, a grep that must print nothing, and name the guard test (cli_spec_mapping_adversary.rs:202)"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 196
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 15's filter omits tests that call the changed code (keyring/custody/tests.rs:359, metadata/er.rs:8407, owner:: production-path tests, apps/connectors local_cli revoke at :846); add them to the filter and state the 120 s step bound for the run"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 194
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 14 names no CHANGELOG heading; the top section is ## 0.76.0 (CHANGELOG.md:3) and there is no Unreleased section; write the heading the Changed entry goes under"
```
