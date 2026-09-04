---
format: aep.planning-md/1
id: story:informative-command-readability
kind: story
status: draft
title: Informative subcommands render as a scannable report, not a JSON dump
summary: doctor, providers and auth status share one generic pretty-printer that spends 26 lines on 6 checks, hides the one warning among the ok rows, and drops fields in compact.
scope:
- confidence: cited
  path: crates/connectors-console/src/doctor.rs
- confidence: cited
  path: crates/connectors-console/src/output.rs
revision: 2
---
## Context

Every informative subcommand builds a `serde_json::Value` and hands it to `output::emit`
(`crates/connectors-cli/src/lib.rs`, 13 call sites). In `text` — the default — that value is walked
by one generic JSON pretty-printer, `output.rs:168`, which knows nothing about what it is printing:
`key: value` per scalar, two spaces per nesting level, a bare `-` line per array element.

Measured on this machine against `connectors 0.5.7`:

| command | lines | records |
|---|---|---|
| `connectors doctor` | 26 | 6 checks |
| `connectors providers` | 927 | 65 providers |
| `connectors providers --query jira` | 22 | 1 provider |
| `connectors auth status` | 133 | 7 providers |

`doctor` spends four lines per check — a bare `-`, then `check:`, `status:`, `detail:` — and the
one `warn` it found here renders identically to the five `ok` lines above it. The module that
produces the report already ranks the three states and says why (`doctor.rs:11`: "`fail` is *this
cannot work*; `warn` is *this works and you should know*"), and the renderer discards that rank at
the last inch. `providers` averages 14 lines per provider, so the catalogue does not fit on a
screen and cannot be scanned.

`-o compact` is closer to readable — `doctor` is 6 lines there — but it is not the default and it
loses data: `compact` unwraps the single array field (`output.rs:126`, `output.rs:137`) and
discards the scalar siblings beside it, so `healthy: true` is absent from every compact `doctor`
run. Verified: `connectors -o compact doctor | grep -c healthy` prints 0.

## Acceptance

`connectors doctor`, `providers` and `auth status` each render in `text` as one aligned row per
record with the severity or readiness of a row distinguishable without reading its text, no output
format drops a field the value carries, and the bytes of `-o json` and `-o yaml` are unchanged.

## Scope

- `crates/connectors-console/src/output.rs` — the renderer. `text` gains a table form for an array
  of uniform objects; `compact` stops discarding scalar siblings.
- `crates/connectors-console/src/doctor.rs` — if severity needs to reach the renderer as more than
  the string `"ok"`.
- No change to `crates/connectors-cli` — the frontend passes a value and a format, and that is the
  right seam. The thin-frontend line cap does not move for this story.

## Notes

- **The renderer is shared, so this is one change and every informative command gets it.** Thirteen
  `output::emit` call sites, one `text` function. A per-command renderer would be thirteen places
  for the format to drift.
- **`json` and `yaml` are contracts and do not move.** `output.rs:1` states why a structured format
  carries its failures on stdout; a caller parsing those bytes is not the reader this story is for.
- **Colour is a decision, not an assumption.** If severity is carried by colour it needs TTY
  detection and `NO_COLOR`; a symbol or a leading status column needs neither and works in a pipe.
  Prefer the form that survives redirection, and treat colour as an addition on top of it.
- **The compact data loss is a defect, not a preference.** It is written here because it lives in
  the same function as the readability work, and it can be split into its own story if that is
  wanted.
