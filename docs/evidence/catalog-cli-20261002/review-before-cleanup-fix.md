unit: catalog whole candidate over f3fb222b7edc7fc29520dd30effdb58bfdec5274; source manifest ccb9dd9b0b041116d20ffd8c14f95ba166a624a5550582f29e352e4a23304c81
verdict: NEEDS-CHANGE — one introduced source-level cleanup safety finding
cases: inherited final ordinary 332 passed/21 ignored plus 2 selected fault cases passed; adversary executions 0, red 0; no after-run count
origin: introduced 1 / pre-existing 0 / undecided 0 findings
wrote-outside-worktree: own worktree lease registry only
needs-coordinator: route narrow cleanup correction and safe task-owned negative control; then affected verification and final gate

```text
$ git --no-pager diff --stat
 adapters/catalog/tests/local_runtime.rs            |  91 +++++++++++++-
 .../catalog/tests/local_runtime/cli_journey.rs     | 135 +++++++++++++++++++--
 2 files changed, 216 insertions(+), 10 deletions(-)
```

That is the author's tracked diff, not reviewer edits. Six author-owned untracked modules also belong to this candidate: background_recovery.rs, guarded_merge.rs, lifecycle.rs, owner_replay.rs, recorded_state.rs and settlement_fault.rs. The complete author delta is 3,451 additions/10 deletions across eight test files. **Reviewer source/test delta is zero.** Only this assigned report was written. No test, build, provider call, signal experiment or AEP mutation occurred. No approval or independence claim is made.

## Cases and runs

No new case was added or executed. Deliberately forcing PID reuse and signalling a possibly unrelated process is not an acceptable reproduction. A constructed call passing an unrelated pidfd directly to the cleanup function would only demonstrate a fabricated input, not reproduce the acquisition race. The coordinator explicitly accepted a source-grounded finding and will scope a safe negative control. There is no red test output to claim.

Read-only checks validated all eight current source hashes against `seccomp-final-source.sha256`. The final author public report read is `report-final-public.md`, SHA `ef4ca397d129fa98b021f289dcc07d13502ba213f75fcdde599a6a6ce63efa3c`. Summing the retained ordinary runner summaries gives 44 summaries, 332 passed, 0 failed, 21 ignored. Retained final selected results are one case containing control12/8/9/10, passed in 73.54s, and separate mode11, passed in 23.10s. These are author executions, not new reviewer runs.

Parsing the retained after-response seccomp records gives: control12 denied=0, modes8/9/10/11 denied=4 each, errors=[], canceled=0, denied syscall set=[18] (pwrite64). Exact-target WAL identities and real CONTINUE records are retained. The report's consolidated ten logical obligations/fifteen historical variants span multiple retained runs; they are not fifteen new executions against the final binary. The final binary's two selected cases and 332 ordinary cases do not execute all 21 ignored entries.

## Finding

| File:line | Verdict | Origin | Severity | Finding |
| --- | --- | --- | --- | --- |
| adapters/catalog/tests/local_runtime/guarded_merge.rs:83 | NEEDS-CHANGE | introduced | blocker | Cleanup now signals child pidfds captured from reusable numeric PIDs without validating that the captured process is still the owned child. |

The capture at `guarded_merge.rs:59–72` reads `/proc/<held-owner-pid>/task/*/children`, parses each numeric PID and calls pidfd_open. It does not validate the captured child's parent/start identity after acquisition. Its existing safety comment explicitly says the resulting handles are only polled and never signalled. The new `finish_filtered_owner` combines these child handles with the strongly identified SO_PEERPIDFD owner handle (`:83`) and sends SIGKILL when a five-second poll does not report exit (`:91–100`).

What reaches it: fault-enabled `Cli::shutdown` captures owner_handles before shutdown and passes them to finish_filtered_owner (`cli_journey.rs:237–241`). This path is also reached during unwind. If a listed native child exits and is reaped between the children-list read and pidfd_open, its numeric PID can be reused. The resulting pidfd then stably names the replacement process; stability after acquisition does not prove ownership at acquisition. A live unrelated replacement survives the grace period and can receive SIGKILL. No raced child exit or wrong-process signal was measured in this pass; the concrete finding is the unvalidated acquisition-to-signal code path. It is introduced by making the formerly observation-only handles signal-capable.

Narrow correction: keep list-derived child handles observation-only, or establish exact ownership after pidfd acquisition while the held owner identity remains live, before granting signalling authority. Do not solve this by broader process scanning or numeric-PID kills. The owner obtained with SO_PEERPIDFD has a stronger provenance and should not be conflated with an unvalidated child handle. A task-owned negative control should prove an unowned supplied/captured process cannot be signalled by the corrected seam, while exact owned cleanup still works. The implementor/coordinator owns the precise test seam and fix; this reviewer made neither.

## Remaining signal and fixture audit

- `settlement_fault.rs:665–694` captures a notifying task's Tgid, opens its pidfd, and checks the same notification ID remains valid before storage. Cancellation drops that new descriptor. This is a materially different ownership basis: the live blocked notification belongs to the fixture's inherited filter. The id is also checked around notification processing. No unrelated-signal path through a newly stored notification-derived handle was established. The numeric-Tgid cache can retain an old exited handle if that number later reappears; that old pidfd cannot retarget an unrelated process, but the cache is not proof of complete coverage of all future reused numbers. No PID-wrap cleanup experiment was run.
- Pre-exec filter installation uses prebuilt BPF, stack ancillary data and syscalls, with the listener thread started before spawn; SCM_RIGHTS receive validates shape/count/truncation and owns received descriptors before refusal. The parent observer is outside the filter. Target inspection checks exact path, regular-file device/inode and a second mapping observation. It does not atomically freeze the tracee fd table; the author correctly disclaims a security boundary. Unknown mapping records an error and disables guessed EIO. No adversarial fd-reuse, syscall-error or monitor-panic injection was run, so successful normal logs are not generalized into fault-proof cleanup.
- Listener lifetime extends through disarm and normal exact owner exit, then monitor joins. Normal logs record graceful exits and reacquired lifetime locks. Controller fallback keeps CONTINUE service while it retires notification-derived process handles. The finding above concerns the separate owner_handles cleanup route, not justification for discarding those lifetime controls.
- Arming follows held actual native response and original typed Dispatching/Pending, Spent redemption and incomplete audit. The mode8/9 recovery branch selects from a quiescent pre-retry state (`guarded_merge.rs:899–909`), accepts only Dispatching/Pending or Indeterminate/Quarantined with absent timestamps, and preserves unconditional original attempt/request, replayed unknown, complete audit, quarantine and no extra PUT/effect assertions (`:934–959`). This is not selection based on whatever result arrives.
- The revoked restoration control drains the existing observer before opening a raw database fd (`guarded_merge.rs:190–205`). It requires O_RDONLY and exact inode, actual fsync after filter installation, CLI --version exit0, a fresh exact-target fsync CONTINUE and no new denial (`settlement_fault.rs:251–319`). It then compares typed state, provider calls/effects and owner absence. This proves fixture fsync restoration, not a write or production recovery by revoked replay. The source and report preserve that distinction.
- The same-image owner replay uses the real inherited startup/lifetime descriptors and unmodified owner::serve, genuine same-executable handshake, and start=false. Typed ER observation retains exact identity joins rather than old business-table SQL. Serialized snapshot evidence includes the four named terminal collections, not a claimed full event-history archive; fictional fixture values and typed references are serialized, not credential/proof/private-key material. No additional concrete finding was established at those boundaries.

## Earlier evidence reuse and necessary verification

The earlier lifecycle, background_recovery and owner_replay module hashes remain identical. The fixture provider `local_runtime.rs` hash also matches the first final-source manifest. `recorded_state.rs` gained evidence/assert_unchanged methods used by the fault cases; its existing capture and join methods remain the ones reviewed earlier. Cli now carries `fault: None` by default, and nonfault commands take the original path; shutdown's new exact-handle branch is conditional on Some. Explicitly taking/dropping the recorded Reader after shutdown preserves its prior field-drop ordering for None. The owner_process extraction retains its exactly-one-child check and original crash path. These are source-grounded reasons why adding seccomp does not itself require a blanket rerun of unaffected lifecycle/modes0–7.

That is a reuse judgement, not an exact-final-binary execution claim. The earliest lifecycle greens predate prior helper/root-permission refinements, as the author discloses; equal production binary hashes alone are insufficient evidence of equal test inputs. No fresh expiry, background or same-image replay execution occurred in this review. If the cleanup fix changes shared capture semantics rather than only restricting the new signal route, the applied/refused/lost-response case (modes0–3) and background live-work case (mode6) exercise owner_process and are directly affected rerun candidates. If correction is confined to fault-only cleanup, rerun the two affected fault selections (control12/8/9/10 and11), plus its safe negative control and appropriate package checks. Root owns final rerun selection and the required integrated gate.

## Fixed identities and handoff

```text
source manifest ccb9dd9b0b041116d20ffd8c14f95ba166a624a5550582f29e352e4a23304c81
binary manifest d2dbc4c098c9418b5b37e591755bdbcc0354b6627a9468add2c47c051b3c2421
d471f3ec4add0ff99a760080a9fb7138217d0092349164d6cda2e98a8d8bad71  local_runtime.rs
877685b55949a230236f55a9a414e4374b847b0df33e6c0bd652e825df6c85df  local_runtime/cli_journey.rs
9ee3ae85475d56677ac91fd3e231f21eca039d9d97116a177f8dda37dd3134e6  local_runtime/guarded_merge.rs
06e6318eb2b8700e8690e466be9f5e6bbae261eb3c68f7c6fa9d2422ae4dff62  local_runtime/background_recovery.rs
3fceddf2a40fb76784cfd77a7d0cbd4c674cee86a3cba2c35fb84567b8f94deb  local_runtime/lifecycle.rs
1ddcf26de557b001278361a81713b7f4da089ccc7e68a72e89d50d9df6f222c2  local_runtime/owner_replay.rs
9034be65cb8d754d4284dafd2be7bd8431435d1e9163960a30d288b1b5643765  local_runtime/recorded_state.rs
087868cc1b758962101a07e62d8547bfa7e7d90035077bcb38b17eb051eb8e06  local_runtime/settlement_fault.rs
d2f9b0e66b85fa4722e3d6f09b9dacd908612630616720c541057d97bfad1890  final local_runtime test executable
```

Source paths above are under adapters/catalog/tests/. Own lease codex-catalog-final-review is released with report handoff. Outside-worktree writes are limited to lifecycle CLI updates in `$HOME/.local/state/worktree/registry.sqlite3` and SQLite-managed transaction files if used. No author outputs were removed. Root owns routing, correction scope, gate and publication.

```findings
- file: adapters/catalog/tests/local_runtime/guarded_merge.rs
  line: 83
  category: judgement
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Cleanup now signals child pidfds captured from reusable numeric PIDs without validating that the captured process is still the owned child. A child exit and PID reuse between the proc children-list read and pidfd_open can capture an unrelated process which the new timeout fallback then signals. This is source-grounded; no wrong-process signal was reproduced.
```
