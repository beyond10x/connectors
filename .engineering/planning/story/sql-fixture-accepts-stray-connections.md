---
format: aep.planning-md/3
id: story:sql-fixture-accepts-stray-connections
kind: story
status: active
title: The SQL adapter's fake server test fails when a stray connection reaches its port
relations:
- serves: vision:independent-contract-adapters
- informed_by: specification:milestone-acceleration-20261002
scope:
- confidence: cited
  path: adapters/sql/tests/local_runtime.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T11:48:32Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-02T11:48:33Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}}
---
## Observed
The 2026-10-02 closing gate at 67fb9da30 failed a_dispatched_read_reaches_the_database_and_returns_its_refusal in adapters/sql/tests/local_runtime.rs:339. The fixture saw an unexpected startup packet and a second session; five isolated repeats passed. Unrelated traffic remains a hypothesis.

## Acceptance

A SQL-fixture-regression conformance result passes only with the complete positive repetition evidence and the failing negative control specified below.

## Verification
Deliberately inject the unexpected connection before changing the fixture. Isolate its endpoint or distinguish fixture identities; count unrelated traffic separately. Never discard malformed or extra SUT connections to make counts pass. A negative control opening a second SUT session must still fail. This changes fixture infrastructure over existing SQL semantics, not runtime behavior.

## Scope
Cited sole edit surface: adapters/sql/tests/local_runtime.rs. No shared gate or runtime changes. Coordinator records evidence after integration.

## Required conformance cases

The named conformance cases sql-fixture-isolates-unrelated-connection and a_dispatched_read_reaches_the_database_and_returns_its_refusal preserve exact admitted-session counts and reject an extra SUT session, while 50 of 50 repetitions pass during a parallel workspace test run.
