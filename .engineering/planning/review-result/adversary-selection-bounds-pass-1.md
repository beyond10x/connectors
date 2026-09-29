---
format: aep.planning-md/3
id: review-result:adversary-selection-bounds-pass-1
kind: review-result
status: active
title: Adversary pass 1 on catalog selection parameter bounds
relations:
- reviews: story:catalog-selection-parameter-bounds
revision: 1
---
unit: story:catalog-selection-parameter-bounds, uncommitted tree wave0929b-bounds on 361d4dbc7
verdict: CONFIRMED
cases: executed 40→45, red 1
origin: introduced 0 / pre-existing 1 / undecided 0
wrote-outside-worktree: scratch/adversary (suite.log)
needs-coordinator: whether the bound refuses per_page ≤ 0

Cases in adapters/catalog/tests/selection_bounds_adversary.rs: non_positive_per_page_is_refused_before_any_request
(red: per_page 0, -1, "0", "-0", "-100" reach the transport), unusual_integer_forms_are_refused_or_sent_as_checked,
a_bound_on_a_non_query_parameter_is_refused_at_load, a_bounded_guarded_write_is_refused_before_its_preflight,
bounds_round_trip_and_unbounded_selections_serialise_as_before (green).

Coordinator routing: fix. `Bound` gains an optional minimum; the GitLab list reads carry minimum 1 and maximum
100; a minimum above the maximum is refused at load. Returned to the implementor.

```findings
[
  {"file": "adapters/catalog/src/lib.rs", "line": 141, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "a bound carries only a maximum, so per_page of 0, -1 or \"-0\" still reaches GitLab on every bounded list read, where the documented short-page stop rule can never fire"}
]
```
