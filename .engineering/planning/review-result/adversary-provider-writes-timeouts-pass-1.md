---
format: aep.planning-md/3
id: review-result:adversary-provider-writes-timeouts-pass-1
kind: review-result
status: active
title: Adversary pass 1 on provider refusals for writes and timeouts
relations:
- reviews: story:provider-refusal-stage-for-writes-and-timeouts
revision: 1
---
unit: story:provider-refusal-stage-for-writes-and-timeouts, uncommitted tree wave0929c-writes on 74f09e6d4
verdict: NEEDS-CHANGE
cases: executed 375→377, red 1
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: scratch/adversary logs
needs-coordinator: whether the host's own timeout and capacity refusals keep admission on writes too

Cases in crates/connectors-host/tests/provider_timeout_adversary.rs: a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout
(red: unmarked Timeout), a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked (green).
Could not break: a timeout after a write was sent stays outcome_unknown; stored refusals replay; old entries replay as
host; 4xx outside the table give Unknown; SQL marks; unmarked limits stay admission or service_failure; origin
survives the owner hop.

Coordinator routing: findings 1, 3 and 5 to the implementor (mark a stalled body, stage from origin on writes, never
retry_explicitly for Applied or Unknown); finding 4 is the coordinator's CHANGELOG line at close; finding 2 is
covered by the adversary's TLS-stall case.

```findings
[
  {"file": "crates/connectors-client/src/lib.rs", "line": 170, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a provider body that stalls past the 15 s deadline after the request was sent and answered returns an unmarked Timeout (admission), contradicting the new semantics.md §6 rule"},
  {"file": "crates/connectors-host/src/http.rs", "line": 497, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the unit's own timeout test never takes the is_connect branch, so removing the guard stays green"},
  {"file": "apps/connectors/src/local/operations.rs", "line": 208, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "every write failure is hard-coded to stage dispatch, so the host's own pre-send timeout or capacity refusal on a write reports dispatch, not admission"},
  {"file": "CHANGELOG.md", "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the wave decision requires a CHANGELOG line stating that an older binary cannot read a new ledger entry carrying origin"},
  {"file": "crates/connectors-host/src/local/owner/mutation.rs", "line": 49, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "next_action ignores the classification, so an Applied write carrying a provider-origin Timeout would report retry_explicitly"}
]
```
