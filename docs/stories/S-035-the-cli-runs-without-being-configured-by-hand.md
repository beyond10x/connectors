---
id: S-035
title: "The CLI runs without being configured by hand"
pillar: Platform
status: in-progress
priority: 1
design:
epic: local-product
areas: [cli, config, protocol]
note: "init/doctor/providers/output and correct exit codes have landed, with the operator surface split into connectors-console; the in-process one-shot and the Connect Session failure reason remain"
---

# The CLI runs without being configured by hand

## Goal

Make `connectors` a program someone can install and use on their own machine, with no deployment,
no Identity and no PostgreSQL — the first of the six stories that turn the local posture from a test
harness for the hosted service into a product. This story is the entry surface: writing a
configuration, finding out why something does not work, and getting results a script can read.

## Why

To call one API through Connectors on a laptop, a person previously had to hand-write a TOML whose
`[owner]` block carries an `authority_snapshot_sha256`. That value cannot be derived from the
documentation, so in practice the file was copied out of `scripts/dev/local-stack.sh` and edited
until the daemon stopped refusing it. Then every subcommand demanded `--config` and `--state-root`
again. The local posture had a complete protocol surface and no way in.

## What landed

- **`connectors init`** writes `~/.config/b10x/connectors.toml` at `0600` with a derived
  `[owner]`, and admits whatever this machine supports (`--integration`, repeatable). It stages the
  file, reads it back through `PersonalConfig::read` — the daemon's own reader, with its ownership
  and permission rules — and only then renames it into place. A configuration that would be refused
  never reaches the destination path, and an existing one is never replaced without `--force`.
  The `authority_snapshot_sha256` is derived from the admitted integration set under a versioned
  domain prefix, computed over the configuration *without* the owner block, because a digest over
  bytes containing itself has no fixed point.
- **`--config` and `--state-root` are optional** on every personal-local command, falling back to
  `default_config_path()` / `default_state_root()`. `serve-hosted --config` stays required: a hosted
  deployment's configuration is installed by whoever operates it, and defaulting to a path in the
  invoking user's home directory would be the wrong file every time.
- **`-o text|compact|json|yaml`**, global. `json` and `yaml` put failures on **stdout** in the
  requested format, so a pipe reads the named refusal instead of an empty stream; `text` and
  `compact` keep them on stderr. `compact` is one record per line.
- **Two defects the envelope was hiding.** Every result was printed as its transport envelope —
  `protocol`, `request_id`, `status` wrapped around the payload — so `-o compact` rendered transport
  metadata instead of records. Worse, a `status: error` envelope was printed as though it were a
  result **and the process exited `0`**: `connectors connection list | jq` reported success for a
  request the daemon had refused by name. Both are fixed in one place, because the step that drops
  the metadata is the step that notices the refusal. A refusal now forwards the Connector's own
  error code and exits non-zero.
- **`connectors doctor`** reports configuration, state root, socket-path budget, daemon liveness and
  credential-store posture, and exits non-zero when something cannot work. Three of its checks exist
  because someone lost time to what they report:
  - the **socket-path budget** is measured against `<state-root>/connect-sessions/<uuid>.sock`, the
    deepest path the daemon binds — not against `connectors.sock`. Sizing the shallow one is what
    let the deeper failure through: the daemon starts, publishes readiness, and only fails later
    when someone tries to hand it a credential, reporting *"connection management is temporarily
    unavailable"* — retriable-sounding, for something that can never work. It cost the concurrent
    Zwirn session a debugging session and this story's first real run.
  - a **socket file with nothing behind it** is what an ungraceful stop leaves, and silence about it
    reads as "the daemon is up".
  - the **credential store is named as unencrypted** while it is a file protected by ownership and
    mode alone. That is a real guarantee against another user and none at all against a copied
    backup. S-036 replaces it with the OS keyring.
- **`init` names what it did not admit.** Its safe default refuses kubeconfig contexts
  authenticated by a credential plugin, because activating one runs that plugin. Every EKS context
  is in that class, so the first version of this command wrote a configuration in which Kubernetes
  was declared and no context could connect — the operator meeting `allow_exec_auth is required` at
  connect time, with no reason to know the switch exists. `init` now reports it and names
  `--allow-exec-auth`. Found by running the command, not by reading it.
- **Three ways to pass caller input** to `operation invoke`: `--input-json` (retained, documented),
  `--input-file <path>`, and `--input -` for stdin. A real payload exceeds the operating system's
  argv limit, at which point an inline argument fails in the shell rather than in this program.

## The architecture fence moved this work, and it was right

`catalog-build/tests/main/architecture_fence.rs::product_cli_is_a_thin_frontend` pins the CLI's
exact dependency list and caps it at 800 production lines. The first cut of this story put `init`,
`doctor`, `providers` and output rendering in the binary and blew both — 2,015 lines and six new
dependencies. The fence's own instruction is to move behaviour behind its owned package, so there is
now **`crates/connectors-console`**: an isolated nested workspace holding the operator-facing
surface, with `connectors-cli` back down to clap definitions, dispatch, and the one process-level
TLS decision only a binary can make.

That is a better shape than the one it replaced, and not only for the fence. The guided `connect`
flows used to `println!` their way through, which meant `connectors connect kubernetes -o json`
silently ignored the format it was given. Moving them into a library forced them to **return their
outcome as data**, and the frontend renders it — so the one command a first-run script most wants to
read now answers in the format it asked for.

## Deviation from the plan, recorded

The plan made `PersonalConfig.owner` an `Option<OwnerConfig>` with a derived fallback. It stays
**required**. `init` generating it achieves the same user-visible outcome — nobody hand-writes a
digest — while an optional field would ripple into `voice()`, `PersonalVoiceConfig`, the hosted
reader, and both external writers of this file (Zwirn's managed placement and the dev stack), for no
gain. A strict, explicit configuration also matches the rest of this crate, which is
`deny_unknown_fields` throughout.

`init` currently admits **Kubernetes only**, because it is the one integration this machine can
declare without asking for a value: Slack needs a workspace id and a token file, Grafana needs an
origin. A flag that silently wrote a placeholder for either would produce a file that parses and
cannot work. The generic catalog-driven integration (S-038) is what makes `init` + `connect <any
provider>` reach the other 60.

## Acceptance

- [x] `connectors init` writes a configuration the daemon reads back, at `0600`, refusing to
      overwrite silently, and refusing to leave an invalid file on disk.
- [x] Every personal-local subcommand runs without `--config` and `--state-root`.
- [x] `-o json` and `-o yaml` emit failures on stdout in that format; `text`/`compact` use stderr.
- [x] A Connector refusal exits non-zero and forwards the Connector's own error code.
- [x] Results render as their payload, not their transport envelope; `-o compact` is one record
      per line.
- [x] `connectors doctor` names a state root too deep to bind a Connect Session endpoint, and exits
      non-zero.
- [x] `operation invoke` accepts input from an argument, a file, or stdin.
- [x] `connectors providers` reports, per catalogued provider, its authority, credential
      requirement, required configuration fields, verify probe and operation count — so coverage is
      a measured fact rather than a claim.
- [ ] A one-shot command composes the runtime in-process when no daemon holds the state lock, so
      `connectors operation search` answers with nothing running. The in-process path must enter at
      the same `BackendRegistry` entry point, so there is exactly one authority path.
- [ ] `ConnectSessionStatus` carries a non-secret failure code. Today `state: Failed` is all a
      caller can see, so `integration-slack` logs its reason with `eprintln!` and a CLI can only
      print "failed". Needs `ConnectSessionTerminal::Failed` to carry a code, which touches every
      caller in `integration-slack` and `integration-jira`.

## Evidence

Measured on 2026-08-20 against a live daemon on this machine:

- `crates/connectors-console`: 30 tests, 0 failed. `crates/connectors-cli`: 4.
  `crates/connectors-config`: 15. `crates/connectors-runtime`: 11. `catalog-build`: 51, which
  includes the architecture fence and the JSON governance gate.
- `connectors providers` reports **61 providers, 978 operations, 45 ready** — read from the compiled
  catalogue this binary embeds, independently reproducing the figures the plan measured from the
  source JSON.
- `connectors connect kubernetes --context <ctx>` against a live EKS cluster: connected, and
  discovered two Prometheus Services behind it.
- `connectors init` → a `0600` file; `connectors serve` → readiness with
  `kubernetes_candidates: 18`; `-o compact connection candidates` → 18 lines, one per context.
- A refused request exits `1` and prints `not_found: no Integration owns this Connection request`;
  under `-o json` the same refusal arrives on stdout as a parseable envelope.
- `doctor` against a 91-byte state root: `socket-path` and `connect-session-path` both `fail`, with
  *"Choose a --state-root at most 48 bytes long"*, exit `1`.

## Superseded by

`story:the-cli-runs-without-being-configured-by-hand` in the AEP planning store, at
`.engineering/planning/story/the-cli-runs-without-being-configured-by-hand.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
