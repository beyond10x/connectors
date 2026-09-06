---
format: aep.planning-md/1
id: review-result:cli-gitlab-adversary-1-20260906
kind: review-result
status: active
title: GitLab whole-unit adversary pass 1 2026-09-06
relations:
- reviews: story:personal-gitlab-schedules-are-discoverable-and-governed
revision: 1
---
unit: story:personal-gitlab-schedules-are-discoverable-and-governed; whole-unit adversary pass 1 at source 6e6551685749d25b70baa04ccc3166278bb30492 plus appended tests
verdict: green — nothing found
cases: executed source 784→789; runtime/catalog 80→81; CLI 88→88; service 57→57; red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 46 persistent scratch files, plus compiler temporaries under ~/.cache/cw6/g/
needs-coordinator: record review and appended tests; final 12-workspace gate and integration remain coordinator-owned

```text
git --no-pager diff --stat
 .../catalog-build/tests/main/catalog_invariants.rs | 111 +++++++++++++++++++++
 .../tests/local_gitlab_schedules.rs                |  40 ++++++++
 2 files changed, 151 insertions(+)

git --no-pager diff --no-index --stat -- /dev/null crates/connector-resolve/tests/adversary_gitlab_pass1.rs
 .../tests/adversary_gitlab_pass1.rs                | 140 +++++++++++++++++++++
 1 file changed, 140 insertions(+)
```

The new resolver test file is untracked, so ordinary diffstat excludes it. Total review changes: three test files, 291 additions, no old test changes and no implementation changes. Source HEAD stayed at the stated commit. The full tracked diff is `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/tests-only.diff`; the untracked addition is `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/new-resolver-tests.diff`.

1. Cases written before the first test invocation

Each of the six cases below was written before any test ran in this review. Each was then selected alone before the suites. All six first runs exited 0 and remain green; there was no red output or compilation failure.

All commands ran in `~/.local/state/worktree/trees/b10x/connectors/wt-0374e7372138` under:

```text
env -u CARGO_TARGET_DIR TMPDIR=~/.cache/cw6/g RUSTC_WRAPPER=/usr/bin/sccache CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=3
```

`CARGO_TARGET_DIR` was unset; targets remained inside this managed tree. Exact shell wrappers, output paths and exit paths are in `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/commands.json`. The following excerpts are verbatim runner output; full logs remain at the paths shown.

`crates/catalog-build/tests/main/catalog_invariants.rs:2352` — Every coverage-gap/generated operation is actually importable; every importer-gap has a matching rejection diagnostic. All 1,847 source rows checked.

```text
cargo test --locked --offline -p catalog-build --test main adversary_gitlab_pass1_coverage_statuses_match_actual_importer_results -- --nocapture
running 1 test
coverage decisions checked for 1847 source operations: {"catalogued_generated": 4, "catalogued_legacy": 20, "coverage_gap": 1510, "importer_gap": 313}
test catalog_invariants::adversary_gitlab_pass1_coverage_statuses_match_actual_importer_results ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 77 filtered out; finished in 0.73s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-coverage.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-coverage.exit`.

`crates/catalog-build/tests/main/catalog_invariants.rs:2407` — Composed nullable/enum/integer bounds, allOf, unique heterogeneous arrays, unknown fields and literal default/enum data preserve a 4-accepted/12-refused truth table.

```text
cargo test --locked --offline -p catalog-build --test main adversary_gitlab_pass1_translation_preserves_composed_constraint_truth_tables -- --nocapture
running 1 test
independent truth table: 4 accepted and 12 refused values
test catalog_invariants::adversary_gitlab_pass1_translation_preserves_composed_constraint_truth_tables ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 77 filtered out; finished in 0.00s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-translation.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-translation.exit`.

`crates/connector-resolve/tests/adversary_gitlab_pass1.rs:21` — Frozen schema 2 still accepts a legacy document; schema 3 and the resolver reject schema-version and profile mutation edges.

```text
cargo test --locked --offline -p connector-resolve --test adversary_gitlab_pass1 adversary_gitlab_pass1_schema_versions_and_profiles_fail_closed -- --nocapture
running 1 test
test adversary_gitlab_pass1_schema_versions_and_profiles_fail_closed ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.05s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-versions.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-versions.exit`.

`crates/connector-resolve/tests/adversary_gitlab_pass1.rs:84` — Real embedded schedule requests percent-encode delimiters, Unicode, braces and percent signs once; integer values beyond binary64 precision and u64::MAX survive unchanged.

```text
cargo test --locked --offline -p connector-resolve --test adversary_gitlab_pass1 adversary_gitlab_pass1_path_and_integer_boundaries_preserve_caller_values -- --nocapture
running 1 test
test adversary_gitlab_pass1_path_and_integer_boundaries_preserve_caller_values ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.58s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-paths.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-paths.exit`.

`crates/connector-resolve/tests/adversary_gitlab_pass1.rs:116` — Template-shaped caller strings, nested objects and heterogeneous arrays remain literal JSON; omitted update bodies remain distinct from {}.

```text
cargo test --locked --offline -p connector-resolve --test adversary_gitlab_pass1 adversary_gitlab_pass1_body_literals_do_not_become_template_instructions -- --nocapture
running 1 test
test adversary_gitlab_pass1_body_literals_do_not_become_template_instructions ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.52s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-body.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-body.exit`.

`crates/connectors-runtime/tests/local_gitlab_schedules.rs:558` — Through the local socket, a selected read-only placement refuses all three mutations despite another writer, absent credentials, forged approval refs and invalid input; custody readiness/get and egress remain at zero.

```text
cargo test --locked --offline --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime --test local_gitlab_schedules adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement -- --nocapture
running 1 test
test adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 1.09s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-admission.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-admission.exit`.

2. Complete affected suites and gates

Baselines are the implementor's reported final lanes, not a pre-emptive review suite run: source 784 in `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/source-fidelity-report.md:354`; runtime/catalog 80 and CLI 88 in `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/resumed-stage2-report.md:224`. Service's 57-case baseline is the unchanged unit target in `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/fidelity-root-workspace-green.log:1239`. Review invocations add `--offline` to the locked package selections. No base tree was assigned or executed, and no pre-existing origin is claimed. These package lanes are reported separately without summing overlapping workspaces.

Source target counts:

| Target | Before | After | Ignored |
|---|---:|---:|---:|
| catalog unit | 9 | 9 | 0 |
| catalog main | 13 | 13 | 0 |
| catalog-build unit | 63 | 63 | 0 |
| catalog-build main | 76 | 78 | 0 |
| catalog-cli unit | 6 | 6 | 0 |
| catalog-cli offline_binary | 1 | 1 | 0 |
| catalog-cli vendor_gitlab example | 7 | 7 | 0 |
| catalog-reader unit | 2 | 2 | 0 |
| catalog-reader main | 13 | 13 | 1 |
| connector-resolve unit | 54 | 54 | 0 |
| connector-resolve adversary_gitlab_pass1 | 0 | 3 | 0 |
| connector-spec unit | 28 | 28 | 0 |
| connector-spec main | 510 | 510 | 0 |
| catalog doctest | 1 | 1 | 0 |
| connector-resolve doctest | 1 | 1 | 0 |
| Other source doctest targets | 0 | 0 | 0 |
| Source total | 784 | 789 | 1 |

Runtime/catalog counts: runtime unit 32→32, local_catalog_writes 8→8, local_gitlab_schedules 5→6, integration-catalog unit 35→35; both doctest targets 0. CLI target counts remain 5, 0, 1, 6, 3, 6, 3, 4, 34, 10, 6, 5, 1, 4, 0 in runner order (88 executed, no ignored). Its four search_bounds cases include all five CLI search ranges before configuration/transport. Service remains 57 unit cases and 0 doctests.

```text
cargo test -p connector-spec -p catalog-build -p catalog-reader -p catalog -p connector-resolve -p catalog-cli --locked --offline
    Finished `test` profile [unoptimized] target(s) in 10.64s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.65s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.77s
test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.99s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 13 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 510 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.88s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.18s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-source.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-source.exit`.

```text
cargo test --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime -p integration-catalog --locked --offline
    Finished `test` profile [unoptimized] target(s) in 0.61s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.44s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.27s
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-runtime.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-runtime.exit`.

```text
cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --offline --no-fail-fast
    Finished `test` profile [unoptimized] target(s) in 0.65s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.99s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-cli.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-cli.exit`.

```text
cargo test -p service --locked --offline
    Finished `test` profile [unoptimized] target(s) in 17.62s
test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-service.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-service.exit`.

```text
cargo clippy -p connector-spec -p catalog-build -p catalog-reader -p catalog -p connector-resolve -p catalog-cli -p service --locked --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized] target(s) in 1.03s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-source.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-source.exit`.

```text
cargo clippy --manifest-path crates/connectors-runtime/Cargo.toml -p connectors-runtime -p integration-catalog --all-targets --locked --offline -- -D warnings
    Finished `dev` profile [unoptimized] target(s) in 2.88s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-runtime.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-runtime.exit`.

```text
cargo clippy --manifest-path crates/connectors-cli/Cargo.toml --all-targets --locked --offline -- -D warnings
    Finished `dev` profile [unoptimized] target(s) in 1.65s
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-cli.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-cli.exit`.

```text
cargo fmt --all --check
(no output)
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-root.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-root.exit`.

```text
cargo fmt --manifest-path crates/connectors-runtime/Cargo.toml --all -- --check
(no output)
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-runtime.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-runtime.exit`.

```text
cargo fmt --manifest-path crates/connectors-cli/Cargo.toml --all -- --check
(no output)
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-cli.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-cli.exit`.

```text
cargo run -p catalog-cli --locked --offline --example vendor_gitlab -- --input ~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/gitlab-openapi-immutable.raw.yaml --root . --check
    Finished `dev` profile [unoptimized] target(s) in 0.24s
1847 source operations accounted for; 4390 example values removed; source SHA-256 435fa94969a44a8baf0e8d8cb4d6cf82186a19adc3ad449e378f54cf80d0b7f2
exit 0
```

Full output: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/vendor-check.log`; exit: `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/vendor-check.exit`.

The vendor check re-read the previously downloaded immutable raw source and reproduced the committed source, coverage and provenance without writes. The source suite also ran whole-catalog fixed-point, schema, pack/document, lock and architecture checks. No whole-12-workspace result is claimed. The inherited catalog-reader benchmark remains ignored; the local runtime's credential and provider ports are fixtures, and no live provider or custody service was contacted.

3. Findings

None. There is no finding row requiring a measured/reachable distinction; the YAML list is empty.

4. Attacks that did not break

- The source migration retained its declared constraint semantics under the composed-schema truth table and literal nested input attacks.
- All 1,847 coverage classifications matched importer availability; the immutable vendor regeneration remained deterministic.
- New schema/profile edges failed closed, while the schema-2 file stayed byte-identical: Git blob `d5a21977ce8d5bc4620eca3adfe19f258ac5808c` at both base `76f3fef9ce53a92d54d5e1c8147c5943315d423f` and source HEAD.
- Passive discovery and selected-placement mutation refusal held with missing credentials and forged approval references; the actual socket route performed no custody or egress operation.
- Literal request bodies, optional omission, URL separators and extreme integers survived the current embedded schedules; the complete affected lanes retained every pre-existing executed case.

5. Outside-write inventory

Exact persistent files written during this GitLab pass (the coordinator's pre-existing brief and the implementor reports were read only):

```text
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-admission.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-admission.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-body.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-body.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-coverage.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-coverage.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-paths.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-paths.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-translation.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-translation.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-versions.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/case-versions.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-cli.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-cli.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-runtime.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-runtime.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-source.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/clippy-source.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/commands.json
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/final-disk.txt
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/final-status.txt
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-cli.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-cli.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-root.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-root.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-runtime.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/fmt-runtime.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/new-resolver-tests.diff
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/raw-report.md
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/report.md
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/resumed-stage2-report.md.outline
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/source-fidelity-report.md.outline
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/source-only.diff
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-cli.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-cli.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-runtime.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-runtime.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-service.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-service.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-source.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/suite-source.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/tests-only.diff
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/tests-only.diffstat
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/vendor-check.exit
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/vendor-check.log
~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed/adversary-1/whole-unit.diff
```

Compiler temporary root: `~/.cache/cw6/g/`. Build targets used were `~/.local/state/worktree/trees/b10x/connectors/wt-0374e7372138/target/`, `~/.local/state/worktree/trees/b10x/connectors/wt-0374e7372138/crates/connectors-runtime/target/`, and `~/.local/state/worktree/trees/b10x/connectors/wt-0374e7372138/crates/connectors-cli/target/`, all inside the assigned tree. The prescribed existing sccache wrapper was reused. No cleanup or worktree/Git lifecycle mutation occurred. Available disk was 31 GB before first compilation and 36 GB at the final snapshot, above the 20 GB floor.

This report covers the GitLab review only. The earlier rate-limit scratch corrections and stage-2 plan were already handed to the coordinator separately; that repository remained held.

```findings
[]
```
