---
format: aep.planning-md/3
id: review-result:security-owner-build-handshake-pass-1
kind: review-result
status: active
title: Security review of the owner build handshake
relations:
- reviews: story:owner-build-handshake
revision: 1
---
unit: story:owner-build-handshake, uncommitted working tree on d215b3569 at wave0929-owner
verdict: INFEASIBLE (1 finding, warning, introduced). Of the five invariants, four hold; this finding is against the third.
cases: executed 11→13, red 2 (1 mine, 1 the adversary's `local_cli.rs` case, which finds the same defect)
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths
needs-coordinator: no

Cases added in apps/connectors/tests/owner_build_security.rs: every_work_request_is_refused_before_it_reaches_an_owner_of_another_build (green; status, stop, begin, invoke, revalidate and WriteClient::connect all return OwnerBuildMismatch against a real owner of another build, which keeps running, and shutdown across builds still works); an_owner_of_the_same_build_that_sheds_one_connection_is_not_reported_as_another_build (red: "an owner running the CLI's own build was refused as another build (greetings carried build: [false])").

Finding: crates/connectors-host/src/local/owner/transport.rs:195 (greet_running). Any Unavailable on the first greeting makes the CLI greet again without `build`, and that path always ends in owner_build_mismatch, even for an owner of the CLI's own build. Measured: owner_build_security.rs:229, exit 101. What reaches it: the owner's capacity drop at transport.rs:694 (32 or more clients) with a slot freeing inside the reconnect window; built with a fake owner, not reproduced against a real one. Suggested fix: take the no-build path only when the owner shows it predates the handshake (owner replies Failed{capacity} instead of closing, or a third greeting with build after the no-build one succeeds).

Checked and not faulted: the CLI never kills, signals or replaces an owner; same_build() runs first on begin, invoke, revalidate, status, stop and WriteClient::connect; the digest comes from /proc/self/exe with no environment or cfg override; the build field cannot bypass peer-UID or socket checks; no test-only override in a release build.

```findings
[{"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 195, "category": "integrity", "severity": "warning", "verdict": "INFEASIBLE", "origin": "introduced", "message": "greet_running retries without build on any Unavailable, so an owner of the CLI's own build that sheds the first connection (capacity drop at transport.rs:694) is refused as owner_build_mismatch with next_action stop_owner; reproduced only against a constructed owner, not a real one at 32 clients."}]
```
