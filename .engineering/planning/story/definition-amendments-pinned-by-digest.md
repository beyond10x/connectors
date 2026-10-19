---
format: aep.planning-md/3
id: story:definition-amendments-pinned-by-digest
kind: story
status: draft
title: A definition pins its amendments file by SHA-256, and a changed file is refused
summary: The amendments file of a definition is checked by no digest in either repository
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-catalog/src/amendment.rs
- confidence: cited
  path: crates/connectors-catalog/src/definition.rs
revision: 4
---
## Outcome

A definition that names an `amendments` file also states its SHA-256; `connectors-build definition`,
`operations invoke --definition` and the definitions repository's `task check` refuse a file whose
bytes differ.

## Why later

The second review of release 0.54.0 (2026-10-19, lane s8) changed one key of
`upstream/slack/slack-web.amendments.json` in a scratch copy of the definitions repository: its
SHA-256 moved from `bf07ef02…` to `b23d3f88…` and `task check` still printed `valid`. The Taskfile
hashes only `source.sha256`, the format has no digest field for amendments
(`spec/domains/definition.yaml:20` there), and the Connectors lowerer passes the path on
(`crates/connectors-catalog/src/definition.rs:2668-2672`) while `amendment.rs:141` checks only the
file's own `source_sha256`. The bundle import path does pin it (`import.rs:1884`). A format field is a
change to both repositories, so it is its own story.

## Acceptance

- The definition format carries `source.amendments_sha256` beside `amendments`; the lowerer refuses a
  mismatch with a named refusal, and a test pins it.
- The definitions repository's `task check` hashes the amendments file against that field.
- `connectors-build definition import` writes the field, and the shipped Slack definitions carry it.
