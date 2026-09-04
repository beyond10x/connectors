---
format: aep.planning-md/1
id: story:cli-first-level-groups
kind: story
status: implemented
title: The top level reads as categories, not as sixteen commands
relations:
- derived_from: epic:cli-surface
- depends_on: story:cli-surface-contract
scope:
- confidence: cited
  path: crates/catalog-build/tests/main/architecture_fence.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
revision: 8
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

Old paths keep working for one release. `moved` rewrites a legacy path before the command runs, and
one stderr line names the path it moved to.

**Nothing in it reads an option's syntax.** clap parses argv, and each parse it performs answers one
question:

1. **Is a word that moved present at all?** Unless one of the first `LEGACY_WINDOW` — 8 — words is
   the first word of a `MOVED` row, no tree is built and nothing is allocated. Every current path
   pays this and nothing else.
2. **Does this release already read the argv?** The real tree is asked to parse it. Only two
   refusals mean a word that moved: `InvalidSubcommand` — `doctor`, `auth status`, `help doctor` —
   and `UnknownArgument`, which is `serve --config` at the group that took the old leaf's name.
   Every other outcome is clap's to answer as it stands: a parse, a help request, a version
   request, a missing value.
3. **Which path was typed?** `read_one_word` reads the argv one word at a time, each read stepping
   over the global options in front of that word — asked of the parser, not named here, so a global
   added later needs no edit. `-o json`, `--output json`, `--output=json` and `-ojson` are one case,
   because clap's option syntax is clap's. The argv handed on is rebuilt from what clap read: the
   globals, `help` if it was there, the new words, then the tail verbatim.

The first version pre-scanned argv from `argv[1]` with a hand-written `spans` that reimplemented
clap's four option-value spellings. That is the argv parsing a Rust CLI does not do, and it was
wrong in six ways, each a path that worked before the move:

| typed | was |
|---|---|
| `connectors --output json <word>` | all eleven entries refused; `--output` is `global = true` |
| `connectors help <word>` | ten of eleven refused, and `help` is a word `--help` advertises |
| `connectors serve help` | rewritten to `serve local help` and refused |
| `connectors auth` | refused, naming no destination |
| `connectors auth help` | the one-word `auth` row put the group's own `help` where `inspect auth` takes no positional |
| `connectors auth -o json status` | a global between the two words of a row defeated the match |

`serve` is in the table and does not shadow the group it names. The row fires only after the real
parse refused, so bare `connectors serve`, `serve --help` and `serve -h` are the group — the way
bare `setup`, `inspect` and `session` are — and `serve --config X` is the old leaf.

`auth` is in the table twice, as the group a person typed and as the two-word path under it. The
longest matching row wins, so the order of the table decides nothing.

`MOVED` and `moved` are `pub`. `tests/moved_paths_are_not_taught.rs` asks the shipped function
whether a written invocation names a path that moved, rather than restating its rules: the
restatement drifted once, which is how `connectors -o compact doctor` shipped, and a hand-copied
`MOVED` in an adversary suite ran three cases against eleven rows of twelve.

## Fence

**The CLI line cap is gone.** `CLI_TOTAL_LINE_LIMIT` was removed on 2026-09-04 by operator
instruction. It had been raised at every one of the six times it fired — 856, 960, 966, 1006, 1014,
1127 — so it never once moved code out of the binary, which is the only thing it was for. What it
did do was cost: raising it inserts lines into `architecture_fence.rs`, and citations into that file
broke that way twice in one day.

`product_cli_is_a_thin_frontend` still bounds the frontend by what it may **link**
(`CLI_DEPENDENCIES`) and what it may **declare** (the forbidden symbols). Those measure the property
directly and refuse rather than negotiate.

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
