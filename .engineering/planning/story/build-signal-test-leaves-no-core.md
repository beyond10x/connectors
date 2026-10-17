---
format: aep.planning-md/3
id: story:build-signal-test-leaves-no-core
kind: story
status: draft
title: The gate's signalled-command test leaves no core file
revision: 1
---
## Problem

`connectors-build`'s test at `crates/connectors-build/src/gate.rs:679` runs `/bin/sh -c "kill -<signal> $$"` to prove the gate reports a signalled command. With the host's core pattern writing to the working directory, a test run leaves `core.<pid>` files (two on 2026-10-17, from `kill -6` and `kill -11`) in `crates/connectors-build/`, untracked and owner-only.

## Acceptance

- The signalled-command test runs its child with a zero core-file limit (`RLIMIT_CORE` 0 for that child), and a test run of `cargo test -p connectors-build` leaves no `core*` file in the crate directory.
