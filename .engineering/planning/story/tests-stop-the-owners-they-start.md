---
format: aep.planning-md/3
id: story:tests-stop-the-owners-they-start
kind: story
status: implemented
title: Tests stop every owner daemon they start
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: apps/connectors/tests/support/mounted.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:35Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:36Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T06:19:40Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Observed
On 2026-10-14, after the wave gates, 123 `__connectors-owner` processes were still running with
parent pid 1. Their working directories were `mounted-jail-*` temporary directories that had
already been deleted. They had been started by the mounted-secret custody and container tests
and by `adapter_fixture` children. 12 came from that day's wave gate and 111 were older, the
oldest about 10 hours. They hold memory and file descriptors until someone kills them, and a
later run can meet a stale owner.

A test that starts an owner daemon (directly, or through a CLI command that spawns one) does
not stop it when the test ends or panics. The jail's temporary directory is removed while the
owner inside it keeps running.

## Acceptance
- Every test fixture that starts an owner stops it on drop, whether the test passes or panics:
  through the owner-stop route, with a kill of the process group as the fallback after a bound.
- A test asserts this: after the mounted custody, container and adapter fixture tests, no
  `__connectors-owner` process started under that test's temporary root is still running.
- A full `cargo test --workspace` run leaves no `__connectors-owner` process with parent pid 1
  behind.
