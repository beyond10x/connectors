---
format: aep.planning-md/3
id: story:legacy-alias-sharing-an-instance-name
kind: story
status: active
title: A legacy alias that shares an instance's name never routes a read to the other entry
summary: 'Reported by loom 0.52.0''s review: a read can reach the other entry; writes stay refused'
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/config.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:07Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:08Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

A read through a name that is both a legacy alias and a configured `[connectors]` instance reaches
exactly the entry the caller named, or is refused as ambiguous. It never silently reaches the
other entry.

## Report

From loom 0.52.0's review, relayed by the coordinator on 2026-10-30, verbatim: "a legacy alias
that shares its name with a `[connectors]` instance can route a read to the other entry; a write
is still refused by its approval proof."

Not yet reproduced in this repository. Legacy aliases and instance entries are resolved in
`crates/connectors-host/src/local/config.rs`; which side wins, and through which caller, is unclear
and needs a reproduction first.

## Acceptance

- `a_read_through_a_name_that_is_both_alias_and_instance_is_refused_as_ambiguous`: a configuration
  where one name is both a legacy alias and an instance is refused at load, or a read through that
  name is refused with a reason naming both entries, and no provider request is sent.
- `a_write_through_such_a_name_stays_refused`: the approval proof keeps refusing a write, as today.
