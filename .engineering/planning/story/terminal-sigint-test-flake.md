---
format: aep.planning-md/3
id: story:terminal-sigint-test-flake
kind: story
status: implemented
title: The controlling-terminal SIGINT test holds under load
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/protected.rs
- confidence: cited
  path: crates/connectors-host/src/local/protected/tests.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:06:15Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-01T11:06:15Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-01T14:52:45Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Observed

2026-09-30, full `cargo test -p connectors-host --lib` loop on wave0930b-flake, run 5 of 10 (237 s, another session's
host test binary running at the same time): `local::protected::tests::controlling_terminal_hides_input_and_restores_echo_after_sigint`
failed with `fixture assertion failed (interrupt) ... protected/tests.rs:84:9: assertion failed: matches!(result,
Err(Error { code: Code::Interrupted, .. }))`. It uses no metadata; it has a deadline.

## Acceptance

- Reproduce under load, name the timing that lets the interrupt miss, and fix the test or the code so it passes 20 of
  20 full --lib runs.
