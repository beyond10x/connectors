---
format: aep.planning-md/3
id: review-result:wave-a-macos-programs-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on provider-programs-run-on-macos (wave A, issue 229)
relations:
- reviews: story:provider-programs-run-on-macos
revision: 1
---
```
unit: story:provider-programs-run-on-macos, committed tree 9069c43a4..1ad0dca16 (worktree cz-u229b, clean)
verdict: green
cases: executed n/a→n/a (no case added: macOS-only code, cannot compile or run here; one existing ignored Linux case run as a probe, green), red 0
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 5 paths under the unit scratch dir (part 6)
needs-coordinator: yes
```

Nothing I found blocks the Acceptance. The Acceptance itself is not proven yet: the round-2 head has had no macOS CI run.

1. `git --no-pager diff --stat`: empty. Nothing was changed in the worktree.

2. Cases added: none. Every attack target is `#[cfg(target_os = "macos")]`. connectors-host and apps/connectors do not build for aarch64-apple-darwin here (ring's C build fails). A case I can't compile could break the macOS CI step, so I didn't write one.
   - Probe (an existing case, not mine): `cargo test --locked -p connectors-grafana --test local_runtime -- --exact cli_journey::grafana_cli_connects_with_a_service_account_token_and_lists_datasources_on_the_saved_connection --include-ignored` gave `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 21.70s`, exit 0. So the shared journey body and the new `setup check` assertions still work on Linux.
   - `cargo clippy --locked --target aarch64-apple-darwin -p connectors-custody --all-targets -- -D warnings`: exit 0.

3. Suite run: none. I added no case, so there was nothing to run it after.

4. Findings:
   - **Contract drift** (`contracts/cli/v1alpha1/semantics.md:2070`, NEEDS-CHANGE, introduced). Four documents still say the copy is removed by its launch or kept only until the greeting:
     - `semantics.md:2070`
     - `ess/domains/cli.yaml:536` (which regenerates `website/docs/reference/contracts/cli.md:2085`)
     - `owner.md:338-340`
     - the module doc at `macos.rs:5`

     In fact the owner now keeps its copy for its whole run and removes it itself. The test doc at `transport.rs:3957` is stale the same way. Every reader of those documents reaches this.
   - **A killed owner leaves its copy** (`crates/connectors-host/src/local/runtime/artifact/macos.rs:351`, CONFIRMED by reading, introduced). `OwnCopy::drop` runs only when `serve_with_read_policy` returns. The owner installs no SIGTERM handler, and macOS has no stale-copy sweep (Windows has `artifact::enter`). So SIGKILL, SIGTERM or a crash leaves a `connectors-image.*` directory holding a full binary in `$TMPDIR`. Before this unit the CLI removed the copy right after the greeting.
     - What reaches it: `kill`, a force quit, jetsam, logout. Nothing in the product sends these signals.
     - Fix, in short: sweep dead owners' copies when the next owner starts, as Windows does, or say in the contract that they remain.
   - **Mutant the suite misses** (`macos.rs:366`, CONFIRMED by reading, introduced). `own_copy()` is never checked in the case where it should find the copy. The only test (`macos.rs:811`) asserts `is_none()` and builds `OwnCopy` by hand. If `own_copy` always returned `None`, or `transport.rs:1580` were deleted, every owner would leak its copy and no test on any platform would fail; the CI journey doesn't look at `$TMPDIR`.
     - Case to add (macOS): launch the test binary from a launch copy, have the child assert `own_copy().is_some()` and drop it, and have the parent assert the directory is gone.
   - **Journey not ignored on macOS** (`adapters/grafana/tests/local_runtime/cli_journey.rs:584`, warning, introduced). On a developer Mac, `cargo test -p connectors-grafana` panics without `CONNECTORS_TEST_CLI`. With the variable set, every run leaves credential items in the user's real login keychain that the test cannot delete (it only reports this, at line 609). CI skips the test by name in "Test the programs".
   - **syslog channel unproven** (`crates/connectors-custody/src/keychain.rs:75`, note, INFEASIBLE here). The only observed cause text came from the `cfg(test)` `REPORTED` hook (run 42713022257). No CI log yet shows a `connectors custody:` line in `log show`. In round 1 the journey's "owner errors" section printed only its header, and no write failed in that run.

5. Attacked and could not break:
   - Exit paths in `serve`: early `?` returns, panics and normal return all drop the copy. Removal races with the CLI's `Launch` are safe (identity check, no double ownership).
   - Timeout reaper path: the copy stays for the owner.
   - Swap by a same-user process: build is measured once (OnceLock), decrypt is any-application, and `OwnCopy::drop` checks the file identity. At most a same-user denial of service.
   - Linux and Windows: the cfg arms are unchanged and the Linux journey probe is green.
   - Pinned ENOENT assertion: the marker parsing matches the round-1 stdout exactly. It goes red only if Apple changes the status or the lookup before the add fails first, which is what the pin is for.
   - Journey Acceptance: `connect` and `datasources.list` are asserted, and the saved item is shown to be in the login keychain. CI must show `test result: ok. 1 passed` in the journey step.

6. Paths written outside the worktree: five files under `<scratch>/units/provider-programs-run-on-macos/scratch/` (adv-custody-clippy-mac.log, adv-cli-build.log, adv-linux-journey.log, adv-fakecc/cc, adv-fakecc/ar).

   Inside the worktree, gitignored: `target/debug/connectors` and the grafana test build from the probe, and the `.local/tmp` directory.

7.
```findings
[
  {"file": "contracts/cli/v1alpha1/semantics.md", "line": 2070, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "semantics.md:2070, cli.yaml:536 (and the generated cli.md:2085), owner.md:338 and macos.rs:5 still say a launched copy is removed by its launch or kept only until the greeting, but the macOS owner now keeps its copy for its whole run and removes it itself"},
  {"file": "crates/connectors-host/src/local/runtime/artifact/macos.rs", "line": 351, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "OwnCopy::drop runs only when serve returns, so an owner ended by SIGKILL, SIGTERM or a crash leaves its connectors-image copy in TMPDIR, and macOS has no stale-copy sweep at owner start as Windows does"},
  {"file": "crates/connectors-host/src/local/runtime/artifact/macos.rs", "line": 366, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "own_copy() is never tested in the case where it should find the copy (the only test asserts is_none and builds OwnCopy by hand), so a mutant that always returns None, or deleting transport.rs:1580, leaks every owner's copy with every test green"},
  {"file": "adapters/grafana/tests/local_runtime/cli_journey.rs", "line": 584, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the macOS journey is not ignored: on a developer Mac the package tests panic without CONNECTORS_TEST_CLI, and with it each run leaves credential items in the real login keychain that the test cannot delete"},
  {"file": "crates/connectors-custody/src/keychain.rs", "line": 75, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "no CI log yet shows a syslog connectors custody line in log show (the only observed cause text came from the cfg(test) REPORTED hook), so the owner's diagnostic channel is unproven"}
]
```
