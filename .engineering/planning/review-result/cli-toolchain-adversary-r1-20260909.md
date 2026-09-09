---
format: aep.planning-md/1
id: review-result:cli-toolchain-adversary-r1-20260909
kind: review-result
status: active
title: Exact-source ESS toolchain adversary, round 1
relations:
- reviews: story:local-cli-ess-surface
revision: 1
---
unit: story:local-cli-ess-surface exact-source toolchain ATTACK1; working tree cli-toolchain-20260909 at base 88c036562f6ae011bd5cb0ce8e4d4671146b4922
verdict: nothing found
cases: executed 9→13, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: final ESS source commit, real source build, pin replacement, manifest regeneration and required integration gates

## 1. Diff proof

Entry and current `git --no-pager diff --stat`:

```text
 crates/connectors-spec/src/toolchain.rs | 609 +++++++++++++++++++++++++++++++-
 crates/connectors-spec/src/v2.rs        |   2 +-
 crates/connectors-spec/toolchain.json   |   2 +-
 docs/gitlab-generation.md               |  67 +++-
 4 files changed, 649 insertions(+), 31 deletions(-)
```

These four non-test paths are the implementation inherited at review entry, not changes made by this adversary. Their entry SHA-256 values are retained in `.local/tmp/cli-wave/adversary-toolchain-r1-entry.sha256` and verified unchanged. The only source addition by this review is the untracked test file `crates/connectors-spec/tests/toolchain_adversary.rs`, which ordinary `git diff --stat` does not show. No implementation, existing test case, AEP artifact or commit was changed. No charter violation was introduced by this review.

Review addition shown by `git --no-pager diff --stat --no-index /dev/null crates/connectors-spec/tests/toolchain_adversary.rs` (exit 1 means files differ):

```text
 .../connectors-spec/tests/toolchain_adversary.rs   | 142 +++++++++++++++++++++
 1 file changed, 142 insertions(+)
```


## 2. Cases written, then executed alone

All four cases were written before any test command. Each ran alone before the broader suite. No case produced a product failure; all are green. There is no red assertion output to report.

An initial attempt to run case 1 failed before compilation or test selection (exit 101):

```text
error: process didn't exit successfully: `/usr/bin/sccache /home/timo/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc -vV` (exit status: 2)
--- stderr
sccache: error: path must be shorter than SUN_LEN

```

The coordinator authorized `RUSTC_WRAPPER=` for the managed-path socket-length limit. All subsequent commands used direct rustc with two jobs, incremental/debug disabled, the assigned temporary directory and this checkout's own target. This infrastructure refusal is not a finding or a red test.

`crates/connectors-spec/tests/toolchain_adversary.rs::malformed_or_wrong_identity_receipt_never_executes_matching_bytes` — Missing, wrong-repository, wrong-commit, wrong-version, wrong-format, wrong-build-argument and unknown nested-field receipts refuse before a marker-writing executable runs; a matching control executes. Current status: green.

Command (exit 0):

```sh
set -o pipefail
RUSTC_WRAPPER= TMPDIR="$PWD/.local/tmp/cli-wave" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connectors-spec --test toolchain_adversary malformed_or_wrong_identity_receipt_never_executes_matching_bytes -- --exact 2>&1 | tee .local/tmp/cli-wave/adversary-toolchain-r1-case1-direct.log
```

Verbatim output:

```text
   Compiling connectors-spec v0.1.0 (/home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-toolchain-20260909/crates/connectors-spec)
    Finished `test` profile [unoptimized] target(s) in 1.68s
     Running tests/toolchain_adversary.rs (target/debug/deps/toolchain_adversary-e2f1c17116d36dcc)

running 1 test
test malformed_or_wrong_identity_receipt_never_executes_matching_bytes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

```

`crates/connectors-spec/tests/toolchain_adversary.rs::symlink_alias_cannot_supply_a_receipt_for_an_unproven_target` — A receipt next to a symlink cannot admit its unreceipted canonical target; a receipt beside the target admits the alias and resolution returns the canonical path. Current status: green.

Command (exit 0):

```sh
set -o pipefail
RUSTC_WRAPPER= TMPDIR="$PWD/.local/tmp/cli-wave" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connectors-spec --test toolchain_adversary symlink_alias_cannot_supply_a_receipt_for_an_unproven_target -- --exact 2>&1 | tee .local/tmp/cli-wave/adversary-toolchain-r1-case2.log
```

Verbatim output:

```text
    Finished `test` profile [unoptimized] target(s) in 0.18s
     Running tests/toolchain_adversary.rs (target/debug/deps/toolchain_adversary-e2f1c17116d36dcc)

running 1 test
test symlink_alias_cannot_supply_a_receipt_for_an_unproven_target ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

```

`crates/connectors-spec/tests/toolchain_adversary.rs::explicit_wrong_source_refuses_even_when_it_reports_the_pinned_release` — An explicit exact-source mismatch refuses before executing a same-release tool, and an empty explicit selection refuses. Current status: green.

Command (exit 0):

```sh
set -o pipefail
RUSTC_WRAPPER= TMPDIR="$PWD/.local/tmp/cli-wave" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connectors-spec --test toolchain_adversary explicit_wrong_source_refuses_even_when_it_reports_the_pinned_release -- --exact 2>&1 | tee .local/tmp/cli-wave/adversary-toolchain-r1-case3.log
```

Verbatim output:

```text
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running tests/toolchain_adversary.rs (target/debug/deps/toolchain_adversary-e2f1c17116d36dcc)

running 1 test
test explicit_wrong_source_refuses_even_when_it_reports_the_pinned_release ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

```

`crates/connectors-spec/tests/toolchain_adversary.rs::source_pin_parser_preserves_legacy_and_rejects_incomplete_source_records` — Legacy release-only records remain readable; partial, whitespace-repository and malformed-length source identities refuse. Current status: green.

Command (exit 0):

```sh
set -o pipefail
RUSTC_WRAPPER= TMPDIR="$PWD/.local/tmp/cli-wave" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connectors-spec --test toolchain_adversary source_pin_parser_preserves_legacy_and_rejects_incomplete_source_records -- --exact 2>&1 | tee .local/tmp/cli-wave/adversary-toolchain-r1-case4.log
```

Verbatim output:

```text
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running tests/toolchain_adversary.rs (target/debug/deps/toolchain_adversary-e2f1c17116d36dcc)

running 1 test
test source_pin_parser_preserves_legacy_and_rejects_incomplete_source_records ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

```

## 3. Broader suite

Sections 1–2 were written before this suite ran. The implementing report records 10 existing library cases, including the disclosed pending generated-manifest assertion. The coordinator explicitly instructed this scoped review run to exclude only `toolchain::tests::committed_manifest_uses_the_repository_pin`; the comparable admitted subset is therefore 9 existing cases, and this review runs 13 after its four additions. No case was edited, weakened, or disabled in source. The full unfiltered package result is not claimed green.

Command (exit 0):

```sh
set -o pipefail
RUSTC_WRAPPER= TMPDIR="$PWD/.local/tmp/cli-wave" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p connectors-spec --lib --test toolchain_adversary -- --skip toolchain::tests::committed_manifest_uses_the_repository_pin 2>&1 | tee .local/tmp/cli-wave/adversary-toolchain-r1-suite.log
```

Verbatim output:

```text
   Compiling connectors-spec v0.1.0 (/home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-toolchain-20260909/crates/connectors-spec)
    Finished `test` profile [unoptimized] target(s) in 0.72s
     Running unittests src/lib.rs (target/debug/deps/connectors_spec-07ca01f7d42b2ff0)

running 9 tests
test toolchain::tests::default_search_is_independent_of_path_version_order ... ok
test toolchain::tests::exact_source_pin_refuses_same_version_without_receipt ... ok
test toolchain::tests::exact_source_pin_rejects_wrong_source_with_the_same_version ... ok
test toolchain::tests::altered_binary_is_refused_before_it_can_execute ... ok
test toolchain::tests::explicit_overrides_win_and_wrong_paths_do_not_fall_back ... ok
test toolchain::tests::invalid_receipts_are_closed_and_cannot_admit_a_tool ... ok
test toolchain::tests::legacy_release_records_remain_explicit_and_source_records_are_strict ... ok
test toolchain::tests::missing_pin_reports_record_and_search_locations_and_cache_is_supported ... ok
test toolchain::tests::source_admission_requires_the_exact_clean_root_and_keeps_ignored_scratch_separate ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.02s

     Running tests/toolchain_adversary.rs (target/debug/deps/toolchain_adversary-e2f1c17116d36dcc)

running 4 tests
test source_pin_parser_preserves_legacy_and_rejects_incomplete_source_records ... ok
test explicit_wrong_source_refuses_even_when_it_reports_the_pinned_release ... ok
test malformed_or_wrong_identity_receipt_never_executes_matching_bytes ... ok
test symlink_alias_cannot_supply_a_receipt_for_an_unproven_target ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Additional checks: `rustfmt --check --edition 2024 crates/connectors-spec/tests/toolchain_adversary.rs` and `git diff --check` both exited 0 with no output. `sha256sum --check .local/tmp/cli-wave/adversary-toolchain-r1-entry.sha256` exited 0 and reported OK for all four inherited implementation files. No broader lint or provider/generation run was made by this adversary.

## 4. Judgement findings

Nothing found. No judgement findings or confirmed failing cases are returned for the four-file working-tree change based on 88c036562f6ae011bd5cb0ce8e4d4671146b4922. This is an agent review result; it approves nothing and does not satisfy independent or human verification requirements.

## 5. Attacked boundaries and limits

- Receipt admission: marker-writing same-version fixtures show missing, malformed, source-mismatched and metadata-mismatched receipts refuse before execution; the valid control executes.
- Receipt-to-executable binding: existing digest mutation case and the new symlink case pass; a receipt beside an alias cannot replace the receipt beside the canonical executable.
- Explicit override behavior: the new public resolver refusal and existing flag/environment precedence/no-fallback cases pass; the default search cases also pass.
- Pin compatibility: the legacy release-only reader and incomplete exact-source refusal cases pass. The current reviewed pin remains the intermediate source commit 19de6406f97dca339136d7c9075ecc9b8fdb7af7.
- Clean source admission: the existing focused fixture checks exact HEAD, root, tracked/nonignored-untracked changes and separation of ignored scratch; it passes. This fixture does not execute `build_from_source`.
- Committed build snapshot: static inspection of `build_from_source` found independent clone, explicit pinned detached checkout, clean-tree checks before and after Cargo, submodule refusal, lock digest checks, bounded Cargo arguments, staged receipt/digest validation and directory installation. No source clone, ESS install, or actual ESS build was run by this adversary; those implementation paths are not claimed executable-tested here.
- Provenance: inspected the `v2::generate` caller's resolution/check path and `ess_source` manifest emission. Actual generation, final-source pin selection and the known manifest integration assertion remain with the coordinator. This disclosed pending integration is not a new finding.
- Reviewed documents: source-pin and build-receipt claims in `docs/gitlab-generation.md`, the story's exact-source acceptance, README/VISION, design and development guidance; the documented toolchain dispatcher remains the coordinator-owned wiring already disclosed by the implementor.

## 6. Outside-worktree writes and handoff

None. Cargo output was confined to this assigned checkout's `target/`; temporary fixtures and logs used its assigned `.local/tmp/cli-wave/`. Worktree lifecycle commands updated their own registry for this review's lease; no other checkout, planning store, external integration, or recovery/publication destination was written.

Retained inside this checkout: the new test file, `adversary-toolchain-r1.md`, `adversary-toolchain-r1-entry.sha256`, the initial infrastructure-refusal log `adversary-toolchain-r1-case1.log`, the four successful single-case logs `adversary-toolchain-r1-case1-direct.log` and `adversary-toolchain-r1-case2.log` through `adversary-toolchain-r1-case4.log`, and `adversary-toolchain-r1-suite.log`, all evidence files under `.local/tmp/cli-wave/`. The exact tree path is /home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-toolchain-20260909; id cli-toolchain-20260909, branch work/cli-toolchain-20260909. No commit or publication was made. Root owns test transfer/integration, verbatim review recording, final-source build/regeneration, preservation and managed cleanup. The Cargo slot has been released; this reviewer releases only lease cli-toolchain-review-finish-20260909 at handoff.

## 7. Findings

```findings
[]
```
