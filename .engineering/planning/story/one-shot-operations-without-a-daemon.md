---
format: aep.planning-md/1
id: story:one-shot-operations-without-a-daemon
kind: story
status: draft
title: One-shot operations without a daemon
revision: 1
---
# Story: one-shot operations without a daemon

## Goal
`connectors operation describe|invoke --config <file> --state-root <dir>` is a client to the personal
daemon's control socket (`crates/connectors-cli/src/lib.rs`, `LocalClient::new(state_root.join(
"connectors.sock"))`); with no daemon it fails `connector-unreachable: local Connector transport
failed: No such file or directory`. The runtime that owns the keyring handles, the egress rules, the
description leases and the audit trail is composed only by `connectors serve` (`PersonalRuntime::bind`).
A caller that is itself a short-lived program — a sync round started by a timer, a CI job, an agent
verb such as `brain acquire` — therefore needs a long-running process beside it before it can read
one page, and `doctor`'s "one-shot commands work" reads as if it did not.

## Shape
Either of two, decided in review:
- **Ephemeral runtime.** When the socket is absent and `--config`/`--state-root` are given, the
  `operation` and `connection` verbs compose the personal runtime in-process for the duration of
  the command (same code path as `serve`, no socket published, leases valid within the process),
  then release the state root. Events still need `serve`; `doctor` says so.
- **`connectors serve --until-idle <secs>`.** A verb spawns the daemon when the socket is absent and
  it exits after a quiet period; concurrent callers share it. Fewer code paths, one more process.

In both, credentials never leave the connectors process; a caller still sees only the protocol.
`doctor`'s wording changes to name which verbs need the daemon.

## Acceptance
- With no daemon, `operation describe` and `invoke` with `--config`/`--state-root` succeed against
  a fixture provider; the state root holds no socket afterwards.
- With a daemon running, behaviour is unchanged (the socket is used).
- `event receive` without a daemon is refused naming `connectors serve`.
- `connectors doctor` names the verbs that need the daemon.
