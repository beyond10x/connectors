---
format: aep.planning-md/3
id: story:kubernetes-exec-fixture-busy
kind: story
status: archived
title: The Kubernetes exec plugin fixture test holds under a full parallel run
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-10-19T18:40:43Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"artifact":1}}}
---
## Observed
`exec_plugin_native_executable_answers_through_its_descriptor`
(`adapters/kubernetes/tests/exec_credential.rs:456`) failed once in a full
`cargo test -p connectors-kubernetes --no-fail-fast` run on 2026-10-16: the exec credential
plugin answered `Err(PluginUnavailable)` where `Ok("fixture-exec-plugin-token")` was expected.
Run alone it passed (`1 passed; 19 filtered out`). The test writes a native fixture executable
and runs it while sibling tests in the same binary spawn processes, the pattern that gives
`ETXTBSY` (text file busy) on Linux when a forked child still holds the write descriptor.

Reproduction: `cargo test --locked -p connectors-kubernetes --test exec_credential` with the
default test threads; not reproduced alone.

## Acceptance
- The cause is shown (the plugin's spawn error kind behind `PluginUnavailable`, logged by the
  test on failure) and recorded here.
- If it is `ETXTBSY`: the fixture executable is written, closed and made executable before any
  sibling can fork with its descriptor open (written once per binary, or by a helper that
  closes it before spawning), or the spawn retries `ETXTBSY` a bounded number of times.
- `PluginUnavailable` from an exec plugin carries the spawn error kind, so an operator can tell
  a missing plugin from a busy one.
- The test passes 50 consecutive full runs of the `exec_credential` binary.
