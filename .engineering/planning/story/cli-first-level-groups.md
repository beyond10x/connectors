---
format: aep.planning-md/1
id: story:cli-first-level-groups
kind: story
status: draft
title: The top level reads as categories, not as sixteen commands
relations:
- derived_from: epic:cli-surface
- depends_on: story:cli-surface-contract
scope:
- confidence: cited
  path: crates/catalog-build/tests/main/architecture_fence.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
revision: 3
---
# Story: the top level reads as categories, not as sixteen commands

## Defect

`connectors --help` lists 16 commands in one block. `init`, `doctor`, `login`, `serve` and
`operation` sit at the same level despite being five different activities, so the help output does
not tell an operator which of them they want.

There is no help-only fix. clap 4.6.6 renders exactly one `Commands:` section per level:
`subcommand_help_heading` renames that section and does not partition it
(`clap_builder-4.6.6/src/output/help_template.rs:403-416`), and `flatten_help` emits one heading per
leaf with its arguments rather than a grouped index (`:878-936`). The grouping has to be real
nesting.

## Shape

Top level goes 16 to 8. The three everyday verbs keep their paths, because they are the point of the
tool and lengthening them costs every invocation.

| group | members | was |
|---|---|---|
| `setup` | `init`, `connect`, `completions` | `connectors init`, `connect`, `completions` |
| `inspect` | `doctor`, `providers`, `auth` | `connectors doctor`, `providers`, `auth status` |
| `session` | `login`, `logout` | `connectors login`, `logout` |
| `serve` | `local`, `hosted`, `mcp` | `connectors serve`, `serve-hosted`, `mcp` |

`operation`, `connection`, `event` and `admin` stay at the top level.

`auth` collapses: `AuthCommand` (`lib.rs:182-191`) has exactly one variant, so `connectors inspect
auth` replaces `connectors auth status` rather than becoming a third level.

Four subcommand enums hold the moved variants verbatim — the variants move, they are not copied.

## Compatibility

Old paths keep working for one release. In `run_from` (`lib.rs:430`) a const table maps a legacy
first token to its group pair before clap is handed the argv, and one stderr line names the new
path. It rewrites `argv[1]` and never inspects a flag: clap remains the only parser, and the repeated
rule that a Rust CLI uses clap derive is not bent by it.

The table is the same one the contract test reads, so the shim cannot drift from the specification.

## Fence

`CLI_TOTAL_LINE_LIMIT` is 1006 and `crates/connectors-cli/src/lib.rs` is at 998
(`crates/catalog-build/tests/main/architecture_fence.rs:33`). Raise it with a dated reason comment,
in the form the `966 → 1006` completions bump set, to what the build reports rather than to an
estimate.

## Cross-repository

`zwirn` forwards argv verbatim into `connectors_cli::run_from`
(`zwirn/crates/agent-app/src/main.rs:124`) and pins this repository at rev `1e0eb9f`
(`zwirn/Cargo.toml:25`), so `zwirn connectors …` changes only when that pin moves. Renaming something
another repository verifies is a coordinated migration with an ADR (`AGENTS.md:7-8`); the ADR and
the pin bump are a follow-up, named in the report, not a blocker for this story.

## Acceptance

- `connectors --help` lists 8 commands.
- `connectors doctor` produces the same output as `connectors inspect doctor`, plus one stderr line
  naming the new path.
- The contract test passes, so the specification and the tree agree after the move.
- `bash scripts/gate.sh` exits 0.

## Depends on

`story:cli-surface-contract`.
