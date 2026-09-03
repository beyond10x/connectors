---
format: aep.planning-md/1
id: story:cli-shell-completions
kind: story
status: active
title: The connectors CLI prints its own shell completion script
summary: connectors completions <shell> emits a clap_complete script for bash, zsh, fish, elvish and PowerShell.
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/catalog-build/tests/main/architecture_fence.rs
- confidence: cited
  path: crates/connectors-cli/Cargo.lock
- confidence: cited
  path: crates/connectors-cli/Cargo.toml
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
revision: 4
---
## Context

`connectors` is a clap derive CLI (`crates/connectors-cli/src/lib.rs:36`) with 14 top-level
subcommands and nested `auth`, `connection`, `event`, `operation` and `admin` groups. Nothing prints
a completion script, so an operator types every verb and flag from memory or from `--help`. clap
ships `clap_complete`, which renders the parser's own `Command` tree into each shell's syntax, so
the script cannot drift from the surface it completes.

Timo asked for this on 2026-09-04: "for my clap cli install this completion thing and implement it".

## Acceptance

`connectors completions <shell>` prints to stdout a completion script for each of bash, zsh, fish,
elvish and powershell that names every top-level subcommand, and the architecture fence in
`crates/catalog-build/tests/main/architecture_fence.rs` admits the new dependency and the new line
count by name.

## Scope

- `crates/connectors-cli/Cargo.toml` — add `clap_complete`, same major as `clap`.
- `crates/connectors-cli/Cargo.lock` — refreshed for the new dependency.
- `crates/connectors-cli/src/lib.rs` — `Command::Completions { shell }`, one dispatch arm, one test.
- `crates/catalog-build/tests/main/architecture_fence.rs` — `clap_complete` on `CLI_DEPENDENCIES`;
  `CLI_TOTAL_LINE_LIMIT` raised with a dated note.
- `README.md` — how to install the script for a shell.
- `CHANGELOG.md` — Unreleased → Added.

## Notes

- The script is generated for the binary name `connectors`. Zwirn embeds this library as
  `zwirn connectors …` and owns its own completion; `run_from` receives argv and does not know which
  front door called it.
- The generated script ignores `--output`: it is a shell program, not a result.
- Dynamic completion (`clap_complete::env`) is unstable in 4.6 and is not used.
