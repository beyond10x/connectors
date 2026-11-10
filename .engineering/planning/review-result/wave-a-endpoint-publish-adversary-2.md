---
format: aep.planning-md/3
id: review-result:wave-a-endpoint-publish-adversary-2
kind: review-result
status: active
title: Adversary pass 2 on endpoint-publish-holds-one-writer (wave A)
relations:
- reviews: story:endpoint-publish-holds-one-writer
revision: 1
---
```
unit: story:endpoint-publish-holds-one-writer, findings cover HEAD e82cff3b8 (git diff 5087593c1..HEAD)
verdict: CONFIRMED (one acceptance gap, warning), plus 2 notes
cases: executed 1→2 (bin local::endpoint::tests, scratch copy of HEAD), red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory: <scratch>/adv2/ (copy/ 968M incl. its target/, case.rs, case.patch, case.sh, case.log, wincheck.sh, wincheck.log, suite.sh, suite.log)
needs-coordinator: decide whether the different-state-directory race holds the unit (last pass)
```

1. `git --no-pager diff --stat` in the worktree: empty. I wrote nothing there. The test I added sits in the module's `#[cfg(test)]` block, and that file is code, so I put it in a scratch copy (`git archive HEAD`). The patch is `<scratch>/adv2/case.patch`.
2. Added case `writers_of_one_file_with_their_own_state_replace_it_at_most_once`. It runs 8 writers on one `config.toml`, each with its own state directory, with the window held at 100 ms. It is red now. Its first run, alone:
   `assertion left == right failed: [... "# writer 0" ... "# writer 7"] left: 8 right: 1`, then `test result: FAILED. 0 passed; 1 failed; ... 56 filtered out`, exit 101.
3. Targeted runs, after the case existed:
   - Worktree: `adversary_d218_u2` 16 passed, `connect_endpoint` 7 passed, `connect_endpoint_adversary` 10 passed, exit 0.
   - Worktree bin `local::endpoint::tests`: 1 passed, exit 0.
   - Scratch copy: 1 passed, 1 failed, exit 101.
   - Windows: `cargo check` and `clippy -D warnings`, bin test profile, `--target x86_64-pc-windows-gnu`, exit 0 for both.
4. Findings:
   - **The lock is keyed to the state directory, but the file it protects is the configuration (`endpoint.rs:526`).** Two connects on one config with different `--state-dir` or `XDG_STATE_HOME` are not serialized, so the Acceptance fails read literally ("concurrent connects ... into one instance"). The `endpoint_lock` comment ("every writer of this file holds the endpoint lock") is not true for them.
     - What reaches it: the global `--state-dir` flag, and `setup init` against an existing configuration with a new state directory (documented as `state_initialized`).
     - Origin: round 1's lock on the configuration directory covered this case; e82cff3b8 reopened it.
     - Acceptance holds without a fix only for the shared-state scenario that CI hit.
     - Fix (not applied): a lock keyed to the configuration, for example lock-then-verify on the configuration file itself.
   - **The endpoint section of the contract and the storage doc do not mention the new behaviour.** `contracts/cli/v1alpha1/semantics.md` §"Giving an instance its endpoint" names no lock and no `metadata_unavailable` refusal. `docs/local-runtime-foundation.md:85` does not list `endpoint.lock` among the state files the host owns.
     - `metadata_unavailable` is a valid code (`ess/domains/cli.yaml:710`, exit 1).
     - Its `next_action = retry_status` misleads when nothing was written. `revision_conflict` / `retry_explicitly` fits a writer that holds the lock past 30 s.
   - **The lock file is opened by path and follows a symlink (`endpoint.rs:503`).** `metadata.lock` is opened handle-relative and owner-checked (`fs::private_file_at`, `metadata.rs:1422`). What reaches it: only a process with the same uid inside the 0700 state directory, so this is a note.
5. Attacked and could not break:
   - Pass 1 F1, F2 and F3 are answered:
     - The Windows target compiles.
     - The configuration directory is no longer locked.
     - A lock failure is no longer `invalid_configuration`.
   - No targeted test asserts that the state directory is absent after connect (`local_cli.rs:161` covers clock-check only).
   - `setup init` / `state_initialized` checks for the database, not the directory, so a state directory that already exists is harmless.
   - The lock file is 0600 and the directory 0700.
   - There is no nested lock and no deadlock with `metadata.lock`.
6. Paths written outside the worktree: `<scratch>/units/endpoint-publish-holds-one-writer/scratch/adv2/` (all of it, including `copy/` at 968M with its `target/`).

```findings
[
  {"file": "apps/connectors/src/local/endpoint.rs", "line": 526, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "endpoint_lock is keyed to paths.state, so connects writing one configuration with different --state-dir/XDG_STATE_HOME are not serialized and all 8 writers replaced the file (round 1's configuration-directory lock covered this; e82cff3b8 reopened it)"},
  {"file": "contracts/cli/v1alpha1/semantics.md", "line": 490, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the endpoint-write section and docs/local-runtime-foundation.md:85 do not mention the endpoint.lock state file or the new metadata_unavailable (observation, retry_status) refusal, whose next_action misleads when nothing was written"},
  {"file": "apps/connectors/src/local/endpoint.rs", "line": 503, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "endpoint.lock is opened by path with create and follows a symlink, unlike metadata.lock's handle-relative owner-checked private_file_at; only a process with the same uid inside the 0700 state directory reaches it"}
]
```
