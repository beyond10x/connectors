---
format: aep.planning-md/1
id: epic:cli-surface
kind: epic
status: draft
title: The CLI surface is specified, not assembled
revision: 1
---
# Epic: the CLI surface is specified, not assembled

## Outcome

The `connectors` command tree is a projection of an ESS specification rather than a hand-written
parser with no contract beside it. Two consequences follow, and the epic is finished when both hold:

1. A command cannot be added in the wrong place, given no target, or dropped, without the gate
   saying so.
2. A second frontend — web, TUI, MCP — reads the same declaration the command tree was generated
   from, so it costs a projection rather than a rewrite.

## Why now

Two defects share one cause, which is that nothing declares the surface.

**No categorization.** 16 top-level commands in one block. `init`, `doctor`, `login`, `serve` and
`operation` sit at one level despite being five different activities. clap renders exactly one
`Commands:` block per level (`clap_builder-4.6.6/src/output/help_template.rs:403-416`), so the
grouping has to be real nesting; there is no help-only shortcut.

**No declared target.** `operation`, `connection` and `event` route by the absence of two flags
(`crates/connectors-cli/src/lib.rs:729,789,871`). The same command line reaches a hosted deployment
or the local socket depending on a file the operator did not name. Carried by
`story:explicit-target-never-implicit`.

## Shape

ESS gains a command-line surface — a third `reached_by` and a `cli:` block — and a fourth synthesis
target beside `rust`, `go` and `web`, which already exist
(`ess generate synthesize --target rust|go|web`). `connectors` is its first consumer: it draws its
domains from `crates/protocol/src/`, declares its groups, generates its parser and completions, and
holds the committed tree byte-identical to a fresh generation.

## Not in this epic

- Building a second frontend on `--target web`. The epic makes it cheap; it does not spend it.
- Renaming any `operation`, `connection` or `event` subcommand. Only the top level moves.
- The `zwirn` pin bump and its ADR (`zwirn/crates/agent-app/src/main.rs:124` forwards argv verbatim).

## Provenance

Filed 2026-09-04 from a plan reviewed with the operator. Plan file:
`~/.claude/plans/lazy-painting-turtle.md`.
