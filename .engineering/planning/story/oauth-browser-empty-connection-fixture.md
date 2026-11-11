---
format: aep.planning-md/3
id: story:oauth-browser-empty-connection-fixture
kind: story
status: implemented
title: Explain and correct the browser fixture empty-connection assertion
relations:
- informed_by: story:ignored-suites-have-a-runner
- decomposes: epic:tech-debt-review-20260930
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/oauth_adversary_tests.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:45Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T21:28:00Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":2}}}
---
## Observed failure

The new ignored runner's explicit disposable-family operator run on 2026-10-02
executed 32 selected cases: 31 passed and one failed. The unchanged
`local::oauth_adversary_tests::chrome_opens_one_connection_per_navigation` in
`crates/connectors-host/src/local/oauth_adversary_tests.rs` expected exactly one
observed connection at line 290 and observed two empty connections. The runner
correctly propagated failure. Its separate missing pre-handshake adversary binary
is a prerequisite report, not this fixture defect.

## Acceptance

A bounded reproduction explains the two observed connections using recorded
browser and listener events. The test asserts the OAuth loopback contract's actual
security property and has a negative control that would fail when that property
is violated. Do not simply change an expected count from one to two, suppress the
failure, or classify the existing browser case out of the disposable inventory.
Retain the original failure and the exact browser identity. A focused run and an
explicit disposable-family rerun must both report their actual outcomes.

## Scope

Cited: crates/connectors-host/src/local/oauth_adversary_tests.rs.
Inferred only after reproduction: its loopback fixture/listener helpers.
The ignored runner is evidence collection; changes to its failure propagation are
not authorized by this story. This is an existing fixture failure exposed by the
runner, separate from the runner's fixed descendant-cleanup bug.
