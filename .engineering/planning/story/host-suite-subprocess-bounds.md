---
format: aep.planning-md/2
id: story:host-suite-subprocess-bounds
kind: story
status: active
title: Subprocess and child-process test bounds are set by the test
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/approval_policy/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/keyring/custody.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/src/local/protected/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/process/write_tests.rs
revision: 8
---
## Acceptance

`cargo test -p connectors-host --lib` passes three runs on a host at load average 25 or more with
no test failing on `Timeout`, `Unavailable`, `Conflict` or a fixture assertion where it passes
idle, and every remaining wall-clock bound in these tests is one the test sets:

- `drop_eof_and_original_deadline_destroy_pending_without_a_write` and
  `native_outcomes_and_lost_replies_never_repeat_a_send` (`process/write_tests.rs:386-394`) pass a
  2 s wall-clock deadline to a separate adapter process.
- Child-process contention tests (`approval_policy/tests.rs:285` and similar in `audit`,
  `approvals`, `approval_keys`, `mutations`) and `keyring/custody.rs:172,281,304` keep 2 s lock
  bounds the per-thread override from `story:host-suite-load-sensitivity` cannot reach.
- `protected::tests::controlling_terminal_hides_input_and_restores_echo_after_sigint`
  (`protected/tests.rs:188`) failed once under load; cause not established.

Production bounds stay as they are.

## Source

The remaining items recorded in `story:host-suite-load-sensitivity` at release 0.13.1.
