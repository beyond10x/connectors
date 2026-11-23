---
format: aep.planning-md/3
id: story:timeout-refusal-says-how-long-it-waited
kind: story
status: draft
title: A timeout refusal of invoke says how long the call waited (waited_ms)
relations:
- decomposes: epic:review-20261123-findings
- depends_on: story:invoke-admission-queues-under-load
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: ess/domains/cli.yaml
- confidence: inferred
  path: website/docs/reference/cli.md
- confidence: inferred
  path: website/docs/reference/contracts/cli.md
revision: 7
---
## Outcome

A `timeout` refusal of `connectors invoke` at `admission`, or at the owner's start, carries
`waited_ms`: how long the call waited, measured by the CLI from the instant its cutoff was taken.

## Source

Split from story:invoke-admission-queues-under-load on 2026-11-23 by the design critic's finding
8: the flag cures the reported refusals alone, and this member changes the shared, closed
`connectors.cli.Failure` (`ess/domains/cli.yaml:791-815`) and every file generated from it.

## Acceptance

- `connectors.cli.Failure` declares `waited_ms` as an optional integer with a presence rule like
  `retry_after_seconds` (`ess/domains/cli.yaml:812-815`); the generated outputs are regenerated.
- A `timeout` at `admission` and a `timeout` of the owner's start carry it; a `timeout` at
  `dispatch` does not (that time is the provider's).
- With `--deadline-seconds 2` and a read that cannot be dispatched in time, `waited_ms` is 2000
  or more: a test in an existing test file.
- `semantics.md` and the generated CLI reference name the member.

## Out of scope

The owner's wire (`owner::Error` is `deny_unknown_fields`): unchanged.
