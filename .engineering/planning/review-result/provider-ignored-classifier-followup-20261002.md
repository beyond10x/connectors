---
format: aep.planning-md/3
id: review-result:provider-ignored-classifier-followup-20261002
kind: review-result
status: active
title: 'Provider ignored-runner follow-up: reproduced gap fixed'
relations:
- reviews: specification:wave-20261002d-provider-acceptance
revision: 1
---
unit: ignored-runner provider classifications — cb26c-plan ignored.rs SHA d68e4a48a48e80e33aa05200a737da757c48e4c323936af8b751546acd80bc8c
verdict: prior finding resolved; nothing further found in bounded follow-up
cases: coordinator retained exact probe 1 failed → 1 passed; final ignored::tests:: 12 passed; this follow-up executed 0
origin: introduced 1 resolved / pre-existing 0 new findings / undecided 0
wrote-outside-worktree: own worktree lease registry only
needs-coordinator: final integrated inventory and gate remain coordinator-owned

```text
$ git --no-pager diff --stat -- crates/connectors-build/src/ignored.rs
 crates/connectors-build/src/ignored.rs | 117 +++++++++++++++++++++++++++++++--
 1 file changed, 113 insertions(+), 4 deletions(-)
```

This is the coordinator's diff. This follow-up changed no source or tests and ran no builds, tests or provider calls. It inspected the current diff, the real fixture preflight and the retained probe/suite logs; no new adversarial cases were added.

The earlier NEEDS-CHANGE judgement is resolved at `crates/connectors-build/src/ignored.rs:579`: a kubeconfig must now be a regular file with no group/other permission bits. This matches the new real fixture's `RealSandbox::new` check at `adapters/kubernetes/tests/local_runtime/cli_journey.rs:1135`. CA/TOKEN behavior is unchanged. Neither this fix nor the fixture asserts current UID ownership, symlink rejection or kubeconfig/API endpoint equality; no such claim follows from this result.

The coordinator executed the previously authored isolated case before fixing the implementation. The retained red log contains this actual assertion and summary:

```text
thread 'ignored::tests::adversary_nonprivate_kubeconfig_is_missing_before_dispatch' (1640252) panicked at crates/connectors-build/src/ignored.rs:935:13:
a mode-0644 kubeconfig cannot satisfy the owner-private fixture prerequisite
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 58 filtered out; finished in 0.01s
```

`probe-red.exit` is 101. After the narrow implementation change, `probe-green.log` reports one passed, zero failed, with `probe-green.exit` 0. Other filtered test binaries report zero selections; they add no executed cases.

The final source strengthens the same case with separate re-executed subprocesses for both mode 0644 (must be missing) and 0600 (must be admitted). This avoids mutating the parent test process environment and prevents an always-missing implementation from passing. `runner-suite.log` includes this strengthened case among 12 passed, zero failed, zero ignored, 47 filtered out; `runner-suite.exit` is 0. These are coordinator executions, not fresh executions by this follow-up. The final filtered suite is not a full package or workspace gate.

```text
f0b029731db94ae870554326b90b3629313be81fdeebba2212df2744e6fe9d84  probe-red.log
46f63d6d323a89b3863ae1de0778775e17e760efe2675469380d176123045813  probe-green.log
bc0a696347a48f2af198a6bc85cce3aa0822ba9f8130a88cb1037434a188ebed  runner-suite.log
d68e4a48a48e80e33aa05200a737da757c48e4c323936af8b751546acd80bc8c  ../../../crates/connectors-build/src/ignored.rs
```

The exact-name mappings retain live classification for PostgreSQL and Kubernetes, custody/CLI independence for native PostgreSQL cancellation, the catalog timing class and helper exclusion. Unknown-name refusal was not broadened. The newly listed catalog adversary case remains an ordinary disposable selected case, not a helper. No additional concrete finding was established.

Outside-worktree writes are limited to lifecycle CLI updates for this review's own `codex-classifier-review-pg` lease in `$HOME/.local/state/worktree/registry.sqlite3` and any SQLite-managed transaction files. The lease is released on handoff. The report is public-safe as authored; `$HOME` denotes the private home prefix. No approval or independence claim is made.

```findings
[]
```
