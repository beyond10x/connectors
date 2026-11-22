---
format: aep.planning-md/3
id: story:definitions-default-write-guard
kind: story
status: draft
title: A definition gives its writes one default guard
relations:
- decomposes: epic:slack-complete-web-api
revision: 1
---
## Outcome

A `connectors-definition/1` file can give its write operations one default guard, so a generated
write needs no hand-written guard: before, a named identity read must answer as stated (Slack:
`auth.test` with `/ok` true); after, the answer must match the declared expectation (Slack: `/ok`
true), else the write is `outcome_unknown`, never applied.

## Change

- The definitions format gains a `defaults.write_guard` (name settled in the spec first) that the
  lowerer applies to every `effect: write` operation without a `guard` of its own. An operation's
  own `guard` replaces the default.
- Today each of the bot selection's 7 writes repeats the same `before` and `after`
  (`crates/connectors-catalog/tests/fixtures/definitions/connectors/slack.yaml`).

## Acceptance

- A definition with the default and two writes, one with its own guard, lowers to the same
  operations as the same definition with both guards written out.
- A write with neither guard nor default is refused at lowering, as today.

## Scope

connectors: `crates/connectors-catalog` (definition format and lowerer), its spec. Inferred.
