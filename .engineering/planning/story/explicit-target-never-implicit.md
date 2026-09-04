---
format: aep.planning-md/1
id: story:explicit-target-never-implicit
kind: story
status: draft
title: A command names its target; nothing infers it from a stored login
revision: 1
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
- A target is always explicit: `--target local|hosted` on the three verbs, or a named context
  (`connectors context use <name>`, stored per machine, printed by `doctor` and in every result
  envelope as `target:`). No target and both a login and a local state root present → refused
  naming both, never a guess.
- The default, when nothing is declared, is **local**. A hosted target is chosen, not fallen into.
- `--config`/`--state-root` stop implying the target; they only say which local configuration and
  which socket, and are refused together with `--target hosted`.
- `connectors login` prints which contexts now exist and that no command changed target.
- `doctor` gains a `target` check: which target the next `operation` would reach and why.

## Acceptance
- With a login present and no flags, `operation search` reaches the local socket (or refuses when
  there is none) and never a remote; `strace -e trace=connect` shows no network `connect()`.
- `--target hosted` reaches the login's deployment; without a login it is refused by name.
- Every result envelope names the target it came from.
- The old implicit route is gone: a test asserts the guard no longer exists.
