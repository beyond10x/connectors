---
format: aep.planning-md/1
id: story:ess-clap-target
kind: story
status: implemented
title: ESS synthesizes a clap parser and its completions
relations:
- derived_from: epic:cli-surface
- depends_on: story:ess-command-line-reach
revision: 4
---
# Story: ESS synthesizes a clap parser and its completions

## Defect

`ess generate synthesize --target rust|go|web` already turns one IR into three implementations, and
`web` is a whole frontend (`crates/generate/ess-synth/src/web/` — `catalog.rs`, `page.rs`,
`layout.rs`, `bridge.rs`, `refusal.rs`). A command-line surface has no target, so a CLI is the one
frontend that stays hand-written, drifts from the specification, and cannot be reproduced from it.

## Shape

- `crates/generate/ess-synth/src/clap/`, beside `rust/`, `go/` and `web/`, in the same module shape
  those three share: `layout.rs`, `name.rs`, `items.rs`, `obligation.rs`, `refusal.rs`, `port.rs`.
- `clap` joins the `--target` value list.

What it emits, from what:

| artifact | from |
|---|---|
| the clap derive tree: root `Parser`, one `Subcommand` enum per group | `cli:` groups, `naming.wire` |
| one typed arg per command input field | command `input:` |
| a `completions` subcommand wired to `clap_complete` for every shell | the `binary:` declaration |
| exit conditions, error variants included | command `outcomes:`, `errors:` |
| a `Handler` trait, one method per command | the obligation model `rust/obligation.rs` already uses |

The generated half is the grammar. Handler bodies stay hand-written and are reported as obligations,
which is the existing synth contract rather than a new one.

## Why it is worth more than a document

`--target web` reads the same IR. Once `cli:` declares the groups, a web frontend gets its navigation
from the declaration that produced the command tree, and a third frontend costs a projection rather
than a rewrite. That is the property a hand-written parser with a contract document beside it does
not have.

## Acceptance

- `ess generate synthesize --target clap --out <dir>` run twice produces identical bytes.
- The generated tree compiles and its `--help` shows one entry per declared group.
- Completions are generated, not hand-maintained.
- Every command with no handler is reported as an obligation, by name.
- `task check` exits 0.

## Depends on

`story:ess-command-line-reach` — there is nothing to project until `cli:` exists.

## Where the work lands

`github.com/beyond10x/ess`, not this repository.
