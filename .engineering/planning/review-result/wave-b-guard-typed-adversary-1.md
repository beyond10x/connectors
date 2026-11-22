---
format: aep.planning-md/3
id: review-result:wave-b-guard-typed-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on catalog-guard-typed-comparison (single defect wave)
relations:
- reviews: story:catalog-guard-typed-comparison
revision: 1
---
```
unit: story:catalog-guard-typed-comparison, commit 8f11ed6fc plus one uncommitted test addition in worktree cb-guard-typed
verdict: green
cases: executed 16→17, red 1
origin: introduced 1, pre-existing 1, undecided 1
wrote-outside-worktree: <scratch>/units/catalog-guard-typed-comparison/adv/{number_case.rs,case.log,suite.log,suite2.log,fresh.log}
needs-coordinator: yes
```

1. Diff stat: `adapters/catalog/tests/adversary_gitlab_mr_writes.rs | 44 +++`, test file only. `touch -m` on `adapters/catalog/src/lib.rs` (content unchanged, see 3).
2. Case added: `adversary_a_number_input_matches_only_a_json_number` on `project.create` (`body.namespace_id` typed `integer`): a preflight answering `/id: "42"` for input `42` refuses, and a postflight answering `/namespace/id: "42"` is not applied. Red alone, exit 101: `["preflight /id \"42\" held for namespace_id 42", "postflight /namespace/id \"42\" applied for namespace_id 42"]` / `test result: FAILED. 0 passed; 1 failed; ... 5 filtered out`.
3. Suite `cargo test --locked -p connectors-catalog-provider --no-fail-fast --test adversary_gitlab_mr_writes --test gitlab_mr_writes`, exit 101: adversary 5 passed, 1 failed (the case above); gitlab_mr_writes 11 passed. With `--skip adversary_a_number_input_matches_only_a_json_number`: 5 + 11 passed, exit 0. The first suite run reported the unit's own `a_resolve_guard_compares_booleans_by_json_type` failed (`requested true, answered "true"`): every worktree of the wave builds into one target directory and cargo treated another tree's newer build of the crate as fresh; after `touch -m` on lib.rs the crate rebuilt from this tree and the test passed.
4. Findings:
   - Number and string inputs still compare as text (`adapters/catalog/src/lib.rs:2016`), against the story Outcome "a number only a number, a string only a string". The Acceptance bullets cover booleans only and hold. What reaches it: nothing found; GitLab answers namespace ids as integers. A global number arm would break path-parameter checks (Drive `/version`, `pipeline_id`, Zendesk `ticket_id`); if typed, limit it to body keys `body_types` types `integer`.
   - Stale model prose (`adapters/catalog/spec/ess/domains/guard.yaml:26`, `:62`): still says `input` and `literal` compare as text, and only a `boolean` expectation compares by JSON type.
   - Wave build infrastructure: the one shared `CARGO_TARGET_DIR` lets cargo link another tree's build of the same crate, so a unit's result may measure another tree's code.
5. Attacked and not broken: reverting the Bool arm turns the unit's test red; mutating it to `Boolean(true)` or `!flag` is caught; a string `"true"` as `resolved` input is refused `invalid_input` by `body_types`; the resolve postflight with `1`, `null`, `0` or `"false"` is unknown; no other shipped guard has a boolean input; the docs paragraph matches the code.

```findings
[
{"file":"adapters/catalog/src/lib.rs","line":2016,"category":"acceptance","severity":"warning","verdict":"INFEASIBLE","origin":"undecided","message":"An input that names a JSON number still compares as text, so project.create accepts a string \"42\" for namespace_id 42 before and after the write, against the story Outcome 'a number only a number'."},
{"file":"adapters/catalog/spec/ess/domains/guard.yaml","line":26,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The model still says input and literal compare as text (line 26), and the Check comment at line 62 says only a boolean expectation compares by type; both are now false for a boolean input."},
{"file":"wave build environment","line":null,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"The one CARGO_TARGET_DIR shared across worktrees lets cargo treat another tree's newer build of the same crate as fresh, so a unit's test run can link code that is not in its tree (the unit's own test went red until lib.rs was touched)."}
]
```
