---
format: aep.planning-md/3
id: review-result:w1022-u3-adversary-r2
kind: review-result
status: active
title: 'Adversary pass 2, wave 20261022 U3: MCP stdio runtime re-landing'
summary: 'break: 2 warnings, 1 note; G1 and J4 fixed in the unit commit, J3 deferred to story:owner-cancels-a-bounded-read'
relations:
- reviews: story:mcp-stdio-runtime-branch
revision: 1
---
Adversary pass 2 on U3 (story:mcp-stdio-runtime-branch). **Verdict: red (by reading; cases written, run pending the disk pause)**. Round-1 cases 6/6 passed against the round-1 fixes. 6 new cases in `apps/connectors/tests/adversary_local_mcp_server.rs` (981 lines).

| # | sev | test | expected | cause |
|---|---|---|---|---|
| G1 | warning | `adversary_a_cancelled_call_frees_its_slot_for_the_next_call` | red: next call refused `capacity` until the owner returns | `apps/connectors/src/mcp/stdio.rs:132-147`, `:284` |
| J3 | warning | `adversary_a_lost_session_leaves_no_provider_read_open` | red: owner read outlives a `lost` session (introduced by the F2/F3 fix) | `apps/connectors/src/mcp/projection.rs:314-334` (cancel checked only before connect) |
| J4 | note | none | an uncancelled call reaches the owner after the `remote_hangup` t0; §4.1 "stop local admission immediately" | `stdio.rs:220-227` |

Expected green: stdin close discards a reply the lease cannot carry; teardown exits within 5.5 s of an 8 s read; a call cancelled before stdin closes is never answered; the legacy window admits no tool request.
Not broken: frame interleaving (`stdio.rs:276`), lease-gated writes (`stdio.rs:152`, `session.rs:198-207`), exit code with unjoined work (`stdio.rs:336-341`), batches (`protocol.rs:44`). Guide gap: the request slot held by a cancelled call.

## Disposition

Coordinator decisions sent to the implementor: G1 a cancel frees the slot (§3.4), cancelled owner work bounded and joined at teardown; J3 owner work cancellable and cancelled at the teardown cutoff; J4 no owner call starts after t0, such a request gets an error reply; guide updated.
