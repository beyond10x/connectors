---
format: aep.planning-md/3
id: review-result:adversary-absent-operation-pass-1
kind: review-result
status: active
title: Adversary pass 1 on absent operation not_found
relations:
- reviews: story:absent-operation-reports-not-found
revision: 1
---
unit: story:absent-operation-reports-not-found, uncommitted tree wave0930b-absent on 62aec7648
verdict: CONFIRMED (2 red cases; both divergences exist at base; the unit's acceptance holds)
cases: executed 328→331, red 2
origin: introduced 0 / pre-existing 2 / undecided 0
wrote-outside-worktree: scratch/adversary
needs-coordinator: describe vs invoke when the profile is not granted; a registry test failed once under parallel load

Cases in apps/connectors/tests/absent_operation_adversary.rs: describe agrees with invoke when the operation's profile
is not granted (red), absent id is not_found everywhere when the profile is not granted (green), an ill-formed id gets
one answer from every verb (red). Could not break: the removed Invoke pre-check (admit_invoke and the supervisor
re-check), writes and guarded writes, a config change between admission and dispatch, the existence source, list
filtering, error data.

Coordinator routing: both findings fixed in this unit (describe uses admit_operation and validates the id; list hides
operations whose profile is not granted). The parallel-load failure of the grown-store replay test is filed as
story:grown-store-replay-test-flake.

```findings
[
  {"file": "apps/connectors/src/local/operations.rs", "line": 54, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "describe checks only the operation grant, so an operation whose profile is not granted describes successfully while invoke and prepare answer forbidden for the same id"},
  {"file": "apps/connectors/src/local/operations.rs", "line": 53, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "describe does not validate the operation id format and answers not_found where invoke and prepare answer invalid_input exit 2 for the same ill-formed id"}
]
```
