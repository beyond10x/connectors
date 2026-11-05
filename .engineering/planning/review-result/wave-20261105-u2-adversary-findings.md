---
format: aep.planning-md/3
id: review-result:wave-20261105-u2-adversary-findings
kind: review-result
status: active
title: Adversary pass on the write-by-name refusal
relations:
- reviews: story:mcp-write-operation-call-by-name-is-refused
revision: 1
---

**Verdict: confirmed, no route to the provider; 1 should-fix closed in the unit, 3 notes.**

The adversary ran on `13b9a4938`, wave 2026-11-05 unit U2. Its cases are in commit `c0acde398` on `unit/2026-11-05-write-by-name`, in `apps/connectors/tests/local_mcp_server_write_refusal.rs`. Executed 3 to 6. Red 1: the V2 case under a local mutation of `projection.rs:197` (`!= V3` became `== V1`), reverted in the same command; the implementation diff was clean afterwards. Coordinator recheck on the restored tree: targeted test `6 passed; 0 failed`, fmt clean, clippy `-D warnings` clean.

The implementor's attribution was checked:
- The Write case is refused first by the binding check at `projection.rs:197`. Write runs only on `connectors-private/2` (`runtime.rs:717-719`; curation binds Write to external_write and the mutation profile).
- The semantic-effects case is held by `projection.rs:214-221`.

Attacked and not broken:
- an upper-cased name, a `C1_` prefix, another instance's name (`forbidden`)
- a stale or ill-typed revision (`stale_description`)
- a batch (-32600), an id-less call
- the 2025-11-25 lifecycle without `_meta`

Read only, not run: duplicate or concurrent calls are refused `capacity`.

```findings
[
 {"file":"apps/connectors/src/mcp/projection.rs","line":197,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"undecided","message":"F1: admitting connectors-private/2 at the binding check left every case of the unit green, because the profile match refuses the Write case first. Fixed in the unit: adv_a_v2_binding_lists_nothing_and_refuses_its_read_by_name goes red under the mutation."},
 {"file":"apps/connectors/src/mcp/projection.rs","line":224,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"F2: the Read-requirement clause can never be false, because validate_for at :157 and the V3-only check at :197 already guarantee every requirement is Read. It stays as an extra layer."},
 {"file":"apps/connectors/tests/local_mcp_server_write_refusal.rs","line":642,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"F3: the V2 Write case asserts zero provider requests with no request on that binding showing the provider could be reached; no MCP control is possible on V2. The V3 control proves the counter."},
 {"file":"crates/connectors-host/src/local/owner/governed/bounded.rs","line":28,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"F4: the owner's bounded read does not recheck semantic_effects or effects other than network, so the MCP projection is the only guard. Filed as story:owner-bounded-read-rechecks-semantic-effects."}
]
```
