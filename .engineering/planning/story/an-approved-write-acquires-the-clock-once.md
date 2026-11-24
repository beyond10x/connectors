---
format: aep.planning-md/3
id: story:an-approved-write-acquires-the-clock-once
kind: story
status: draft
title: An approved write acquires the approval clock once
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
## Observation

One approved write through the owner acquires the approval clock twice: `time()`
(`crates/connectors-host/src/local/owner/mutation/execution.rs:84-97`) is called at `:297` and
at `:433`. With `approvals issue` before it, one approved write needs three acquisitions, and
each can be refused by a network dropout
(story:an-approved-write-survives-one-refused-clock-acquisition).

## Change (draft)

The second call reads the clock of the first where that clock still answers, and acquires
again where it does not.

## What stands against a plain reuse

Design critic of the story above, round 1:

- The second acquisition is taken after native preflight on purpose
  (`execution.rs:427-433`: "No policy/key/metadata lock is held during native preflight or
  fresh clock acquisition").
- A bounded clock refuses every reading once a counted source's elapsed upper bound passes
  30 s (`crates/connectors-host/src/local/clock.rs:817-818`;
  `contracts/service/clock.md:168-169`), and its answer widens with the rate error up to the
  cap of 8,000 ms.
- A slow preflight would therefore turn a write that is admitted today into `unavailable`
  after audit admission and preflight.

## Open before it is proposed

- Whether the saving (one acquisition of three) is worth a second code path, now that each
  acquisition makes a second attempt.
- The acceptance: a test with a preflight longer than the 30 s window (with a test clock, not
  a wait), and a count of the requests each source receives during one admitted write (1 with
  the reuse, 2 today).
