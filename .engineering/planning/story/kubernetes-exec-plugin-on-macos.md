---
format: aep.planning-md/3
id: story:kubernetes-exec-plugin-on-macos
kind: story
status: implemented
title: Kubernetes exec credential plugins run on macOS
summary: 'D370 item 3: exec plugin from a verified owner-private copy'
refs:
- provider: github
  reference: beyond10x/connectors#234
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 2, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "proposed", to: "active", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 3, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "active", to: "implemented", at: "2026-11-03T13:11:13Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: kubernetes-exec-plugin-on-macos

## Outcome

A Kubernetes instance on macOS whose kubeconfig logs in through a pinned exec credential plugin
(such as `aws eks get-token`) gets its token, under the same admission as on Linux.

## Context

D370, item 3; https://github.com/beyond10x/connectors/issues/234. The plugin is verified and executed
through `fexecve` on its descriptor (`adapters/kubernetes/src/exec_credential.rs:361-437`, and
the Linux `Image`). Elsewhere `verify` refuses with `plugin_unavailable` (`:439-445`). macOS has
no `fexecve`; the host already launches programs on macOS from a verified owner-private copy
(`crates/connectors-host/src/local/runtime/artifact/macos.rs`).

## Acceptance

- On macOS the plugin passes the Linux admission checks: a regular file, not a symbolic link as its
  last component, owned by the user or root, not group- or world-writable, no set-ID bits, a matching
  SHA-256. It is then executed from an owner-private copy (0500 file in a 0700 directory) whose
  digest is checked again before exec. The copy is removed after the run.
- A native plugin and a `#!` script plugin both return an `ExecCredential` token. The argument,
  environment and output bounds are the Linux path's.
- A digest mismatch, a group-writable plugin and a set-ID plugin are refused on macOS as on Linux.
- The exec credential tests run on the macOS job of story:provider-programs-run-on-macos.

## Out of Scope

- Plugins that need an interactive terminal.

## Acceptance as decided in wave D370 (D372)

Acceptance line 1 asks for execution from an owner-private copy. Wave D370 decided otherwise (D372,
2026-11-02, `approval-record:connectors-d370-wave` § Decided in the wave): on macOS the pinned plugin
runs from its pinned path, because a copy breaks a bundled CLI that loads files from beside its
executable. Before the run the final component is not a symbolic link, the open descriptor passes the
Linux file checks and the pinned SHA-256, every directory of the resolved parent path is owned by
root or the effective uid and is not group- or world-writable, and right before spawn the path still
names the descriptor's (dev, ino). The orchestrator, 2026-11-02: "your U4 in-place exec decision is
accepted with the documented residual; record it in the store." Residual, documented in CHANGELOG
0.65.0, the reads contract and `docs/local-kubernetes-cli.md`: root or the same user can replace the
file between the check and the run.

Read as amended, line 1 is held by `adapters/kubernetes/src/exec_credential.rs` tests
`a_script_plugin_runs_from_its_own_path` (:1220), `a_native_plugin_runs_from_its_own_path` (:1246),
`digest_mismatch_group_writable_and_set_id_plugins_are_refused` (:1262),
`a_plugin_under_a_writable_directory_is_refused` (:1285) and `a_file_swapped_in_after_the_check_is_refused`
(:1306), which ran on the macOS job of pull request #235 (run 41463903366). Recorded 2026-11-03 by wave
D417.
