---
format: aep.planning-md/3
id: review-result:wave-20261106-u1-adversary
kind: review-result
status: active
title: Wave 2026-11-06 U1 adversary pass 1
relations:
- reviews: story:provider-programs-run-on-macos
revision: 1
---
unit: U1 adversary (conn-w1106-u1 at e77320412, diff da0371c72..e77320412)
verdict: green
cases: executed 7→7, red 0
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: ~/.cache/connectors-wave-20261106/u1/adv/linux-journey.log, ~/.cache/connectors-wave-20261106/u1/adv/local-runtime.log, ~/.cache/connectors-wave-20261106/u1/adv/t/ (empty)
needs-coordinator: no

Diff --stat: empty. No test file added, no file changed in the worktree, no commits.

Cases added: none. The macOS code compiles only for macOS and cannot run here, so no red case could be written for it on this host. The Linux side passed every attack below.

Runs (after the review, through cargo.sh):
- Linux journey, --ignored --exact, a short TMPDIR and the built CLI, against the real qualified gnome-keyring-daemon on this host, covering the new shared setup check checks in configure: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 20.87s, exit 0.
- --test local_runtime: test result: ok. 6 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out, exit 0. Linux behaviour unchanged.
- target/ 2.3G. Cargo time under 1 minute; wall time about 25 minutes.

Attacked and not broken:
- Same assertions on macOS as on Linux: one journey() covers the wrong-token refusal, routes, uids, complete, provenance and revalidation; macOS restarts the owner, not custody, with a comment saying why.
- Passing without running: Cli::new panics when CONNECTORS_TEST_CLI is unset; setup check must report macos_keychain; the test needs removed >= 1 (:513).
- Cleanup deleting an item it did not write: it keeps only accounts carrying this store's fresh authority UUID, and each delete removes one exact item_name.
- Workflow step: fails if the test is renamed or filtered out (macos-host.yml:113, plus pipefail). The journey also runs in "Test the programs" (time only).
- Keychain without HOME: the keychain path comes from getpwuid_r, not HOME (custody keychain.rs:213); the owner's env_clear is not a defect.
- The connectors-build ignored platform-gated scan still holds on Linux.
- Developer Mac: plain cargo test -p connectors-grafana now fails without CONNECTORS_TEST_CLI, as acceptance 2 asks.

Notes (INFEASIBLE here: they need macOS, and nothing on this host reaches them):
1. cli_journey.rs:263: the cleanup search runs with user interaction allowed; on a locked keychain, after setup init, it may show an unlock prompt.
2. cli_journey.rs:303: the test binary deletes items that connectors created, and only the decrypt ACLs are widened; if deletion needs the creator's authorization, the cleanup fails loudly and leaves items. The first macOS CI run decides this.
3. cli_journey.rs:377: if the owner shutdown fails while the test is unwinding, the process aborts and LoginKeychain::drop (:326) never runs.
4. cli_journey.rs:256: every authority_at error is read as "nothing written". Fix, not applied: return empty only when the state directory does not exist.

```findings
[{"file":"adapters/grafana/tests/local_runtime/cli_journey.rs","line":263,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"U1-ADV-1: the cleanup keychain search runs with user interaction allowed, unlike every custody call, so a locked login keychain on a developer Mac may prompt on the failure path despite the doc comment; not measurable without macOS"},
{"file":"adapters/grafana/tests/local_runtime/cli_journey.rs","line":303,"category":"acceptance","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"U1-ADV-2: the test binary deletes items the connectors binary created while any_application_access widens only decrypt ACLs; if deletion needs the creator's authorization the cleanup fails loudly and leaves items, decided only by the first macOS CI run"},
{"file":"adapters/grafana/tests/local_runtime/cli_journey.rs","line":377,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"U1-ADV-3: a failed owner shutdown in Cli::drop during unwinding panics twice and aborts, so LoginKeychain::drop never removes the written login keychain items"},
{"file":"adapters/grafana/tests/local_runtime/cli_journey.rs","line":256,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"U1-ADV-4: any Metadata::authority_at error is read as nothing written, so on the failure path cleanup reports removing 0 items while items remain"}]
```

(Coordinator: home-directory paths in the header shortened to ~ for the repository's personal-path gate; nothing else changed.)
