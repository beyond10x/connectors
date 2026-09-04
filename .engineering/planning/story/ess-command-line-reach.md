---
format: aep.planning-md/1
id: story:ess-command-line-reach
kind: story
status: implemented
title: A component can say its callers are people at a terminal
relations:
- derived_from: epic:cli-surface
revision: 4
---
# Story: a component can say its callers are people at a terminal

## Defect

`ess/1` has two answers to *where are this component's callers*: `in_process` and `network`
(`crates/specify/ess-domain/src/component.rs:142-156` at tag `0.13.5`). Neither describes a binary a
person types at. A CLI's callers are deployed with it, so `in_process` is nearly right and says the
surface never leaves the process — but the surface does leave it, as a grammar rather than a call,
and nothing in the model can say so. The consequence is that a command-line surface has no contract
ESS can project, which is why `connectors` has 16 top-level commands and nothing that declares them.

## Shape

- A third `Reach` variant, `command_line`, stating the same kind of fact the other two state — who
  issues the commands — and naming no wire, port, path or verb. The `Reach` doc comment's own rule
  holds: which contract follows is derived, never chosen.
- `Reach::ALL` and `Reach::as_str` (`:160-167`) gain it; the doc comment saying "the two words"
  becomes three.
- `RawComponentSpec` gains one optional `cli:` block: a `binary:` name, an ungrouped `commands:`
  list, and `groups:` of `{name, summary, commands}`. It holds what cannot be derived — which
  activity a command belongs to. Paths inside a group derive from `naming.wire`, as OpenAPI paths
  already do.

## Refusals

Five, and they are the reason to model this rather than write a document beside the parser:

| refused | why |
|---|---|
| `cli:` on a component that is not `reached_by: command_line` | it describes a surface the component does not have |
| a command in `cli:` that is not in `accepts.commands` | the unknown-target refusal `relations:` already gives |
| an accepted command in no group and not in `commands:` | the surface would be incomplete and nothing else would say so |
| an accepted command in two groups | a command has one path; two answers is an undecided question written twice |
| `reached_by: command_line` with no `cli:` block | nothing to project |

## Format decision

`ess/1`, not `ess/2`, on the precedent `reached_by` set: it serialises out when unset, so every
existing document digests exactly as before (`Reach::is_in_process` doc comment). `ess/AGENTS.md`
requires this be decided explicitly and requires an old-reader compatibility test before any
persisted field, so both are part of this story:

- a test proving a document using neither addition produces byte-identical IR;
- the forward-incompatibility written down: an older `ess` binary refuses a document that uses
  either, the same consequence `reached_by` had.

## Acceptance

- `ess validate` accepts a component declaring `reached_by: command_line` with a `cli:` block.
- Each of the five refusals has a test that names it.
- The old-reader test passes: IR bytes unchanged for documents without the new fields.
- `task check` exits 0.

## Where the work lands

`github.com/beyond10x/ess`, not this repository. Filed here because
`epic:cli-surface` spans both and `connectors` is the first consumer.
