---
format: aep.planning-md/3
id: story:kubernetes-exec-plugin-on-macos
kind: story
status: implemented
title: Kubernetes exec credential plugins run on macOS
summary: 'D370 item 3: exec plugin from a verified owner-private copy'
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 2, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "proposed", to: "active", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 3, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "active", to: "implemented", at: "2026-11-02T15:48:08Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"verification":1}}}
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
