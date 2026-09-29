---
format: aep.planning-md/3
id: review-result:adversary-owner-build-handshake-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the owner build handshake
relations:
- reviews: story:owner-build-handshake
revision: 1
---
unit: story:owner-build-handshake, uncommitted working tree wave0929-owner on base d215b3569 (implementation as of 13:56, binary built 14:00)
verdict: CONFIRMED (the correction holds on every path I was asked to attack; 2 red cases on other ground)
cases: executed 14→18, red 2
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory (scratch/adversary2, 9 entries)
needs-coordinator: yes. Finding 1 is a design question: should an owner refuse work from a CLI that sends no build? The contract currently says it should serve such callers.

Cases added in apps/connectors/tests/local_cli.rs: adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs (:656, green; capacity 24, ok:not_running 76, mismatch 0; pass 1 had 42); adversary2_the_build_probe_against_an_owner_at_capacity_reports_capacity (:701, green); adversary2_the_identification_retry_loop_ends_at_its_deadline_without_flooding_the_owner (:726, red: "the identification loop opened 966 owner connections in 10.009324501s with no pause between rounds"); adversary2_a_real_pre_handshake_owner_is_refused_by_name_and_the_reverse_is_observed (:755, ignored, green once against a real 0.15.1 owner); adversary2_an_owner_refuses_work_from_a_caller_that_never_named_its_build (:799, red: "an owner of this build served a status request to a caller that never named its build: {\"kind\":\"success\"}").

Suite (`cargo test -p connectors --no-fail-fast`), exit 101: compatibility 3, input 1, local_cli FAILED 10 passed 2 failed 1 ignored, owner_build_security 2 passed.

Attacked and could not break: same-build owner at 32 clients (0/100 misreported); build probe at capacity answers capacity; a real 0.15.1 owner is refused by name, left running and can be shut down; the loop cannot outlive the 10 s deadline; an owner of another build cannot be reported as the same build; every released owner v0.12.0 through v0.15.1 has deny_unknown_fields on Request.

```findings
[{"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 827, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "the owner serves work requests on a connection greeted without build, so an older CLI (real 0.15.1 measured, exit 0) runs against a newer owner after a rollback and nothing refuses it (local_cli.rs:799 red)"}, {"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 224, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the greet_running retry loop reconnects with no pause, opening 966 owner connections in 10 s against a constructed owner that keeps dropping greetings with build; no real owner was shown to reach this (local_cli.rs:742 red)"}]
```
