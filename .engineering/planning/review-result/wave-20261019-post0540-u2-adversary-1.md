---
format: aep.planning-md/3
id: review-result:wave-20261019-post0540-u2-adversary-1
kind: review-result
status: active
title: 'U2 adversary pass 1: write-postflight-present-check'
relations:
- reviews: story:write-postflight-present-check
revision: 1
---
unit: story:write-postflight-present-check
verdict: red
cases: executed 1128→1136 (provider 768→772, catalog 360→364), red 1
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: 9 paths, all in the assigned scratch `~/.cache/connectors-wave-20261019b/u2/adversary-1/` (listed in part 6)
needs-coordinator: yes. The `connectors-definitions` mirror in acceptance bullet 2 is not done, and it is cross-repo.

**1. Diff stat.** `git --no-pager diff --stat` is empty: no tracked file changed. `git status --short` shows two untracked files, both tests:
```
?? adapters/catalog/tests/write_postflight_present_adversary.rs
?? crates/connectors-catalog/tests/definition_present_adversary.rs
```
No implementation file was touched.

**2. Cases added.** Each was run alone first.

| File::case | Asserts | Now |
|---|---|---|
| provider `a_preflight_presence_refuses_an_absent_member_as_it_refuses_a_null_one` | a `present` check before the write: absent member and null member give the same code (`Forbidden`, or `Conflict` under `mismatch: conflict`) | **red** |
| provider `a_presence_after_a_read_after_the_write_reads_the_reread_answer` | `present` with `postflight.read`: present → applied, null or absent → unknown | green |
| provider `a_presence_reads_nested_and_array_pointers` | `/issues/0/id`, `/meta/key`: empty array, null member and wrong parent type → unknown; `{}` value → applied | green |
| provider `a_presence_of_false_is_refused_in_any_of_and_further_preflights` | `present:false` is refused at load in `any_of` and in `further_preflights` | green |
| catalog `a_question_mark_lowers_to_a_presence_in_before_after_and_any` | `"?"` → present in before/after/any; `"=?"` → literal `?` | green |
| catalog `an_unquoted_question_mark_never_lowers_to_another_expectation` | unquoted YAML `?` (flow and block) is refused or lowers to present, never to an input or a literal | green |
| catalog `an_import_refuses_an_input_reference_named_question_mark` | the importer refuses `{"input":"?"}` | green |
| catalog `a_presence_in_a_preflight_and_any_of_round_trips_through_the_importer` | import then lower of a preflight and `any_of` presence gives back an identical selection | green |

The red output, from the run of that file alone (`red-provider.log`):
```
assertion `left == right` failed: assignee absent, no mismatch
  left: Some(UpstreamProtocol)
 right: Some(Forbidden)
test result: FAILED. 3 passed; 1 failed; ... EXIT=101
```
The catalog file alone: `test result: ok. 4 passed; 0 failed`, EXIT=0.

**3. Suite runs** (all after the cases existed; `-j 8`, TMPDIR at `<worktree>/.local/tmp`):

| Command | Result | Exit |
|---|---|---|
| `cargo test --locked -j 8 -p connectors-catalog-provider --no-fail-fast` | 771 passed, 1 failed (`write_postflight_present_adversary`), 64 ignored | 101 |
| `cargo test --locked -j 8 -p connectors-catalog --no-fail-fast` | 364 passed, 0 failed | 0 |
| `cargo clippy --locked -j 8 -p connectors-catalog-provider -p connectors-catalog --all-targets -- -D warnings` | clean | 0 |
| `cargo fmt -p connectors-catalog-provider -p connectors-catalog -- --check` | clean | 0 |
| `cargo test --locked -j 8 -p connectors-build --test catalog_guard_model` | 7 passed | 0 |

**4. Findings** (covering commit 540e05a80 plus the two untracked tests):

| Finding | Where | What reaches it | Verdict / origin |
|---|---|---|---|
| A `present` check before the write sends an absent member to `UpstreamProtocol` and a null member to `Forbidden`/`Conflict`. Suggested fix (not applied): `Pinned::Present => true` in `Pinned::present`. Only the preflight calls that function. | `adapters/catalog/src/lib.rs:881` | No shipped selection has a preflight presence. The definition guide offers `"?"` in `before.expect`. | INFEASIBLE / introduced, warning |
| Acceptance bullet 2 says "mirrored in the connectors-definitions repository". In `connectors-definitions` at `origin/night/20261019` (1ed41fb), `connectors/jira.yaml` has no `after` on `issue.create` and only `/body/type: =doc` on `issue.comment.add`. An older lowerer would read `"?"` as an input reference (code reading, not run). | `crates/connectors-catalog/tests/fixtures/definitions/connectors/jira.yaml` vs the definitions repository | the acceptance statement | NEEDS-CHANGE / introduced, warning |
| `present` holds for any non-null value, so a create answered 201 `{"id":"","key":""}` reports applied. This is the documented contract. | `adapters/catalog/src/lib.rs:891` | no documented Jira answer | CONFIRMED / introduced, note |

**5. Attacked and could not break:** null, absent and `{}` in the postflight; nested and array pointers; postflight with `read`; `present:false` refused at load in `checks`, `any_of` and `further_preflights`; `"?"` and `"=?"` lowering; unquoted `?`; importer refusal of `{"input":"?"}`; round trip; guard model conformance; other Jira writes (`issue.link.add` answers 201 with no body and no id); revision re-pins (only the three Jira pins and the gateway prefix moved).

**6. Paths written outside the worktree:** `~/.cache/connectors-wave-20261019b/u2/adversary-1/` (`run.sh`, `red-provider.log`, `red-catalog.log`, `suite-provider.log`, `suite-catalog.log`, `clippy.log`, `fmt.log`, `guard-model.log`); the shared compile cache `~/.cache/sccache/connectors`.

**7. Findings block**


```findings
[
  {"file": "adapters/catalog/src/lib.rs", "line": 881, "category": "boundary", "severity": "warning", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A present check before the write refuses an absent member as upstream_protocol but a null member as forbidden or conflict, though both fail the same comparison (test: adapters/catalog/tests/write_postflight_present_adversary.rs::a_preflight_presence_refuses_an_absent_member_as_it_refuses_a_null_one)"},
  {"file": "crates/connectors-catalog/tests/fixtures/definitions/connectors/jira.yaml", "line": 78, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Acceptance requires the connectors-definitions repository to mirror the checks; its jira.yaml (origin/night/20261019 1ed41fb) still lacks them, so the in-tree copy now differs from its source (no test)"},
  {"file": "adapters/catalog/src/lib.rs", "line": 891, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "present holds for an empty string, false, 0, {} and [], so a create answered with an empty id and key reports applied, as documented (test: adapters/catalog/tests/write_postflight_present_adversary.rs::a_presence_reads_nested_and_array_pointers)"}
]
```
