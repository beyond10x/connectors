---
format: aep.planning-md/3
id: review-result:wave-d417-u1-adversary
kind: review-result
status: active
title: Adversary pass on the cold owner start
relations:
- reviews: story:cold-owner-start-verifies-once
revision: 1
---
**Verdict: confirmed, 1 blocker, 2 acceptance gaps, 1 note. Decided in D422 (approval-record:connectors-d417-d422-u1-reviews): all four are fixed in the unit.**

Adversary on `e60844a1d`, wave D417 unit U1. Cases in commit `23247269c` on `unit/d417-cold-owner-start`:
`apps/connectors/tests/cold_owner_start_adversary.rs` and
`crates/connectors-host/src/local/metadata/checkpoint_tests.rs`. Executed 12 to 16, 3 red. Each red case was first run
alone: F1 relock `Err(MetadataUnavailable)` at `checkpoint_tests.rs:294` (exit 101); F2 and F3 `(2, 2, 2)` against
`(1, 1, 1)` at `cold_owner_start_adversary.rs:125` and `:188`. Logs are in `~/.cache/connectors-wave-20261103/u1-adv/`.
The adversary wrote F4 as `suspected`; the block records it as NEEDS-CHANGE.

```findings
[
 {"file":"crates/connectors-host/src/local/metadata/er.rs","line":579,"category":"concurrency","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"F1: take_or_wait waits while its caller holds the lifecycle flock (activate_er(false), metadata.rs:1057) for a handle that an unlocked observation (Registry::transaction_observation) holds; the observation then cannot relock and is refused metadata_unavailable, and the locked open waits out the lock bound before it gets the handle. Before this change the locked open verified afresh at once and the relock succeeded. Test: crates/connectors-host/src/local/metadata/checkpoint_tests.rs::a_locked_open_never_waits_for_a_handle_whose_user_waits_for_its_lock (red: relock Err(MetadataUnavailable))"},
 {"file":"crates/connectors-host/src/local/owner.rs","line":668,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"undecided","message":"F2: A cold read whose description is not cached for the configured selection (as after an adapter or executable change) verifies the store twice, in the owner and again in the CLI: the owner's cached answer description_unavailable falls back to cached_direct and to launch_admits in the CLI. This breaks the acceptance's 'one cold start verifies once'. Test: apps/connectors/tests/cold_owner_start_adversary.rs::a_cold_read_of_an_uncached_description_verifies_the_store_once (red: (2,2,2), read exit 0)"},
 {"file":"crates/connectors-host/src/local/owner.rs","line":567,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"undecided","message":"F3: For an instance with a bound description, a cold read with --connection runs Registry::describe directly in the CLI (bound_described) after the owner has started, so the store is verified in two processes. Warm reads of such instances also verify the whole store in the CLI every time. Test: apps/connectors/tests/cold_owner_start_adversary.rs::a_cold_read_of_a_bound_description_verifies_the_store_once (red: (2,2,2), read exit 0)"},
 {"file":"apps/connectors/src/local/invoke.rs","line":1001,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F4: owner.md says a target invoke that names neither --approval-file nor --idempotency-key starts the owner first, but the code leaves out --describe, so a cold 'invoke --describe' reads the store directly. The contract and the code disagree. Test: none"}
]
```

Attacked without a break: a tampered store refuses `metadata_unavailable` on the cold fallback with no invocation; a
write with no flags is refused `approval_required` before any provider call; lock order CHECKOUTS then IDLE; another
store's or a replaced file's handle is never handed out; a thread never waits for its own handle; the verification log
exists only under `cfg(debug_assertions)`; the owner's `connections` mapping matches the direct listing.
