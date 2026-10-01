---
format: aep.planning-md/3
id: story:socket-path-limit-in-long-checkouts
kind: story
status: active
title: 'Gate fails from a checkout whose path is long: owner socket exceeds SUN_LEN'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/tests/absent_operation.rs
- confidence: cited
  path: apps/connectors/tests/failed_connect.rs
- confidence: cited
  path: apps/connectors/tests/failed_connect_adversary.rs
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: apps/connectors/tests/owner_build_security.rs
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: inferred
  path: crates/connectors-host/src/local/keyring/custody/tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:06:17Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-01T11:06:17Z", actor: "human:timo", revision: 6}
---
## Defect

`connectors-build gate --msrv` sets `TMPDIR` to `<checkout>/.local/tmp/gate-XXXXXX`
(`crates/connectors-build/src/gate.rs:7-16`). Tests that bind or connect the owner socket by its
absolute path (`<tempdir>/state/owner.sock`) then fail with `path must be shorter than SUN_LEN` when the
checkout path is long. Observed 2026-09-30 in the managed worktree
`~/.local/state/worktree/trees/b10x/connectors/rel-0-20-0` (65-byte root, 116-byte socket path, limit 108):
`absent_operation::the_owner_invoke_request_answers_not_found_before_the_grant`, 4 tests in
`failed_connect.rs` and at least 7 more (output was truncated; `failed_connect_adversary.rs` binds the same way) failed. The same commit passed from
`~/beyond10x/connectors`.

The host already avoids this: it binds and connects through `/proc/self/fd/<state dir fd>/owner.sock`
(`crates/connectors-host/src/local/owner/transport.rs:595-600`).

Direct socket paths in tests: `apps/connectors/tests/absent_operation.rs:365`,
`failed_connect.rs:118,264`, `failed_connect_adversary.rs:98`, `owner_build_security.rs:186`,
`local_cli.rs:543,555,612,648,697,720,775,818,930,1025`.

## Acceptance

- The full gate passes from a managed worktree under `~/.local/state/worktree/trees/`.
- Tests reach the owner socket through a directory descriptor, as the host does, or the gate's
  temporary root is short enough for every socket path it creates; say which.
