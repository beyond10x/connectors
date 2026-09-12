---
format: aep.planning-md/1
id: story:host-suite-load-sensitivity
kind: story
status: draft
title: The connectors-host timing suite fails under concurrent load and passes alone
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Acceptance

`cargo test -p connectors-host --lib` passes on a machine under concurrent load —
a second full gate running, several agents building — with no test failing on
`Timeout`, `Unavailable` or `OutcomeUnknown` where it passes on an idle machine.
The bound each of these tests waits on is derived from something the test
controls, or the test states the load it requires and refuses rather than failing.

## What was measured

Four separate gate runs on 2026-09-12 were turned red by tests in
`crates/connectors-host` that pass three times out of three when run alone.

| gate | load | failed | error |
|---|---|---|---|
| `wave/helm-reads-20260912` | two gates + agents | `drop_eof_and_original_deadline_destroy_pending_without_a_write` | `Timeout` |
| release v0.3.0 | 13.20 | same, plus `native_outcomes_and_lost_replies_never_repeat_a_send` | `Unavailable`, `Timeout` |
| `wave/mcp-w3-20260912` | 8.51 | same, plus `recovering_append_reuses_exact_fields_and_reads_a_lost_acknowledgement` | `Timeout`, `OutcomeUnknown` |

Each time, the named tests were re-run alone on the same tree: **3 of 3 passing**,
8.8 to 9.1 seconds each. The suite reports `135 passed; 2 failed` under load and
`137 passed` idle.

The three tests live in `local::runtime::process::write_tests` and
`local::audit::tests`. All three wait on a deadline that a busy scheduler can
exceed.

## Why this is worth a story rather than a re-run

It has cost four gate runs today, each roughly eight minutes of compile and test,
and it blocked the v0.8.0 release gate — which is how it was noticed rather than
absorbed.

The failure is indistinguishable from a real defect at the moment it happens. A
coordinator reading `exit 1` has to stop, read what the step printed, identify the
tests, re-run them alone and only then conclude. A wave that trusted the exit
status alone would have held a green unit out; a wave in a hurry would have
re-run the gate until it passed, which is the same thing as not gating.

## Out of scope

Making the tests faster. The problem is not duration, it is that a bound which
holds on an idle machine is asserted as though it always holds.

## What is not established

Which bound each test waits on, and whether the three share one mechanism or have
three. Nothing here reads the test sources — this story is filed from four
observations and their re-runs, and the first task under it is to read the three
tests and say what each is waiting for.
