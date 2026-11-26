---
format: aep.planning-md/3
id: story:invoke-retries-host-side-not-attempted-causes
kind: story
status: draft
title: invoke retries the host-side causes of a write that was not attempted
summary: A clock refusal, the host's own preflight timeout and a busy adapter join the retry set of invoke once each has a marker and a budget rule
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice
- depends_on: story:a-clock-refusal-of-issue-and-invoke-names-its-sources
revision: 1
---
## Gap

A consumer asked that `invoke` retry every write that connectors classifies `not_attempted`,
naming unavailable, timeout and the approval-clock refusal.
story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice retries only the
provider's own answers to the preflight (timeout, capacity, unavailable). This story owns the
host-side causes it leaves out:

- a clock refusal at stage `admission`, after the owner's second attempt: the host answers it
  as a bare `unavailable` without a clock marker today
  (`crates/connectors-host/src/local/owner.rs:261-272`), so it cannot be told from other
  `unavailable` answers until story:a-clock-refusal-of-issue-and-invoke-names-its-sources marks
  it;
- the host's own timeout of a hung preflight, which ends the adapter child
  (`crates/connectors-host/src/local/runtime/process.rs:1083-1085`), so a next try needs a fresh
  child and the hang has already used most of the 20 s write deadline;
- a busy or unavailable adapter (`PluginBusy`, origin host).

## Change, to be designed

Each cause joins the closed set of the retry story only with a rule for its budget: a hung
preflight costs up to the host's preflight bound, so the reserve rule of the retry story
decides whether a second try fits in the write's deadline. UNMAPPED: whether a caller-named
write deadline (story:landed-write-is-not-unknown-after-the-owners-budget) is needed for these
retries to be useful.

## Acceptance

To be written once the clock refusal is marked and the budget rule is settled.
