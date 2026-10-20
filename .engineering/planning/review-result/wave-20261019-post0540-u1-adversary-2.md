---
format: aep.planning-md/3
id: review-result:wave-20261019-post0540-u1-adversary-2
kind: review-result
status: active
title: 'U1 adversary pass 2: approval-issue-slack-bot-post'
relations:
- reviews: story:approval-issue-slack-bot-post
revision: 1
---
unit: story:approval-issue-slack-bot-post, at 478aae2e6 plus one new untracked test file in the unit worktree
verdict: red
cases: executed 259→261, red 1
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: ~/.cache/connectors-wave-20261019b/u1/adversary-2/ (case1-red.log, case2.log, suite-connectors.log, probe/)
needs-coordinator: yes. Pass-1 deferred its F5 as INFEASIBLE, blamed on the ESS presentation runtime. That reason is wrong: the refusal is rendered in connectors code, at `apps/connectors/src/local/session.rs:298`. Acceptance line 2 is unmet. `session.rs` is not in the unit's file list.

**1. Diff stat**
```
?? apps/connectors/tests/approval_field_adversary_2.rs
```
No tracked file and no implementation file changed.

**2. Cases added** in `apps/connectors/tests/approval_field_adversary_2.rs`, each run alone first

| Case | Asserts | Now |
|---|---|---|
| `an_approval_business_document_refused_from_a_file_or_stdin_names_input` | `approvals prepare` and `approvals issue` with `--input-file` absent, a directory, or 256 KiB + 2 bytes, and with `--input-stdin` at 256 KiB + 2 bytes: `invalid_input` with `field: "input"` (semantics.md:1135-1137) | red |
| `a_refused_instance_selection_names_adapter_on_the_three_approval_commands_only` | `--connector` alone and `--instance` alone: policy-set, prepare and issue name `adapter`; policy-status, `operations invoke` and `connections describe` have no `field` | green |

Red output, verbatim:
```
left: [("prepare", "absent", String("invalid_input"), Null), ("prepare", "directory", String("invalid_input"), Null), ("prepare", "oversized", String("invalid_input"), Null), ("prepare", "stdin oversized", String("invalid_input"), Null), ("issue", "absent", String("invalid_input"), Null), ("issue", "directory", String("invalid_input"), Null), ("issue", "oversized", String("invalid_input"), Null), ("issue", "stdin oversized", String("invalid_input"), Null)]
```
Probe of the binary, absent file:
`{"error":{"code":"failure","data":{"code":"invalid_input","kind":"usage","next_action":"none","stage":"arguments"}}}`
`--input-json '{'` gives the same envelope plus `"field":"input"`. Both are the host's refusal shape; `cli_surface_minor.rs:634` labels it `owner` (`2/failure/invalid_input`).

**3. Suite:** `cargo test --locked -j 8 --no-fail-fast -p connectors` exit 101: 260 passed, 1 failed (the red case), 10 ignored, 261 executed.

**4. Findings** (commit 478aae2e6)

| # | file:line | Verdict / origin | What was measured |
|---|---|---|---|
| 1 | `apps/connectors/src/local/session.rs:298`, source branch `:149-161` | CONFIRMED / introduced (the contract text) | Every source-read refusal of the business document on prepare or issue loses its `field`: the host reader (`protected::document_until`) returns `invalid_input` with no `field`, and `finish` renders it through `owner_failure`, not `approval_failure`. Fix: set `field = Input` on that reader error for `approval-prepare` and `approval-issue` only, and route those callables through `approval_failure` in `finish`. Covers pass-1 F5's absent-file and directory cases. |
| 2 | `apps/connectors/src/local/select.rs:104` | CONFIRMED / introduced | Before case 2, only `issue` asserted `field: "adapter"` from `refusal_of`; dropping prepare or policy-set from its list, or widening it to every command, stayed green (inferred by grep, not executed). Case 2 pins it. |

**5. Attacked and could not break:** routing (only policy-set, prepare, issue use `approval_failure`); no existing key lost; `operations invoke`, `connections describe`, `approval-keys-*` carry no `field`; `Paths::resolve` and the config read return `invalid_configuration`; the symlink sentence matches `filesystem::directory`; `approval_failure` sets `field` only on `invalid_input`.

```findings
[
  {"file": "apps/connectors/src/local/session.rs", "line": 298, "category": "contract-drift", "severity": "blocker", "verdict": "CONFIRMED", "origin": "introduced", "message": "an_approval_business_document_refused_from_a_file_or_stdin_names_input: approvals prepare/issue refusing an --input-file (absent, directory, over 256 KiB) or oversized --input-stdin answer the host's failure/invalid_input at stage arguments with no field, because session::finish renders the source error through owner_failure and acquire sets no field; semantics.md promises field input for all three document sources, and it is fixable in connectors, so pass-1 F5's INFEASIBLE verdict does not hold"},
  {"file": "apps/connectors/src/local/select.rs", "line": 104, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "a_refused_instance_selection_names_adapter_on_the_three_approval_commands_only (added, green): before it only issue asserted field adapter from refusal_of, so dropping prepare or policy-set from its list or widening it to every command stayed green; mutation inferred by grep, not executed"}
]
```
