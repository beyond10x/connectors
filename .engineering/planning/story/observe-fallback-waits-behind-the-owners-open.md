---
format: aep.planning-md/3
id: story:observe-fallback-waits-behind-the-owners-open
kind: story
status: draft
title: A fallback read after the owner's 30 s span waits behind the owner's open
relations:
- informed_by: story:connections-list-answers-through-the-owner
revision: 1
---
## Outcome

A `connections list`, `connections describe` or unnamed `describe` whose owner request ends at the
client's 30 s span answers within a stated bound, or the contract says what the bound is.

## Source

The adversary pass on story:connections-list-answers-through-the-owner, 2026-11-23
(review-result:adversary-connections-list-pass-1, third finding), read from the code, no test:

- When the client's 30 s span ends, the owner's work is not cancelled.
- A `list` may still hold `metadata.lock` during a fresh complete open, and the fallback (direct)
  read then waits up to 30 s behind it: about twice the open time.
- If the owner's open exceeds about 60 s the command answers `metadata_unavailable`.

## Open

- Not shown by a run: it needs an open over 30 s, which no test fits inside the 60 s budget.
- Two ways out, not chosen: cancel the owner's work when the client leaves, or state the bound in
  `contracts/cli/v1alpha1/owner.md`.

## Acceptance

Not written. The first step is a measurement of the open time at the store size where the open
passes 30 s (the ignored test `a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds`
takes `CONNECTORS_COLD_START_STORE_MB`).
