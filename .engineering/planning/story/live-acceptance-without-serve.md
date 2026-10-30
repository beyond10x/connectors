---
format: aep.planning-md/3
id: story:live-acceptance-without-serve
kind: story
status: draft
title: The live acceptance runner runs without the removed serve command
relations:
- informed_by: story:connectors-do-short-invocation
revision: 1
---
## Problem

0.60.0 removes `connectors serve` (story:connectors-do-short-invocation, D9), but the live
acceptance runner in `crates/connectors-conformance` still describes a federated gateway at
`127.0.0.1:17100` before any scenario group runs. Nothing serves that gateway any more, and the
runner has no option to skip the federated groups, so a live run cannot complete
(`docs/live-e2e.md`).

## Outcome

The live acceptance runner runs again. It either hosts the gateway itself through the
`connectors_host::federation` library, which is still shipped, or it runs the federated groups only
when a gateway address is given and reports them as not run otherwise.

## Acceptance (draft)

- A live run with no gateway runs every non-federated group and reports the federated ones as not
  run, by name.
- A live run with a library-hosted gateway runs the federated groups.
