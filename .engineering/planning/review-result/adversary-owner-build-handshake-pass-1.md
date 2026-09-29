---
format: aep.planning-md/3
id: review-result:adversary-owner-build-handshake-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the owner build handshake
relations:
- reviews: story:owner-build-handshake
revision: 1
---
unit: story:owner-build-handshake, working tree wave0929-owner (uncommitted, base d215b3569)
verdict: NEEDS-CHANGE
cases: executed 11→13, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 (scratch dir, 4 logs)
needs-coordinator: the untracked apps/connectors/tests/owner_build_security.rs and target-security/ in this tree are not mine; another agent added them while I worked

Cases added in apps/connectors/tests/local_cli.rs: adversary_a_transient_greeting_refusal_from_the_same_build_is_not_a_build_mismatch (:432, red: "a same-build owner was reported as another build after one refused greeting ... owner_build_mismatch ... stop_owner"; test result: FAILED. 0 passed; 1 failed); adversary_owner_answers_build_only_when_asked_and_answers_its_own_digest (:483, green).

Suite (`cargo test -p connectors --no-fail-fast`), EXIT=101: local_cli 6 passed 1 failed; owner_build_security 1 passed 1 failed (the security reviewer's case, same defect).

Real binary, 32 idle connections held and one released at a random point, `adapters status` from the same build, 100 runs: err:owner_build_mismatch 42, ok:not_running 58. At the base commit the same condition gave `unavailable`.

Attacked and not broken: every Client method except shutdown calls same_build (begin, invoke, revalidate, status, stop) and WriteClient::connect does; an old owner is refused by name; /proc/self/exe reads the running image; JSON output carries readiness/stop_owner/operational with exit 1; same-build CLI and owner pass; spawn path held under the lock.

```findings
[{"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 195, "category": "concurrency", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "any Unavailable on the first greeting (including the owner's own 32-client shedding at transport.rs:694) falls back to a build-less greeting that a same-build owner answers without build, so a healthy same-build owner is reported owner_build_mismatch/stop_owner (42/100 runs against the real binary; case local_cli.rs:432 red)"}]
```
