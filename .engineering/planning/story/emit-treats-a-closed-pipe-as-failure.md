---
format: aep.planning-md/1
id: story:emit-treats-a-closed-pipe-as-failure
kind: story
status: active
title: A reader that closes the pipe early makes connectors exit non-zero
summary: emit returns Err(BrokenPipe) and only the completions subcommand maps it to success, so piping into head fails above the pipe buffer.
tags:
- ready
- wave-cli
scope:
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: inferred
  path: crates/connectors-cli/tests/closed_pipe.rs
revision: 13
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

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 10 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived from the measured first-pass findings and retained correction on 2026-09-06.

- `crates/connectors-cli/src/lib.rs` — cited.
- `crates/connectors-cli/tests/closed_pipe.rs` — cited.

High confidence. Normalize BrokenPipe at the result-output completion boundary, including the existing nested admin output wrapper, while allowing commands such as doctor to finish their semantic status decision. Socket/network/stdin and other writer failures remain failures. Prove every output format, healthy and unhealthy reports, nested admin output and non-BrokenPipe controls. Preserve retained first-pass cases. The existing JSON/YAML diagnostic renderer can exit 101 when a semantic failure's error envelope encounters closed stdout; this remains unsuccessful and is not a successful-result output path.

Would collide with any unit editing these exact paths; directory scope entries additionally require containment review.

## Execution queue

CLI execution queue 2026-09-06: 7 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.
