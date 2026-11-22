---
format: aep.planning-md/3
id: review-result:wave-c-connect-begin-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on definition-connect-begin-carries-selection
relations:
- reviews: story:definition-connect-begin-carries-selection
revision: 1
---
unit: story:definition-connect-begin-carries-selection, commit 8c47ef8581 (base 8239f5b6f6)
verdict: green
cases: executed 4→4, red 0
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/units/definition-connect-begin-carries-selection/adv1.md
needs-coordinator: no

1. `git diff --stat`: empty. No files changed.
2. Cases added: none. No red case found. `executed 4→4` comes from the implementor's green.log (definition_connect_question, ignored tests included). I ran no suite, because rule 3 says no run without a case of my own.
3. Suite run: none, for the same reason.
4. Findings (commit 8c47ef8581):
 - F1 contracts/cli/v1alpha1/owner.md:173-175, contract-drift, warning, NEEDS-CHANGE, introduced. The new text says the owner reads a request "only from a caller of its own build". That is false. The owner answers callers of any build, and only the caller compares and refuses (transport.rs:2133-2143, "The caller compares and refuses"). The conclusion still holds: the new CLI's `same_build()` (transport.rs:610) never sends the field to another build, and the owner treats it as optional. Fix: one doc line saying the CLI sends `begin` only to an owner of its own build. Caller: anyone reading the contract. Acceptance holds without the fix.
 - F2 apps/connectors/tests/definition_connect_question.rs:595, judgement, note, CONFIRMED, introduced. The comment says custody "would answer `custody_unavailable`". In this test custody is a real, unlocked keyring, so a Begin that went on reaches the token prompt instead. The implementor's red.log shows it hanging at "GitLab personal access token: ", then the 120 s on_terminal panic. The file's module doc (:12-15) still says custody names a socket that does not exist, which is wrong for this test. Caller: the next reader. Acceptance holds without the fix.
5. Attacked and could not break:
 - Compared value: both sides compute the digest with `Adapter::selection()` from the same recorded entry. The CLI sends the selection the question was asked for, not a fresh load. The owner compares against the `selected()` snapshot it uses for everything after. There is no TOCTOU inside the owner, because later checks compare against the same snapshot (post-Complete :2696, pre-publish :2783).
 - A replacement after the owner's compare ends in `lifecycle_conflict` after Complete, before Validate. Validate runs only on the checked entry's provider (pre-existing). The CLI does prompt in that case, but the credential reaches only the owner.
 - No credential: in the fixed code nothing can be typed, because the refusal comes before `Reply::Capture`. A prompt with no scripted answer hangs and the test panics, so the test does go red on a credential prompt. The implementor's red.log shows this against the pre-fix code.
 - Release build: the hook is `#[cfg(debug_assertions)]`, the same pattern as about 10 existing CONNECTORS_TEST_* hooks. Only `[profile.dev]` and `[profile.test]` turn debug-assertions on.
 - Error code: `RevisionConflict` maps to `publication` / `retry_explicitly` (local.rs), which matches semantics.md:616-618.
 - Other paths: session.rs begin and begin_grant ask no question and send unchanged bytes. The reuse request carries no target selection, but the owner requires the same provider authority, a static entry and `receives_reuse`, so a replaced target cannot receive the copy at another host. The "connect fresh" path after a failed reuse goes through the same `connect()`.
 - Protocol: `skip_serializing_if` keeps the bytes of every other Begin unchanged, and `deny_unknown_fields` plus the caller-side `same_build()` keeps the field away from older owners.
6. Paths written outside the worktree: the adv1.md above. A copy of the catalog provider was made under scratch/adv1/, never used, and deleted.
Coordinator note (not a finding): the implementor's scratch/run.sh sets TMPDIR=~/.cache/ccb, directly under ~/.cache. The directory is gone now.

```findings
[{"file":"contracts/cli/v1alpha1/owner.md","line":174,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"States the owner reads requests only from callers of its own build, but the owner serves any build and only the CLI's same_build() keeps expected_selection from other builds"},
 {"file":"apps/connectors/tests/definition_connect_question.rs","line":595,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Comment says a Begin that went on would answer custody_unavailable, but this test runs a live keyring, so it would reach the token prompt; the module doc's nonexistent-socket claim is also stale for this test"}]
```
