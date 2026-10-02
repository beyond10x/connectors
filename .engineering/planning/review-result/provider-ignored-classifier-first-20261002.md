---
format: aep.planning-md/3
id: review-result:provider-ignored-classifier-first-20261002
kind: review-result
status: active
title: 'Provider ignored-runner review: kubeconfig prerequisite gap'
relations:
- reviews: specification:wave-20261002d-provider-acceptance
revision: 1
---
unit: ignored-runner provider classifications — cb26c-plan working tree at 2cda52c70948b9f78e1734aa546dd4beb8668040
verdict: NEEDS-CHANGE (judgment; executable probe pending)
cases: executed 0→0, red 0; no tests or builds run in this read-only pass
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: one tool-managed lease metadata location
needs-coordinator: grant/apply the proposed test-only probe after live windows; no confirmed red is claimed

## 1. Diff and source identity

Observed `git diff --stat`:

```text
 crates/connectors-build/src/ignored.rs | 74 ++++++++++++++++++++++++++++++++--
 1 file changed, 71 insertions(+), 3 deletions(-)
```

This is the coordinator's pre-existing uncommitted implementation diff, the subject of the review. This reviewer made no tracked changes, added no tests to source, and ran no AEP command. Source SHA-256 before and after inspection: `474506ea79a3e27213dfd1956ee1943fde7b98cc51c72a954d6c47e518ddc5d0`. The bounded review covers only this diff, not an endorsement of the whole runner or the authored PostgreSQL unit.

## 2. Proposed case; execution pending

[kubeconfig-mode-probe.patch](kubeconfig-mode-probe.patch) proposes one test-only addition to the existing module: `ignored::tests::adversary_nonprivate_kubeconfig_is_missing_before_dispatch`. It creates a regular mode-0644 file, re-executes only itself with the kubeconfig path in its environment, and asserts that the actual `prerequisites` function reports KUBECONFIG missing. The subprocess isolates environment changes; it contains no provider call or new ignored helper entry. `git apply --check` succeeded. The patch is **unapplied and unexecuted** because Kubernetes and then catalog hold exclusive live windows.

There is no red output to quote and no claim of a confirmed failing test. The expected failure is a source-grounded prediction awaiting that probe.

## 3. Suite execution

None. No Cargo, rustc, compiled test executable, provider command or live call was run. No existing suite count was assumed from another revision. Existing runner tests were read, including explicit-family default, helper exclusion, unknown inventory blocking and exact-one-case execution checks.

## 4. Judgment finding

| File:line | Verdict | Origin | Severity | Finding |
| --- | --- | --- | --- | --- |
| crates/connectors-build/src/ignored.rs:579 | NEEDS-CHANGE | introduced | warning | The new CONNECTORS_K8S_KUBECONFIG prerequisite accepts world-readable files that the selected RealSandbox tests reject, so inventory can report them runnable instead of missing-prerequisite. |

What was inspected: the new KUBECONFIG member joins a loop whose predicate is only `Path::is_file()` (lines 579–583). In the Kubernetes worker source, `adapters/kubernetes/tests/local_runtime/cli_journey.rs:1135–1141`, `RealSandbox::new` requires a regular file **and** `permissions().mode() & 0o077 == 0`. The fixture checks this before its provider work. Root's new four-case classification explicitly lists KUBECONFIG, so it reaches this predicate.

Caller reachability: the documented explicit live-family inventory/execution path with a regular 0644 kubeconfig and other prerequisites available. The runner's current source predicts no missing kubeconfig reason; the selected fixture's source predicts refusal inside the test. This remains fail-closed and does not create false green provider acceptance, hence warning rather than blocker. Executable confirmation is pending.

The old CA/TOKEN entries share the existence-only check, but they predate this diff and are not counted as introduced findings here. A narrow correction can handle only KUBECONFIG. The fixture does not currently check UID equality, symlink identity or equality between the kubeconfig endpoint and CONNECTORS_K8S_SANDBOX; do not describe a mode-private regular-file check as proving those properties.

## 5. Reviewed boundaries without another finding

- Every added PostgreSQL and Kubernetes exact package/target/test name matches the owning worker source; all are Live. Native PostgreSQL future-drop cancellation correctly omits CLI/custody prerequisites.
- Catalog's eight new disposable journeys, one expiry Timing case and exact prepared-attempt helper match their owning modules. The helper has no family, is excluded from unknown counts and never selected for top-level execution.
- No broad prefix or suffix classification was introduced. Unknown names remain unknown and block all execution; credentials alone do not select Live or Timing.
- Docker PATH validation and exact `/usr/bin/kubectl` validation match how those fixtures launch their tools. The environment inherited by test execution carries the provider selections; built CLI/provider paths remain separately supplied by the existing Cargo discovery path.
- Existing list handling uses compiled libtest `--ignored --list --format terse`, exact package/target/name identity, duplicate rejection and exact-one-result enforcement. This pass did not regenerate or execute that inventory.
- Existence/tool checks do not prove live connectivity, PostgreSQL version/container contents or catalog kernel capabilities. Those remain fixture assertions; this pass makes no broader prerequisite-completeness claim.

## 6. Writes and limits

Only the assigned `.local/provider-wave-briefs/classifier-review` scratch contains review output and the unapplied probe. Outside this tree, `worktree hook session-start/heartbeat/session-end` writes tool-managed lease metadata under `$HOME/.local/state/worktree`. No compiler cache, fixture, implementation, planning store or other worker source was written. The coordinator owns any test execution, correction, integration and final gate.

```findings
- file: crates/connectors-build/src/ignored.rs
  line: 579
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The new CONNECTORS_K8S_KUBECONFIG prerequisite accepts world-readable files that the selected RealSandbox tests reject, so inventory can report them runnable instead of missing-prerequisite.
```

Publication redaction: this copy replaces only the literal local home prefix the private home path with `$HOME`. The original raw report remains unchanged, SHA-256 `abb05666526c8229718de50f8a9d46a01abc7fbe87ddec8951540c4ab22283d5`. No factual text or finding has been revised.
