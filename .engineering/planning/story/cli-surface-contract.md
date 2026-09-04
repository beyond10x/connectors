---
format: aep.planning-md/1
id: story:cli-surface-contract
kind: story
status: draft
title: The command tree is generated, and the gate holds it there
relations:
- derived_from: epic:cli-surface
- depends_on: story:connectors-ess-domain
scope:
- confidence: cited
  path: crates/connectors-cli/src/generated
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: cited
  path: docs/design/19-the-cli-surface.md
- confidence: cited
  path: scripts/gate.sh
revision: 5
---
# Story: the command tree is generated, and the gate holds it there

## Defect

`crates/connectors-cli/src/lib.rs` declares 16 top-level commands and 14 subcommands by hand. No
artifact says what the surface is, so nothing catches a command added in the wrong place, a command
dropped, or a command whose target nobody declared. The completions script is already generated from
the parser (`lib.rs:170-179`), which proves the parser can be a source — but the parser itself is
the source of record, and it answers to nothing.

## Shape

- `ess generate synthesize --target clap --out crates/connectors-cli/src/generated/`, committed.
- `crates/connectors-cli/src/lib.rs` gains `pub fn command() -> clap::Command { Cli::command() }`,
  reusing what the `Completions` arm already builds.
- `crates/connectors-cli/tests/cli_surface.rs`, dev-deps `serde_norway` + `serde`
  (`Cargo.toml:132`). `tests/` is excluded from the CLI line fence and `[dev-dependencies]` is not
  fenced (`crates/catalog-build/tests/main/architecture_fence.rs:592-606`, `:559-570`), so this
  costs no fence headroom.
- `scripts/gate.sh --final` gains `ess validate --path ess/system` and the regeneration check.
- `docs/design/19-the-cli-surface.md`: why these groups, the target rule, and that this repository is
  the first consumer of the ESS command-line construct. The series runs to 18.

## What the test asserts

1. The committed generated tree is byte-identical to a fresh `ess generate synthesize`. A stale
   contract fails the gate.
2. Every command in `connectors_cli::command()` is in the specification, and every command in the
   specification is in the tree. Both directions, so neither file can drift alone.
3. `operation`, `connection` and `event` carry `--target`. Until
   `story:explicit-target-never-implicit` lands they are a named allowlist of exactly three
   exceptions, and the test fails when the list is any other size — a countdown that reaches zero
   when that story is done.

## Adoption is incremental

The generated tree lands first as a checked parallel artifact. The hand-written enums are replaced
one group at a time, with the gate green between each. A single cutover of all 16 commands is the
version of this that fails at 2 a.m.

## Acceptance

- `cargo test -p connectors-cli --locked` fails when a command is added to the parser and not the
  specification, and when the committed tree is stale. Both proven by a deliberate edit, reverted.
- `bash scripts/gate.sh --workspace crates/connectors-cli` exits 0.
- `connectors setup completions fish` is generated rather than hand-maintained.

## Depends on

`story:connectors-ess-domain`.
