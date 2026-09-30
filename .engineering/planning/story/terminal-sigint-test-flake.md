---
format: aep.planning-md/3
id: story:terminal-sigint-test-flake
kind: story
status: draft
title: The controlling-terminal SIGINT test holds under load
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed

2026-09-30, full `cargo test -p connectors-host --lib` loop on wave0930b-flake, run 5 of 10 (237 s, another session's
host test binary running at the same time): `local::protected::tests::controlling_terminal_hides_input_and_restores_echo_after_sigint`
failed with `fixture assertion failed (interrupt) ... protected/tests.rs:84:9: assertion failed: matches!(result,
Err(Error { code: Code::Interrupted, .. }))`. It uses no metadata; it has a deadline.

## Acceptance

- Reproduce under load, name the timing that lets the interrupt miss, and fix the test or the code so it passes 20 of
  20 full --lib runs.
