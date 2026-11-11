---
format: aep.planning-md/3
id: story:reconnect-retained-bound-matches-refresh
kind: story
status: implemented
title: Reconnecting a refreshed OAuth2 connection is not refused capacity below the refresh retention bound
tags:
- cr-003
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: inferred
  path: docs/local-connection-registry.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T09:12:02Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-10T09:12:03Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T06:19:32Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Defect

Code review finding CR-003 (S2): Reconnect of a refreshed OAuth2 connection is refused `capacity` at 8 undeleted versions.

- Citations: `crates/connectors-host/src/local/registry/lifecycle.rs:118-119` (`if retained>=8 { return Err(Failure::Capacity); }`); `crates/connectors-host/src/local/registry/refresh.rs:31-36` (`const RETAINED: i64 = 64`, "an hourly refresh retires 24 a day"), `:417`; `docs/local-connection-registry.md:220`.
- What happens: with one-hour tokens, seven or more refreshes in a day leave 8 or more undeleted versions. When the refresh token is then revoked, `connections reconnect` reaches `Registry::prepare` and fails `capacity` until versions age out after 24 h.
- Status: both bounds verified by reading; the route to `prepare` (`owner/transport.rs:2738`) is the reviewer's trace. Not run.
- Fix: one bound for both paths, or count only unretired versions in `prepare`.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-003. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A reconnect of a connection with between 8 and 64 retained versions is not refused `capacity`; a registry test is red on the base and green after, and one retention bound is documented in `docs/local-connection-registry.md`.
- If no test can make it red, that evidence is recorded and nothing is changed.
