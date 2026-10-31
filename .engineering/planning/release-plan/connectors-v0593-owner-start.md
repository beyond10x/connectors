---
format: aep.planning-md/3
id: release-plan:connectors-v0593-owner-start
kind: release-plan
status: implemented
title: 'Release 0.59.3: a read waits for a slow owner start within its own deadline'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "implemented", at: "2026-10-30T05:24:40Z", actor: "human:timo", revision: 3}
---
## Outcome

Patch release 0.59.3, alone, under `approval-record:d248-owner-start-release-0593`:

- A read on `operations invoke` with no owner running waits for the owner it starts within its own 30 s
  deadline instead of a fixed 10 s. On a large store the owner's start outlasted 10 s, the CLI ended it and
  answered `timeout` at `admission`, and every retry did the same
  (`story:owner-start-outlasts-a-fixed-bound`).
- Other commands that start an owner wait up to 60 s for it, also for one another process is starting; a
  command that times out leaves the owner it started to finish its start.

This is a patch: the CLI's surface, the contracts' types and the models are unchanged; owner.md states the
new waits.

## Scope

| story | status |
|---|---|
| story:owner-start-outlasts-a-fixed-bound | implemented |

## Limitations

- Writes keep the 10 s wait for an owner's greeting inside their 20 s budget, so a write that has to start an
  owner on a large store can still time out.
- Every cold start verifies the whole store twice and the store keeps no snapshot; a store whose
  verification outlasts 30 s still times out the first read (the next one reaches the owner it left).
