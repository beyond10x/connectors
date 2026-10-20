---
format: aep.planning-md/3
id: review-result:wave-20261019-post0540-u4-adversary-1
kind: review-result
status: active
title: 'U4 adversary pass 1: ci-timing-flakes-20261018'
relations:
- reviews: story:ci-timing-flakes-20261018
revision: 1
---
unit: story:ci-timing-flakes-20261018, commit b9731c6ef plus one new untracked test file in the worktree
verdict: red
cases: executed 159→160, red 1
origin: introduced 2, pre-existing 0, undecided 1
wrote-outside-worktree: ~/.cache/connectors-wave-20261019b/u4/adversary-1/{case1-first.log, case1.log, suite-kubernetes.log, probe/listen_probe.rs, probe/listen_probe, probe/state/}
needs-coordinator: yes: (1) decide whether the "an operator can tell" acceptance line is met by changing the code or by rewording it; (2) run the macOS "host and CLI" job on this branch, because nothing has run `listen` on macOS yet

**1. Diff.** `git diff --stat` is empty: no tracked file changed. `git status --short` shows one new file: `?? adapters/kubernetes/tests/exec_plugin_unavailable_operator_view.rs`. No implementation file was touched.

**2. Case added (written first, then run alone).** `an_operator_tells_a_busy_exec_plugin_from_a_missing_one` starts the real `connectors-kubernetes` child through `connectors_host::local::runtime::Child`, the way the host does, and calls `validate` three times: while a separate `sh` process holds the plugin open for writing (busy); after that writer is released (control: `Err(Unavailable)`, the plugin ran and exited 1); after the plugin file is removed (missing). It asserts the host gets a different answer for busy and for missing. Red now (`case1.log`):
```
assertion `left != right` failed: the host receives the same answer for a busy plugin and a missing one
  left: "Err(InvalidConfiguration)"
 right: "Err(InvalidConfiguration)"
test result: FAILED. 0 passed; 1 failed
```

**3. Suite run.** `TMPDIR=…/tmp nice -n 19 cargo test --locked -j 8 -p connectors-kubernetes --no-fail-fast` → EXIT=101; every binary passed except `exec_plugin_unavailable_operator_view: 0 passed; 1 failed`. 160 tests executed.

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| adapters/kubernetes/src/local.rs:417 | NEEDS-CHANGE | undecided | `exec_failure` maps `PluginUnavailable(_)` to `Failure::InvalidConfiguration`; `Failure` (runtime.rs:42) has no fields, so the error kind stops there. | Every exec-plugin validate or operation. The acceptance line folded in from kubernetes-exec-fixture-busy says "so an operator can tell a missing plugin from a busy one". Not run at the base commit. |
| crates/connectors-host/src/local/owner/transport.rs:1338 | CONFIRMED | introduced | The macOS owner now depends on hard-linking a Unix socket; nothing has run that on macOS. | Every macOS owner start and relisten. If APFS refuses `link` on a socket, no macOS owner can start. |
| contracts/cli/v1alpha1/owner.md:291 | CONFIRMED | introduced | The contract says the macOS socket "is bound and reached by its full path"; it is now bound at `owner.bind` and linked into place. | Readers of the contract. Documentation only. |

**5. Attacked and could not break** (Linux replica of `listen`, `probe/listen_probe.rs`, through `/proc/self/fd/<fd>/owner.sock`): stale `owner.bind` removed and listen succeeds; umask 000, 022, 077, 777 give a 600 socket with link count 1 and no staging file left; existing `owner.sock` gives EEXIST, keeps its inode, removes the staging name; after replacement a new connect reaches the new listener; two owners binding at once can misroute, but only the lifetime-lock holder calls `listen` (`serve` uses `flock` with `LOCK_NB`, `retire` keeps the lock); tether and CLI checks compare (device, inode), which a hard link keeps; `PluginUnavailable` `code()` unchanged, `Refusal` has no `Display` or serde derive; the deadline-test retry loop does not hide an early deadline; OAuth `delivered()` panics after 10 s and cannot hang.

```findings
[
  {"file": "adapters/kubernetes/src/local.rs", "line": 417, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "undecided", "message": "an_operator_tells_a_busy_exec_plugin_from_a_missing_one is red: the host gets Err(InvalidConfiguration) for both a busy and a missing plugin because exec_failure drops the error kind, so the story's 'an operator can tell' acceptance line is not met"},
  {"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 1338, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "no test has run listen's hard link of a Unix socket on macOS, the platform whose flake it fixes; a_connection_closed_by_a_retiring_owner_reaches_the_next_owner must pass on the macOS job before merge"},
  {"file": "contracts/cli/v1alpha1/owner.md", "line": 291, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the contract says the macOS owner socket is bound at its full path, but it is now bound at owner.bind and hard-linked to owner.sock (no test)"}
]
```
