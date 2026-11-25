---
format: aep.planning-md/3
id: review-result:debug-override-adversary-pass-1
kind: review-result
status: active
title: Adversary read of the debug-only override helper, first commit
relations:
- reviews: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
revision: 1
---
Verdict: no finding holds the change. For the first commit of the change of
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build. Two
introduced findings are fixed in the second commit of the same pull request; two are limits of the
scan and are named in its module comment; one existed before and has its own draft story.

Adversary read of the committed change. The adversary added four cases against the scan, in a
module and a fixture of its own, uncommitted, and ran them with the change's tests on the build
host of the shared pool: 7 tests run, 3 passed (the change's), 4 failed (its own). It edited no
product line and applied no mutant. It did not run the 17 tests of package `connectors` that set
the older variables.

## Findings

1. Introduced, shown by three failing cases, nothing reaches it today:
   `crates/connectors-build/tests/debug_override_reads.rs`. The scan does not report a literal
   after a comment inside the call, a literal inside `OsStr::new`, or a literal handed to
   `std::env::var` imported under another name. No source file uses one of these forms.
   Outcome: the module comment names the three forms as limits. The scan is not widened.
2. Introduced, shown by one failing case, nothing reaches it today: a read through
   `std::env::vars()` or `vars_os()` compared with the name is not reported. The four calls of
   `vars` or `vars_os` in product sources compare no override name.
   Outcome: named in the module comment as a limit.
3. Introduced, read and not run: nothing pinned `release_build_ignores_a_debug_override`. Deleted
   or renamed out of `release_build_`, the lane stayed green on
   `release_build_has_no_debug_assertions`, the one test `crates/connectors-build/src/ignored.rs`
   pinned.
   Outcome: fixed. `ignored.rs` pins the test by its text; shown red by a rename.
4. Introduced, read and not run: with `adapters` dropped from the scan's list of directories all
   three scan tests stayed green.
   Outcome: fixed. The scan asserts that it read `adapters/catalog/src/lib.rs`; shown red with
   the directory dropped.
5. Existed before: `contracts/cli/v1alpha1/owner.md` line 80 says a debug CLI passes
   `CONNECTORS_TEST_OWNER_PARENT` alone to the owner. The parent commit passed seven names.
   Outcome: a draft story,
   story:the-owner-contract-names-every-test-variable-a-debug-cli-passes-to-the-owner.

## Attacked and not broken

- No path reads an override in a build without `debug_assertions`. `cargo check --release` of
  `connectors-sdk`, `connectors-host` and `connectors` ended 0 without a warning.
- `release_build_ignores_a_debug_override` runs under the lane's command (2 tests run, both
  passed). A child run that is empty or filtered out panics on "the child printed no value".
- The three sites parse as before: `0` gives 0, `+250` gives 250; empty, negative, blank,
  overflowing, non-numeric and non-UTF-8 values give the production value. No site lost or gained
  an override.
- `owner_environment` on Unix in a debug build adds only the prefix family; a release build
  passes the installation directory alone. The Windows spawn is untouched and its lint is green.
- With `CONNECTORS_TEST_OVERRIDE_EXAMPLE_MS=7` in the runner's environment the helper's tests
  pass under `cargo test` and under nextest.
- No false report of the scan in the tree.
