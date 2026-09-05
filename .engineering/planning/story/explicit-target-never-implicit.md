---
format: aep.planning-md/1
id: story:explicit-target-never-implicit
kind: story
status: active
title: A command names its target; nothing infers it from a stored login
tags:
- ready
- wave-cli
- wave-platform
relations:
- derived_from: epic:cli-surface
scope:
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface_drift.rs
- confidence: inferred
  path: crates/connectors-console/src/doctor.rs
- confidence: inferred
  path: crates/connectors-console/src/output.rs
- confidence: cited
  path: docs/design/19-the-cli-surface.md
revision: 16
---
# Story: a command names its target; nothing infers it from a stored login

## Defect
`connectors operation …`, `connectors connection …` and `connectors event …` decide where they go
by the absence of two flags: when neither `--config` nor `--state-root` is given and
`~/.local/state/b10x/connectors/identity-sessions.json` holds a login, the request goes to that
hosted deployment (`crates/connectors-cli/src/lib.rs`, the guard
`if config_path.is_none() && state_root.is_none() { AuthenticatedHostedClient::active() … }` at
the three call sites, ~L729, ~L789, ~L871). Otherwise it goes to the personal daemon's socket. The
same command line therefore reaches two different systems depending on a file the operator did
not name, and `connectors login` run once changes the target of every later `operation invoke`
on the machine — including the payloads a script passes as input. Measured 2026-09-04 with
`strace -e trace=connect`: `operation search --query slack` with no flags connected to
`devcenter.dev.babelforce.com:443`; with `--config`/`--state-root` to the local socket; the daemon
was never consulted on the first route. No other verb has this fallback: `mcp`, `admin`,
`serve-hosted`, `login`, `logout` are hosted by name; `connect`, `auth`, `providers`, `doctor` are
local by construction. The pattern exists in connectors only; `zwirn` and the platform agent use
the hosted client explicitly.

## Shape
- A target is always explicit: `--target local|hosted` on the three verbs, reported
  in every dual-target result envelope as `target:`. Named contexts are deferred; an omitted target
  means local regardless of stored login, as the acceptance below requires.
- The default, when nothing is declared, is **local**. A hosted target is chosen, not fallen into.
- `--config`/`--state-root` stop implying the target; they only say which local configuration and
  which socket, and are refused together with `--target hosted`.
- `connectors login` prints the hosted deployment and that no command changed the default local target.
- `doctor` gains a `target` check: which target the next `operation` would reach and why.

## Acceptance
- With a login present and no flags, `operation search` reaches the local socket (or refuses when
  there is none) and never a remote; `strace -e trace=connect` shows no network `connect()`.
- `--target hosted` reaches the login's deployment; without a login it is refused by name.
- Every result envelope names the target it came from.
- The old implicit route is gone: a test asserts the guard no longer exists.

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-platform: 5 of 10; wave-cli: 1 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `crates/connectors-cli/src/lib.rs` — cited.
- `crates/connectors-cli/tests/cli_surface.rs` — cited.
- `crates/connectors-cli/tests/cli_surface_drift.rs` — cited.
- `crates/connectors-console/src/doctor.rs` — inferred.
- `crates/connectors-console/src/output.rs` — inferred.
- `docs/design/19-the-cli-surface.md` — cited.

High confidence. Parser/routing and CLI surface exception tests are cited. Resolve contradictory Shape wording in favor of Acceptance and existing ESS Target: omitted target always means local, even with login present. No named contexts in this unit. Dual-target connection/event/operation envelopes, including errors, report the chosen target. Hosted with local-only flags is refused; missing hosted login is named.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 2 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.
