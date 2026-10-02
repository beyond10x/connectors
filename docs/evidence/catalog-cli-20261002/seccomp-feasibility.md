unit: bounded Linux seccomp USER_NOTIF settlement-fixture feasibility
verdict: kernel mechanism feasible on this host; ER settlement integration unproven
cases: one throwaway Rust smoke execution, passed; no production CLI or provider execution
origin: fixture feasibility; no production defect established
wrote-outside-worktree: worktree CLI lease metadata only
needs-coordinator: scope test-only integration and its acceptance before implementation

The candidate's existing control12 succeeded and chmod mode8 did not interrupt ER settlement; modes8–11 remain unmet. The retained remount probes also returned mount refusals while an already-open descriptor remained writable. This pass did not alter those outcomes, production code, catalog tests or AEP.

## Measured mechanism

Host: Linux 6.18.49-1-MANJARO, x86_64. `/proc/sys/kernel/seccomp/actions_avail` advertises `user_notif`. Kernel notification-size query returned 80/24/64 bytes, matching the explicit probe ABI structs. The one smoke source is `smoke.rs`, built directly with the existing `target/debug/deps/liblibc-00c0bc427f17191c.rlib`; no Cargo build or new dependency/cache was needed. Rustc was 1.98.1. Compile exit and execution exit are both 0, retained beside this report.

The parent opened two new task-owned files, forked a child and stayed outside the filter. Before self-exec, the child set NO_NEW_PRIVS, installed an x86_64 BPF filter with NEW_LISTENER, transferred its listener with SCM_RIGHTS and closed its listener copy. The file descriptors stayed open across exec. Only write-family, fsync/fdatasync and ftruncate syscalls were sent to the parent; ordinary syscalls remained allowed. The parent checked notification PID, `/proc/<pid>/fd/<fd>` path and device/inode against the exact target, validated the notification id, and returned CONTINUE unless both armed and matched. It never fabricated a successful syscall result or wrote on behalf of the child.

Actual single-run output:

```text
kernel notification sizes=80/24/64
notif pid=2185774 fd=3 syscall=1 target=true armed=false response=CONTINUE
notif pid=2185774 fd=3 syscall=74 target=true armed=false response=CONTINUE
ARM after genuine write/fsync, file=A
notif pid=2185774 fd=3 syscall=1 target=true armed=true response=EIO
notif pid=2185774 fd=3 syscall=74 target=true armed=true response=EIO
notif pid=2185774 fd=4 syscall=1 target=false armed=true response=CONTINUE
notif pid=2185774 fd=4 syscall=74 target=false armed=true response=CONTINUE
DISARM after target EIO write/fsync and unrelated real success
notif pid=2185774 fd=3 syscall=1 target=true armed=false response=CONTINUE
notif pid=2185774 fd=3 syscall=74 target=true armed=false response=CONTINUE
PASS child exit=0; EIO=2 CONTINUE=6; same preopened fd; target=AD unrelated=C; listener/control closed; exact files removed
```

The child itself asserted real write/fsync success before arm, errno EIO for both target syscalls while armed, real success on the unrelated file, and real success after disarm. Parent content checks establish that rejected B was absent and successful A/D/C reached the files. The exact child was waited with exit 0. Both files were removed after handles closed. The probe has a ten-second observer/wait deadline and an exact-child kill/wait guard for failure. No detached descendants or production processes were started by this probe. Source, binary and logs are retained as assigned debug scaffolding, not committed tooling.

```text
c5965db0e46b2f700c0fb884027ad92906ded8ee277b0e6dac0ba39a68e9620c  smoke.rs
8694808452633e32a16acf373d0ebe18886fcd18faa778cf6868950f31651712  smoke
0bad03c6169dece20b2f75524af75694b52ac15f3a5ea7a8533b1ddf017523be  smoke.log
```

## Source-grounded integration limits

The existing `MetadataFault` in `adapters/catalog/tests/local_runtime/guarded_merge.rs:144` only changes permissions. The actual failure arm is after the held provider response and typed `assert_dispatched_and_spent()` at lines 662–707 (and equivalent mode11 path at 574). Keep that order and original-attempt/audit assertions. Current ER provision/open gives the same metadata path to the real SQLite recorded provider (`crates/connectors-host/src/local/metadata/er.rs:1682`, `:1765`); this does not identify which exact syscall or WAL inode it uses to settle. That must be observed in the scoped integration.

1. Startup/inheritance: install before the first CLI that can start the owner, not just on the later invocation CLI. `Cli::command` is the narrow test seam (`adapters/catalog/tests/local_runtime/cli_journey.rs:148`). Production owner creation starts a separate session and execs a captured image (`crates/connectors-host/src/local/owner/transport.rs:673`). Kernel filter inheritance across fork/exec is documented; the smoke measured exec inheritance, not the actual owner/adapter descendant chain. NO_NEW_PRIVS compatibility and preserved real image/FD3/FD4 admission must pass a no-fault control.
2. Spawn deadlock: a multithreaded libtest must use an async-signal-safe pre_exec hook with prebuilt filter and stack-only sendmsg. Start the unfiltered listener service before `Command::spawn`, because spawn can wait for the child exec-error pipe, and an intercepted startup write can otherwise wait for a listener that has not started. The smoke deliberately used a single-threaded fork parent; it is not copy-paste production fixture code. Do not intercept the SCM_RIGHTS bootstrap sendmsg or parent barrier control.
3. Exact files and syscalls: enumerate only the assigned canonical state directory's exact metadata.sqlite3, metadata.sqlite3-wal, metadata.sqlite3-shm and, if actually used, metadata.sqlite3-journal. Match regular-file device/inode and exact path per notification; account for legitimate journal recreation with fresh identities. No broad filename prefix, unrelated directory, other process scan or writable-file corruption. Test write, pwrite64, writev, pwritev/pwritev2, fsync/fdatasync and ftruncate if reached; the smoke exercised only write and fsync. Shared mmap writes and io_uring would not be intercepted by this set. Do not infer coverage from a nonzero listener count: retain the actual targeted syscall and original settlement state.
4. Mapping races: `/proc/<tid>/fd` access can refuse under credential/dumpability/proc policy. Notification ids can be cancelled, and PID is a notifying thread id. An ID_VALID check does not atomically freeze an fd table. In a multithreaded owner, another thread could close/reuse an fd after inspection; a path/inode check is not a general security boundary. Treat ambiguous/unreadable mapping or cancelled replies as fixture failure/diagnostic, never guessed EIO to another fd. A stable SQLite-handle lifetime in the actual target must be grounded; no universal exact-target guarantee is established by this single-thread smoke. No tracee memory reads or pointer-argument rewriting are needed.
5. Keep scope local: only descendants of the filtered owned CLI can notify. The test parent, provider fixture, custody daemons and unrelated processes remain unfiltered. An unfiltered recorded-state reader must not perform a competing settlement write while armed. Additional native-provider dispatch, changed deadline, falsified native response, changed audit outcome or altered ER semantics are not authorized by this mechanism.
6. Cleanup: production owner `setsid()` means the invoking CLI's process group cannot clean it up. Keep the listener alive and serving CONTINUE after disarm until exact owner/child shutdown is acknowledged or exact owned-process exit is established. Closing the last listener while live tracees remain changes notifications to ENOSYS; it is not restoration. Join observer, close socket/listener copies and reap owned direct children on every terminal path. Never signal a reused PID/group; use owned handles/pidfds or existing exact-incarnation shutdown. The smoke proved cleanup only for its one direct child.

The Linux kernel documentation describes inheritance, listener transfer and notification-id handling; the local seccomp.h additionally cautions that CONTINUE is not a safe general security-policy mechanism. This proposal uses it as a bounded fault fixture, with the races above explicitly unresolved for production integration. [Linux seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html)

## Smallest proposed source scope and acceptance

Scope only `adapters/catalog/tests/local_runtime/guarded_merge.rs`, the shared test `cli_journey.rs` command helper for an optional per-fixture controller, and one new sibling test-support module (for example `settlement_fault.rs`). Existing libc suffices. No production files, dependencies, contract semantics, new executable CLI or AEP writes. Enable the controller only for settlement modes8–11 and identical no-fault control12, before setup/connection can launch an owner. Mode-specific arming remains after held real response, original Dispatching/Pending, Spent redemption and incomplete admitted audit are proven; mode10/11 revoke completes before arm. Default is CONTINUE. Replace chmod only after this acceptance is scoped.

Acceptance must preserve all existing named outcomes and count modes8–11 individually; no readiness/unavailable shortcut may count as settlement refusal. Require:

- An unarmed filtered control12 has the same native Applied response, completed/replayable original attempt, complete audit and exactly one effect/PUT as the unfiltered control, with no injected errors.
- On each armed case, record at least one actual EIO reply to the exact metadata file identity after the barrier and before response completion, along with syscall, notification/task identity and arm epoch. If none occurs or state settled anyway, report the injection rejected, not a production defect.
- Preserve original returned known-effect/refused/unknown-outcome expectations, missing settlement/expiry, incomplete audit and original request/attempt/connection linkage. Disarm and verify restart/recovery without duplicate PUT/effect, including revoked non-disclosure in modes10/11. Require the original durable record rather than a replacement attempt.
- Exercise no-fault CONTINUE and unrelated-fd positive controls, startup refusal and observer teardown/error cleanup as test-only helpers. Validate kernel capability and ABI before selecting this mechanism; unsupported hosts get an actionable prerequisite failure, never a passing skip. Keep existing deadlines unchanged and record any overhead that makes control fail.

Ranked integration predictions to test only after authorization: (1) open-FD SQLite write bypass explains chmod failure; an observed armed WAL/database pwrite EIO leaves the original settlement pending. (2) the owner escaped filtering or uses an uncovered write mechanism; no matching notification appears despite completion. (3) the fault hits another earlier persistence step; durable/returned observations fail the required settlement-specific cause even with a notification. (4) observer startup or cleanup introduces deadlock; unarmed control fails its original bounds. The smoke supports mechanism (1)'s feasibility but decides none of these ER-specific predictions.

No second probe or catalog integration was executed. Coordinator owns the correction scope, raw evidence retention and final lifecycle. Own lease is released on report; only lifecycle CLI registry updates occur outside this worktree. Public home paths are written as `$HOME`; no credentials were inspected or retained here.
