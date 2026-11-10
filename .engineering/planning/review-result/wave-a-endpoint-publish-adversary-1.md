---
format: aep.planning-md/3
id: review-result:wave-a-endpoint-publish-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on endpoint-publish-holds-one-writer (wave A)
relations:
- reviews: story:endpoint-publish-holds-one-writer
revision: 1
---
```
unit: story:endpoint-publish-holds-one-writer (95590f0ba, git diff 5087593c1..HEAD)
verdict: red
cases: executed 17→17 (implementor's count; no case added), red 0 (the red evidence is a compiler run, not a case)
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/units/endpoint-publish-holds-one-writer/scratch/{wincheck.sh,wincheck.log,unittest.sh,unittest.log}
needs-coordinator: yes
```

1. `git --no-pager diff --stat`: empty, so the worktree is untouched. `.local/tmp` and `target/x86_64-pc-windows-gnu` are ignored by git.
2. I added no cases. The red evidence comes from a program: a check of the bin's test target for the Windows target, `cargo check --locked -p connectors --bin connectors --profile test --target x86_64-pc-windows-gnu`, exit 101:
   ```
   apps/connectors/src/local/endpoint.rs:526:22: error[E0433]: cannot find `unix` in `os`: could not find `unix` in `os`
   apps/connectors/src/local/endpoint.rs:531:65: error[E0599]: no associated function or constant named `from_mode` found for struct `std::fs::Permissions` in the current scope
   error: could not compile `connectors` (bin "connectors" test) due to 2 previous errors
   ```
3. Suite: I did not run it because I added no case. I ran the unit's own test alone on Linux three times (`cargo test --locked -p connectors --bin connectors -- writers_of_one_text_replace_it_at_most_once`). Each run: `test result: ok. 1 passed; 0 failed; ... 55 filtered out`, exit 0.
4. Findings:
   - **The Windows CI job stops compiling (blocker, NEEDS-CHANGE, introduced).** `endpoint.rs:526`: the new test uses `std::os::unix::fs::PermissionsExt` with no `#[cfg(unix)]`. What reaches it: `.github/workflows/windows-host.yml` runs `cargo build --locked --workspace --all-targets` and `cargo test -p connectors`, so the whole Windows host job goes red. The fix (not applied): gate the test or the `set_permissions` call on `cfg(unix)`.
   - **Every endpoint write on Windows probably fails (blocker, INFEASIBLE here, introduced).** `endpoint.rs:495`: `directory.lock()` is std `File::lock`, which on Windows calls `LockFileEx` on a directory handle. That handle is opened with `FILE_FLAG_BACKUP_SEMANTICS` and `FILE_LIST_DIRECTORY` access (`filesystem/windows.rs:514-524`). I believe NTFS refuses byte-range locks on a directory open, but this machine has no Windows host or wine, so it is unproven. If it does, every `connections connect --endpoint` on Windows fails with `invalid_configuration`; on the base it worked. Two things make this likely and hidden:
     - The rest of the codebase locks a dedicated lock file (`LOCK` plus `try_lock`, as in `metadata.rs:1459-1475`) and never a directory.
     - Nothing on Windows exercises this path: `adversary_d218_u2.rs` is Linux-only, `connect_endpoint.rs` is unix-only, and the new unit test does not compile there.

     The fix (not applied): take the lock on a lock file in the configuration's directory, as `metadata.rs` does, with a bounded `try_lock` loop.
   - **Lock failures get the wrong error name (warning, CONFIRMED, introduced).** `endpoint.rs:496`: a failed lock maps to `InvalidConfiguration`, which tells the operator the file is invalid when the lock is what failed. On Windows that would be every write.
   - Acceptance on Linux holds without fixing anything above. On Windows it only holds trivially: if the lock always fails, no connect goes on.
5. Attacked and could not break:
   - The check-then-rename race on Linux: flock is per open-file description, and every thread or process opens its own directory handle.
   - The lock is released on every error path: each `?` drops `directory`, and nothing duplicates its descriptor.
   - The read-back outside the lock: a different-instance writer can land between the publish and the read-back, which gives a spurious `revision_conflict` but never two connects going on. That behaviour is the same as on the base.
   - The connect-level race in `adversary_d218_u2`: a later reader of the winner's text is refused with `another_host`, and an earlier reader fails the check under the lock.
   - Mutants: moving the lock after the check, or ignoring a lock failure, would make the new test red (8 published).
6. Paths written outside the worktree: the four scratch files in the header.
7. The `findings` block:

```findings
[
  {"file": "apps/connectors/src/local/endpoint.rs", "line": 526, "category": "boundary", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the new unit test uses std::os::unix::fs::PermissionsExt without cfg(unix), so the connectors bin test target does not compile for Windows and windows-host.yml (build --all-targets, test -p connectors) goes red"},
  {"file": "apps/connectors/src/local/endpoint.rs", "line": 495, "category": "judgement", "severity": "blocker", "verdict": "INFEASIBLE", "origin": "introduced", "message": "std File::lock is LockFileEx on a FILE_FLAG_BACKUP_SEMANTICS directory handle, which NTFS is expected to refuse, so every connect --endpoint on Windows would fail; unverifiable without a Windows host and covered by no Windows test, while the codebase elsewhere locks a dedicated lock file"},
  {"file": "apps/connectors/src/local/endpoint.rs", "line": 496, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "a failed directory lock is reported as invalid_configuration, naming the file as invalid when the lock failed"}
]
```
