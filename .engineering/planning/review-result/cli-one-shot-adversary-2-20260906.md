---
format: aep.planning-md/1
id: review-result:cli-one-shot-adversary-2-20260906
kind: review-result
status: active
title: CLI one-shot final adversary pass 2026-09-06
relations:
- reviews: story:one-shot-operations-without-a-daemon
revision: 1
---
unit: story:one-shot-operations-without-a-daemon at 17763bbc1552fcda2b9a323bfdd04f5fcef9bd22 plus final tests-only diff
verdict: green (final cases pass; no confirmed product blocker)
cases: executed 626→631 affected, red 0 final; 2 initial fixture reds retained
origin: introduced 0 / pre-existing 0 / undecided 2
wrote-outside-worktree: 4 namespaces; exact retained paths in outside-write-inventory.md
needs-coordinator: yes — record this second/final pass and retain tests/history; no production correction requested
```text
$ git --no-pager diff --stat
 crates/connectors-cli/tests/one_shot_operations.rs | 173 +++++++++++++++++++++
 .../connectors-runtime/tests/one_shot_runtime.rs   | 101 ++++++++++++
 2 files changed, 274 insertions(+)
```

Only assigned test files changed. Both committed files remain exact byte prefixes, preserving all eight first-pass cases; see [count/preservation proof](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/counts-and-preservation.json) and [final patch](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final-tests-only.patch). No production, model, AEP, Git, lifecycle or operator-state mutation occurred. This is the second/final attack, not approval or an independent-verification claim.

Read the complete unit diff against3df1cd2d, saved charter/brief, acceptance, repository instructions, prior reports/correction patches and relevant callers. Inspected CLI routing; composition/lock ordering; operation/Connection validation; registry ownership/leases/claims/shutdown; Catalog, Platform, Slack, Kubernetes and Monitoring lifetime classification; supervisor startup, candidate generation and provider audit. Worktree and ESS skills were applied; Connectors was consulted, and no integration client was needed.

Public redaction declaration: the public report mechanically replaces only the absolute local home directory prefix with `~`. All other bytes—including commands, statuses, findings and fixture mistakes—are identical. Full logs/patches are retained beside this report.

1. Cases and deciding executions

All five cases were written before the first test run: [initial patch](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/cases-before-execution.patch), timestamp09:24:15.320093015+0200; first deciding log completed09:24:41.006495209+0200. No suite ran first. The runtime fixture's required execution_ref: None initializer was added by static inspection before its first compilation; no compile red occurred. [Exact fixture corrections](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fixture-corrections.patch) and [final pre-suite patch](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/cases-final-before-suite.patch) preserve the complete history.

| Case | Assertion and actual reach | Final |
|---|---|---|
| CLI:776 | Real describe/invoke processes: malformed HTTP200 provider output is sent once, audited attempted/indeterminate, and ownership permits the next process | green |
| CLI:813 | Two real invoke processes transport the provider-owned next cursor from page1 into page2 once each | green |
| CLI:857 | Real CLI processes discover stable Kubernetes candidates with a nonexistent auth helper; no activated Connection is published; activation requires serve | green |
| Runtime:525 | Invalid owner digest, oversized query and oversized invocation refuse before missing config/state access | green |
| Runtime:593 | Public LocalOneShot with a malformed stub reply reduces it to a valid error, calls shutdown once and releases ownership | green |

CLI line references name crates/connectors-cli/tests/one_shot_operations.rs; Runtime references name crates/connectors-runtime/tests/one_shot_runtime.rs. The malformed-reply stub tests the public transport seam; no production caller producing that malformed result is claimed. Kubernetes measures passive success despite the missing helper and named activation refusal; source inspection shows open only reads kubeconfig. It is not an OS child-process trace.

Every Cargo command below used cwd ~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5, CARGO_TARGET_DIR unset, TMPDIR=~/.cache/cw6/o, RUSTC_WRAPPER=/usr/bin/sccache, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_BUILD_JOBS=3.

Exact individual deciding commands, each before the complete suites:
```text
cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked final_adversary_invalid_provider_output_is_not_resent_and_releases_the_state_root -- --exact --nocapture
cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked final_adversary_provider_cursor_survives_two_distinct_one_shot_processes -- --exact --nocapture
cargo test --manifest-path crates/connectors-cli/Cargo.toml --test one_shot_operations --locked final_adversary_kubernetes_candidates_never_publish_a_dead_connection_or_run_auth_exec -- --exact --nocapture
cargo test --manifest-path crates/connectors-runtime/Cargo.toml --test one_shot_runtime --locked final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state -- --exact --nocapture
cargo test --manifest-path crates/connectors-runtime/Cargo.toml --test one_shot_runtime --locked final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release -- --exact --nocapture
```

| Command above | First execution | Corrected execution, when needed | Raw output and exit files |
|---|---|---|---|
|1|101: test incorrectly read top-level outcome; no-resend assertions already passed|0: reads event.outcome|[initial output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/deciding-provider.log), [initial exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/deciding-provider.exit); [final output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_invalid_provider_output_is_not_resent_and_releases_the_state_root.log), [final exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_invalid_provider_output_is_not_resent_and_releases_the_state_root.exit)|
|2|0|—|[output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_provider_cursor_survives_two_distinct_one_shot_processes.log), [exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_provider_cursor_survives_two_distinct_one_shot_processes.exit)|
|3|101: empty target_grants rejected before discovery|0: required synthetic Prometheus Grant supplied|[initial output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_kubernetes_candidates_never_publish_a_dead_connection_or_run_auth_exec.log), [initial exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_kubernetes_candidates_never_publish_a_dead_connection_or_run_auth_exec.exit); [corrected output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/corrected-kubernetes.log), [corrected exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/corrected-kubernetes.exit)|
|4|0|—|[output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state.log), [exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state.exit)|
|5|0|—|[output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release.log), [exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release.exit)|

Neither initial red is a product defect. Existing audit_line at crates/integration-platform/src/audit.rs:222 nests the event; its existing test helper reads event.outcome at tests.rs:209. Kubernetes validation at crates/connectors-config/src/personal.rs:1050 requires nonempty target_grants. Only these new fixtures were corrected; no pre-pass case changed.

2. Complete checks

Before626 comes from the supplied brief/first-pass full lanes, not a pre-case run or base execution. After counts come from the retained runner outputs:

| Lane | Before passing | After passing | Failed | Existing ignored | Exit |
|---|---:|---:|---:|---:|---:|
|CLI|120|123|0|0|0|
|Runtime|332|334|0|2|0|
|Console|87|87|0|0|0|
|Server package|87|87|0|0|0|
|Affected total|626|631|0|2|0|
|Root workspace including server87|1141 supplied by correction report|1141|0|4|0|

Server is a root-workspace member: its --workspace command additionally ran the entire root. The four selected workspace runs total1685 passing/0failed/6existing ignored, comprising631affected plus1054additional root cases. Do not add1141 to631 without removing the duplicate server87. All15ESS fences passed in that complete root run.

For each LANE in connectors-cli, connectors-runtime, connectors-console, server, these exact commands ran after all five deciding cases:
```text
cargo test --manifest-path crates/$LANE/Cargo.toml --workspace --locked --no-fail-fast
cargo clippy --manifest-path crates/$LANE/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo fmt --manifest-path crates/$LANE/Cargo.toml --all -- --check
```

All twelve commands exited0. Complete untruncated logs and exact exits:

- connectors-cli: [tests](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-connectors-cli.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-connectors-cli.exit), [strict clippy](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-connectors-cli.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-connectors-cli.exit), [format](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-connectors-cli.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-connectors-cli.exit).
- connectors-runtime: [tests](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-connectors-runtime.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-connectors-runtime.exit), [strict clippy](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-connectors-runtime.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-connectors-runtime.exit), [format](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-connectors-runtime.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-connectors-runtime.exit).
- connectors-console: [tests](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-connectors-console.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-connectors-console.exit), [strict clippy](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-connectors-console.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-connectors-console.exit), [format](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-connectors-console.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-connectors-console.exit).
- server: [tests](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-server.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/full-server.exit), [strict clippy](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-server.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/clippy-server.exit), [format](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-server.log)/[exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/fmt-server.exit).

The explicit installed binary was inspected first; no separate ess0.18.0 filename existed:
```text
$ ~/.cargo/bin/ess --version
ess 0.18.0
$ ~/.cargo/bin/ess specify validate --path ess/system
connectors v1 — 10 file(s), valid
exit 0
```
[Version output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/ess-version.log), [validation output](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/ess-validate.log), [validation exit](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/ess-validate.exit). No model/binary changed.

3. Findings and reachability

No confirmed product defect was found. Both rows preserve invalid-probe dispositions against17763bbc's initial added fixtures; both corrected cases are green. No base worktree was assigned and no base execution was performed, so origins remain undecided.

| File:line | Verdict | Origin | Severity | Finding |
|---|---|---|---|---|
|crates/connectors-cli/tests/one_shot_operations.rs:801|INFEASIBLE|undecided|note|The initial invalid-output probe reads a top-level audit outcome although the stored row nests it at event.outcome, so its first red is a fixture-accessor error rather than an invocation failure.|
|crates/connectors-cli/tests/one_shot_operations.rs:859|INFEASIBLE|undecided|note|The initial Kubernetes probe supplies empty target_grants although configuration validation requires at least one target Grant, so its first red never reaches passive discovery.|

What was measured: row1's initial run exited101 on unwrap at:803 after the exactly-one-request assertion passed; row2 exited101 at success():108 after receiving a structured configuration refusal from fixture:859. What reaches it: only the new test's wrong JSON accessor in row1; an invalid synthetic configuration refused by existing validation in row2. Neither establishes the claimed product defect. Exact red output remains in the linked deciding logs.

The prior Monitoring/Registry citation shifts and separately identified old Kubernetes offsets are not open findings. Corrected17763bbc passed the complete root fences in this pass.

4. Coverage and limits

The five new cases passed alongside retained race/async shutdown ownership, uncertain invoke transport, unknown/ambiguous lifetime owner, hosted isolation, JSON-source, route/header confinement, revoked Grant and persistent-command refusal cases. Existing lanes also retain default paths, separate-process dispatch, read-only write refusal, browser aliases and ephemeral Slack supervisor coverage.

No live provider, hosted deployment, operator credential store, OS-wide child-process inventory, arbitrary embedding-future cancellation/panic scenario, third attack or unrelated credential unit was exercised. The report claims only the named fixture/source paths. Coordinator owns planning, commits, publication and closure.

5. Outside writes

[Exact outside-write inventory](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/outside-write-inventory.md) lists all explicit scratch files, the340timestamp-selected retained tool paths, assigned TMPDIR and compiler/Cargo caches. [Raw path list](~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/adversary-2/retained-tool-write-paths.txt) is preserved. Transient fixtures removed by their existing test/compiler owners can be identified only by namespace; no syscall trace was taken and shared-cache timestamps do not establish unique attribution.

Only per-tree build targets were used. No cleanup occurred. Lowest sampled free space23GB was reported before building; final available31,544,557,568bytes remained above20GB. Final HEAD remained17763bbc and git diff --check emitted no output.

```findings
- file: crates/connectors-cli/tests/one_shot_operations.rs
  line: 801
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: The initial invalid-output probe reads a top-level audit outcome although the stored row nests it at event.outcome, so its first red is a fixture-accessor error rather than an invocation failure.
- file: crates/connectors-cli/tests/one_shot_operations.rs
  line: 859
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: The initial Kubernetes probe supplies empty target_grants although configuration validation requires at least one target Grant, so its first red never reaches passive discovery.
```
