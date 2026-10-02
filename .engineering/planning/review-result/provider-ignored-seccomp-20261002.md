---
format: aep.planning-md/3
id: review-result:provider-ignored-seccomp-20261002
kind: review-result
status: active
title: Ignored runner seccomp prerequisite follow-up
relations:
- reviews: specification:wave-20261002d-provider-acceptance
revision: 1
---
unit: root ignored-runner seccomp/helper additions — ignored.rs eb5dac2560acbdec85b6033f0811770e8974930340c8a3f2a01ba68761183318
verdict: nothing found in bounded read-only comparison
cases: prior 12 passed inherited from before these additions; 0 executions in this pass
origin: introduced 0 / pre-existing 0 / undecided 0 findings
wrote-outside-worktree: own worktree lease registry only
needs-coordinator: run focused tests and compiled inventory after the active diagnostic window

```text
$ git --no-pager diff --stat -- crates/connectors-build/src/ignored.rs docs/development.md
 crates/connectors-build/src/ignored.rs | 150 ++++++++++++++++++++++++++++++++-
 docs/development.md                    |   8 ++
 2 files changed, 154 insertions(+), 4 deletions(-)
reviewer source delta: 0
```

This is the inherited coordinator diff, including previously reviewed provider classifications and kubeconfig correction. This pass covers only the later second-helper exclusion, two fault-case prerequisites, nonmutating seccomp ABI query and new development paragraph. No test, build, compiled inventory, live call, filter installation or AEP mutation was performed. No approval or independence claim is made.

No concrete discrepancy was found:

- `crates/connectors-build/src/ignored.rs:160` excludes the exact `cli_journey::guarded_merge::owner_replay::catalog_same_image_owner_fixture` name for package `connectors-catalog-provider`, target `local_runtime`. This matches the module path and function at worker `adapters/catalog/tests/local_runtime/owner_replay.rs:149`. It remains a helper, with no selected/executed journey count. Unknown-name handling remains unchanged.
- The exact two fault journey names at `ignored.rs:186` match worker `guarded_merge.rs:289` and `:300`. They retain Disposable classification with custody/CLI and add the seccomp prerequisite; other catalog cases do not acquire it.
- `ignored.rs:604` checks Linux x86_64, an exact whitespace-delimited `user_notif` action and successful GET_NOTIF_SIZES returning `[80,24,64]`. This matches the checks in worker `settlement_fault.rs:78`: its C-layout Notification/Response/Data structures are 80/24/64 bytes on that admitted ABI. The three-u16 output buffer matches the kernel size-query structure. The query installs no filter and does not set NO_NEW_PRIVS.
- Capability absence becomes a missing prerequisite only for entries naming it (`ignored.rs:523`). Actual listener-installation policy and owned-process fd inspection remain runtime fixture checks. `docs/development.md:173` explicitly distinguishes those from the runner's preliminary query and calls a runtime refusal a test failure. This pass does not certify the still-active fault implementation or imply that an available ABI proves settlement injection works.

Input hashes inspected:

```text
eb5dac2560acbdec85b6033f0811770e8974930340c8a3f2a01ba68761183318  crates/connectors-build/src/ignored.rs
5bb22d5f3af6969d7b7b249fa7de7cac47e7e986a04237aa1dbb2182f13e3a98  docs/development.md
90399d2e72c17761c5e614a4ccfda6b61fb929f0085aa0ba987d0d94077848f9  worker adapters/catalog/tests/local_runtime/settlement_fault.rs
```

The worker file is active; its hash identifies only the capability-check comparison read in this pass. Final inventory must come from the final compiled test targets. Prior 12 passing classifier cases predate these additions and are not green evidence for the new code.

Only this assigned report was authored. Outside-worktree writes are the worktree CLI's own lease updates in `$HOME/.local/state/worktree/registry.sqlite3` and SQLite-managed transaction files if used. Own `codex-classifier-seccomp-review` lease is released with handoff.

```findings
[]
```
