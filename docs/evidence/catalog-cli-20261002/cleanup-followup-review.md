unit: catalog cleanup correction; frozen source manifest b545c005ffa28b7dbd19605909d388dde3fd843a02dcd805509ab5137b625c8f
verdict: NOTHING-FOUND — prior cleanup finding resolved within this bounded follow-up
cases: inherited ordinary before 332/0/21, after 334/0/21; author controls 2 passed; reviewer cases added 0, executions 0
origin: introduced 0 / pre-existing 0 / undecided 0 new findings
wrote-outside-worktree: own worktree lease registry only
needs-coordinator: record disposition and perform integrated gate; no further correction identified here

```text
$ git --no-pager diff --stat
 adapters/catalog/tests/local_runtime.rs            |  91 +++++++++++++-
 .../catalog/tests/local_runtime/cli_journey.rs     | 137 +++++++++++++++++++--
 2 files changed, 218 insertions(+), 10 deletions(-)
```

This is the author's tracked delta. Six author-owned untracked modules also belong to the candidate: background_recovery.rs, guarded_merge.rs, lifecycle.rs, owner_replay.rs, recorded_state.rs and settlement_fault.rs. The complete author delta is 3,593 additions/10 deletions across eight test files. Reviewer source/test delta is zero; only this report was written. No builds, tests, provider calls, process signals or AEP mutations were performed. The lease registry is under `$HOME/.local/state/worktree/registry.sqlite3`, including its managed SQLite transaction files. This report makes no approval or independence claim.

Scope and identities

Compared the preserved `cleanup-fix-before/` source with the frozen candidate and inspected the two new controls, retained author logs and affected shutdown/notification callers. Only cli_journey.rs, guarded_merge.rs and settlement_fault.rs changed from the preserved pre-fix source. The other five source files are unchanged. All eight current file hashes passed the supplied manifest check again at review completion.

- Current manifest: `cleanup-final-source.sha256`, SHA `b545c005ffa28b7dbd19605909d388dde3fd843a02dcd805509ab5137b625c8f`.
- Pre-fix manifest: `cleanup-fix-before/sources.sha256`, SHA `b6921bb5a6b39f83b5213f302355ad65ae7c59371fe21ec824465e394c4dc132`.
- Current cli_journey.rs: `cb330065dd81afd611fa33648c2cc47ae5d74774ebaf55839a7e607a0db8a374`.
- Current guarded_merge.rs: `c660af0d0e526c655076de71e3ca20267962d71cb2ecfa25e8fce93a60bb4a62`.
- Current settlement_fault.rs: `d992bd64bb920da5bf7731f1ccd0756072e7341651c7e972aa00c4418ad7495a`.
- Author report: `report-cleanup-final-public.md`, SHA `a3d6090bd12ec43a84d5ac00e06fd08e2bd9ca25f1d99677f3b7b8fdf933b6cc`.
- Binary manifest: `cleanup-final-binaries.sha256`, SHA `84b609ea5284acf24eeef09fcc74b2cc91a04801536d1733640e3787361ab8de`; final test binary SHA `1e91b30d108ff1e98ef990c720a173b0fdc2ba2edcb1a5d2649cee727e9a52e4`.

All evidence paths above are relative to `.local/provider-wave/catalog/`. Source citations below are relative to the repository.

Prior finding disposition

The blocker in `adversary-final/report.md` (SHA `feb48e9c79a8bdd79c1a28eb9a600fa840105cb16fad91fabce7bd8950a50653`) is resolved by separating signal authority. In `adapters/catalog/tests/local_runtime/guarded_merge.rs:91`, only the owner slot obtained from the private socket's SO_PEERPIDFD can authorize forced retirement. At `:114`, list-derived child handles are only polled; an unobserved exit records a cleanup failure without signalling. Numeric child-list acquisition still does not prove ownership, but these handles no longer authorize a signal. The existing crash helper's strong-owner capture and signal path are unchanged.

The new boundary control at `guarded_merge.rs:142` supplies a retained task-owned sibling as the observation-only handle, checks that it remains live, and requires cleanup refusal. Its paired strong-owner control at `:187` requires forced retirement to deliver SIGKILL while retaining the cleanup failure. This is a constructed authority-boundary control, not a kernel PID-reuse reproduction. It exercises the removed behaviour without exposing unrelated processes to signals.

The cache limitation recorded in the prior report is also corrected. `adapters/catalog/tests/local_runtime/settlement_fault.rs:697` polls the held cached pidfd, retains a live handle, removes an exited handle, and refuses unexpected poll events. A replacement capture at `:681` opens a new pidfd and validates the notification ID before storing it. Cancellation drops the new descriptor. A live cached handle cannot retarget on numeric reuse; an exited cached handle no longer suppresses the new notification-validated capture. The control at `:727` proves live retention and exited-handle removal with one actual task-owned process; it does not claim to force PID wrap or reuse.

For the corrected cleanup failure path, `guarded_merge.rs:131` preserves failure on a normal call but returns false when already unwinding. `adapters/catalog/tests/local_runtime/cli_journey.rs:237` then returns from shutdown, allowing `Cli::drop` at `:263` to drain the observational reader and field cleanup to continue. The existing Controller fallback at `settlement_fault.rs:344` retains notification-validated pidfds and services CONTINUE while retiring tracees, then stops and joins its listeners. The correction removes the identified second-panic path; this is not a claim that every possible OS error, poisoned mutex or monitor failure during destruction has been fault-injected.

Inherited execution evidence and limits

Read retained author logs; no reviewer execution was needed and no new red-capable case was added. The author's boundary red is 1 failed, exit 101, before the correction; it records the observation-only sibling receiving SIGKILL. The corrected controls are 2 passed, exit 0, with the sibling still live and cleanup refused. Exact log SHAs:

- `cleanup-negative-red.log`: `9bdbf85aae206744088bc92c432d836d8efc0c89bfbb350cf4440944ff46ac7d`.
- `cleanup-controls-green.log`: `7fcdc5ed6b4a536198297ea60ad8e18363b090116c7827d70532b03930cbfb63`.
- `cleanup-matrix-green.log`: `ceecae237163ae847f49052e559c5db4d75200df9fa4ca81a25686beac1fa10e`; control12 and modes8/9/10, one selected test passed in 53.11 seconds.
- `cleanup-revoked-green.log`: `0141a2c73f9b7dd17ad13a31582a8f594fe0b52477b5def6979fe43c03bc4456`; separate mode11, one selected test passed in 12.62 seconds after the matrix exit was observed.

The retained final ordinary log has 44 result summaries: 334 passed, 0 failed, 21 ignored, including local_runtime 49 passed/21 ignored. Before the two new controls the author count was 332/0/21. Clippy/fmt are inherited author green evidence. The consolidated ten logical obligations/fifteen variants span retained runs; these are not fifteen fresh executions against this final binary. Two controls are additional ordinary tests, not additional journey variants.

This correction leaves shared capture, same-image replay, crash helpers and the fault-None command/shutdown path unchanged. Its affected fault cases and controls were rerun by the author. I found no changed relevant path requiring an additional lifecycle/modes0–7 rerun for this narrow correction. Earlier evidence retains its original input identities and limitations; unchanged production binaries alone do not establish identical test inputs.

No additional source-grounded finding was established within this follow-up. No race stress, PID-reuse reproduction, universal panic-cleanup claim or broader suite execution is inferred from the controls.

```findings
[]
```
