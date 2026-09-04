---
format: aep.planning-md/1
id: story:emit-treats-a-closed-pipe-as-failure
kind: story
status: draft
title: A reader that closes the pipe early makes connectors exit non-zero
summary: emit returns Err(BrokenPipe) and only the completions subcommand maps it to success, so piping into head fails above the pipe buffer.
scope:
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-console/src/output.rs
revision: 2
---
## Context

`connectors providers | head -1` exits non-zero once the output exceeds the 64 KiB pipe buffer.

Observed at runtime by the `adp:adversary` agent on 2026-09-04, not inferred:
`emit(Format::Text, providers::run(""))` into a reader that exits immediately returns
`Err(Io(Os { code: 32, kind: BrokenPipe }))`. It neither panics nor succeeds.
`MainError::Output` at `crates/connectors-cli/src/lib.rs:385` then falls through to `emit_error`
and `ExitCode::FAILURE` at `:444`. The only place a closed pipe is mapped to `Ok(())` is
`Command::Completions` at `:459`, added the same day for the completions script.

Marked `pre-existing`: `emit` and the CLI's error handling are byte-identical to the base commit
the readability unit forked from.

## Acceptance

A reader that closes the pipe early — `connectors providers | head -1` above the pipe buffer —
leaves `connectors` exiting 0, in every output format, and a genuine write failure still exits
non-zero.

## Notes

Reachability moved in the opposite direction this week. The CLI output readability work shrank
`providers` text output from roughly 40 KiB to roughly 14 KiB, which is below the pipe buffer, so
the defect became harder to hit rather than easier. It is still wrong: `head`, `less` and a reader
that stops early are ordinary, and the output grows with the catalogue.
