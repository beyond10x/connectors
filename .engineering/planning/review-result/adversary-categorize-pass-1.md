---
format: aep.planning-md/1
id: review-result:adversary-categorize-pass-1
kind: review-result
status: active
title: Adversary pass 1 against the categorized top level
relations:
- reviews: story:cli-first-level-groups
revision: 1
---
# Adversary pass 1 — `story:cli-first-level-groups`

Worktree `wt-e9b482b00009`, uncommitted over `967062d`. `adp:adversary`.
128,662 tokens, 60 tool uses, 875 s.

## Header, as returned

```
verdict: red
cases: executed 54->61, red 7
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: yes — the tree moved under the adversary at 19:36
```

## Four regressions in the compatibility shim

Each is a path that worked at the base commit and does not now.

| construction | result |
|---|---|
| `connectors --output json <any moved word>` | all **11** entries fail; the shim reads `argv[1]`, finds `-o`, matches nothing |
| `connectors help <moved word>` | **10 of 11** exit 2 `unrecognized subcommand`, and `help` is a word `connectors --help` advertises |
| `connectors serve help` | rewritten to `serve local help` and refused, while `connectors help serve` prints `help` in the group's own `Commands:` list |
| `connectors auth` | exits 2 naming no destination; `MOVED` carries `auth status` and nothing for `auth` |

The sharpest form, measured by hand:

```
$ connectors serve --config /nope/x.toml
note: `connectors serve` is now `connectors serve local`, and the old path works for one more release
runtime: Connector configuration could not be read: No such file or directory (os error 2)

$ connectors -o json serve --config /nope/x.toml
error: unexpected argument '--config' found
```

The same argv means two different commands depending on whether a global flag precedes it.

The `serve help` case has a mechanism worth recording: the guard calls `find_subcommand` on
`Cli::command()`, which the derive returns **unbuilt**. clap adds `help` in `_build_self`, so the
guard cannot see the group's fourth command.

## Two the coordinator caused

The operator instructed the `CLI_TOTAL_LINE_LIMIT` fence removed entirely — a line count that had
been raised six times (856, 960, 966, 1006, 1014, 1127) and had never once moved code out of the
binary. The coordinator removed it while this adversary was running.

- `docs/design/19-the-cli-surface.md:182` still gives that cap as the first of "Two reasons, both
  measured" for keeping `ess/generated/clap/` outside the crate. The claim outlived the fence.
- The removal shrank `architecture_fence.rs` from about 648 lines to 616, and the design document
  cites `:607-621`. `cli_surface.rs::every_citation_this_unit_wrote_resolves` is **red** on the tree
  as handed, so the unit's declared 53-green does not hold.

That is the fifth `path:line` citation broken by a line shift in one day, and the only one caused by
the coordinator's own edit rather than an agent's.

## What held

- `ess/generated/clap/` byte-identical under the installed `ess`; `diff -r` exit 0.
- Completion scripts render for all five shells: bash 2735, zsh 2151, fish 401, powershell 762,
  elvish 635 lines. The legacy `connectors completions fish` works and prints the note.
- The guard's suppression direction has no wrong instance: `serve` is the only moved word that is
  also a group, and `serve local|hosted|mcp` were never legacy paths.
- Every `MOVED` entry driven bare with `--help` is green.
- The root workspace suite including `catalog-build --test main` (75) is green.

## Findings, verbatim

```findings
- file: crates/connectors-cli/src/lib.rs
  line: 492
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the shim computes its leading token sequence from argv[1] without skipping the global -o/--output flag, so all eleven legacy paths fail outright when the documented format flag precedes the word that moved, and `connectors -o json serve --config X` refuses what `connectors serve --config X` runs.
- file: crates/connectors-cli/src/lib.rs
  line: 506
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the table is matched at argv[1] only, so `connectors help <moved word>` — the path `connectors --help` itself advertises — exits 2 with `unrecognized subcommand` for ten of the eleven entries.
- file: crates/connectors-cli/src/lib.rs
  line: 498
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the guard calls find_subcommand on an unbuilt Cli::command(), which has no clap-generated `help` subcommand, so `connectors serve help` is rewritten to `serve local help` and refused even though `connectors help serve` prints `help` in the group's own Commands list.
- file: crates/connectors-cli/src/lib.rs
  line: 468
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: MOVED carries the two-word `auth status` and no one-word `auth`, so `connectors auth` and `connectors auth --help` — a first-level group at the base commit — exit 2 naming no destination.
- file: crates/catalog-build/tests/main/architecture_fence.rs
  line: 12
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: CLI_TOTAL_LINE_LIMIT and its assertion were removed on the operator's instruction, and docs/design/19-the-cli-surface.md:182 still gives that cap as the first of two measured reasons for keeping ess/generated/clap outside the crate.
- file: docs/design/19-the-cli-surface.md
  line: 184
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the unit's own cli_surface.rs::every_citation_this_unit_wrote_resolves is red on the tree as handed because the page cites architecture_fence.rs:607-621 and that file now has 616 lines, so the declared 53-green does not hold.
- file: crates/connectors-cli/tests/moved_paths_are_not_taught.rs
  line: 141
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the printed-path fence requires the moved words to sit adjacent to `connectors `, so a global flag between them hides the site, and it misses crates/connectors-console/src/output.rs:176 which is inside the class the unit itself corrected at output.rs:449.
```
