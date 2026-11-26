---
format: aep.planning-md/3
id: story:an-unknown-write-is-settled-by-a-declared-read-back
kind: story
status: draft
title: An unknown write is settled by a read-back its operation declares
summary: A write operation declares a read-back beside its preflight; after an unknown write the owner runs it once and names confirmed_by
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:landed-write-is-not-unknown-after-the-owners-budget
revision: 1
---
## Gap

A consumer whose write answers `unknown` runs a read of its own afterwards to learn whether the
write landed. Split from story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice
on round 1 of the plan critics: a read-back named on the command line leaves its input, its
match rule and its admission open.

## Change, to be designed

- A write operation declares its read-back in the provider definition, beside `preflight`
  (`adapters/catalog/providers/slack/operations.json:64`): the read operation, its input built
  from the write's input, and a pointer into the read's answer that shows the write.
- After an unknown write, the owner runs the declared read once, under the read's admission
  and deadline (`ess/domains/cli.yaml:1820-1824`). The operation must be admitted on the
  instance.
- The answer names `confirmed_by` when the read shows the write. UNMAPPED: whether the ledger
  attempt moves from `unknown` to `applied` or stays `unknown`; a same-key replay answers from
  the ledger.
- The pairing of a write operation with a read operation is a relation and is modelled in ESS
  first (guardrail 7).

## Acceptance

To be written once the design is settled.

## Relations

Informed by story:landed-write-is-not-unknown-after-the-owners-budget, whose workaround is this
read-back.
