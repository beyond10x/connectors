---
format: aep.planning-md/1
id: story:console-clippy-findings
kind: story
status: draft
title: connectors-console fails clippy on five pre-existing findings
summary: Three write_with_newline, one items_after_test_module and one unused_imports, invisible until the package gained a gate lane.
scope:
- confidence: cited
  path: crates/connectors-console/src/enrol.rs
- confidence: cited
  path: crates/connectors-console/src/envelope.rs
revision: 2
---
## Context

`cargo clippy -p connectors-console --all-targets --locked -- -D warnings` exits 101 on five
findings, none of them in code any current story touches:

| file | lint | count |
|---|---|---|
| `crates/connectors-console/src/enrol.rs` | `write_with_newline` | 3 |
| `crates/connectors-console/src/enrol.rs` | `items_after_test_module` | 1 |
| `crates/connectors-console/src/envelope.rs` | `unused_imports` | 1 |

Found by the `adp:implementor` agent on the CLI output readability unit, 2026-09-04, and reported
as `pre-existing` rather than introduced. It verified a fix applies clean and leaves the package
green (clippy exit 0, 55 tests passed), then reverted it because the files were not its to change.
The patch is at
`~/.cache/connectors-wave/informative-command-readability/scratch/pre-existing-clippy.patch`; it is
outside the repository and will not survive a cache clear.

These went unseen because `crates/connectors-console` was in no gate lane. That is being fixed
separately, in the wave that found it, which is what makes these five visible from now on.

## Acceptance

`cargo clippy -p connectors-console --all-targets --locked -- -D warnings` exits 0, and the package's
tests still pass.

## Notes

The `write_with_newline` fix rewrites roughly 150 lines of `enrol.rs`, so it is a reformat rather
than a two-line change. That size is the reason it was not folded into the unit that found it.
