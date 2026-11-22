---
format: aep.planning-md/3
id: review-result:wave-c-body-null-adversary-2
kind: review-result
status: active
title: Adversary pass 2 on required-body-key-refuses-null
relations:
- reviews: story:required-body-key-refuses-null
revision: 1
---
unit: story:required-body-key-refuses-null
verdict: green
cases: executed 169→170, red 1
origin: introduced 4, pre-existing 0, undecided 1
wrote-outside-worktree: <scratch>/units/required-body-key-refuses-null/scratch/{adv_case.rs,adv2-case1.log,adv2-case1b.log,adv2-suite-provider.log,adv2-suite-catalog.log,adv2-suite-build.log,adv2-suite-nested.log}, …/required-body-key-refuses-null/adv2.md
needs-coordinator: yes

Covers c1a507803a plus the uncommitted test below. Diff: `adapters/catalog/tests/jira_issue_writes_adversary.rs | 46 +++` (test file only).
Case added: `adversary_edit_schema_and_prepare_agree_on_every_body` checks that the issue.edit input schema and `prepare` agree on 8 bodies. It is RED. Run alone (cargo-unit.sh … --test jira_issue_writes_adversary -- adversary_edit…): `left: ["body {}: declared true, prepare not-attempted InvalidInput"]`, `test result: FAILED. 0 passed; 1 failed`, exit 101. The other 7 bodies agree: fields {} / null, lone nulls, summary/labels null, summary "".
Suite, run after the case existed: provider body_types 5 ok, connector_declarations 12 ok, gateway_prefix 7 ok, jira_issue_writes 11 ok, jira_issue_writes_adversary 16 ok 1 FAILED (mine, exit 101), nested_closed_body 10 ok. catalog definition_import 43 ok, definition_writes 20 ok, definition_writes_adversary 38 ok (exit 0). build catalog_guard_model 7 ok (exit 0). `before` = the same run without my one case.

1. closed.rs:525 (undecided, note). The anyOf of a nested `body_any_of` sits on its parent object, and nothing requires that parent. issue.edit `body: {}` is valid by the declared schema and refused by prepare as invalid_input before any request. The flag plays no part, and the lines match 9b97114df1, but I could not run it there. Reached by any caller of `issue.edit`; nothing is sent. Fix: require the parent segment at each level above `any_of`.
2. semantics.md:279 §7 (introduced, warning, contract drift). There is no entry for the new optional `body_null_clears`. An older reader refuses it as an unknown field, and the Jira selection now carries it. There is also no entry saying a required or any-of key given as `null` now counts as missing for every provider.
3. catalog-jira.md:301, local-catalog-provider.md:1055 (introduced, note). Both docs say a member or key given as `null` counts as the change and is sent. A typed member is still refused: `summary: null` and `labels: null` are "not of its declared type" in schema and prepare alike, which my case checks. Narrow the text to untyped members.
4. connector_declarations.rs:802 (introduced, note, needs coordinator). Every Jira instance's configuration revision moves (pins 12/13/14 and gateway_prefix). After upgrade a pinned instance does not start until `configuration_revision` is re-copied. A connected one is `pending` until `connections revalidate`. The CHANGELOG entry must say so. The test's doc comment lists the reasons pins moved and does not name this one; gateway_prefix.rs does.
5. definition.rs:1672 (introduced, note, mutant). `body_null_clears: false` with a valid `body_any_of` is refused with "…expected at least two distinct keys under one parent", which is misleading. No test covers either definition-side refusal: deleting the `Some(_)` arm silently drops the flag and the suite stays green. The new any-of refusal text (closed.rs:472) is pinned by no test either.
Could not break: the new cases go red with the flag reverted, whether in check(), schema(), operations.json or import/lower (definition_import jira round trip). The flag does not leak to required keys, the required-plus-any_of combination or the nested Jira body. Engine load refuses a flag without any_of (closed.rs:300), and so do definition load (:1666), the import no-body_keys check, check_download and the ESS invariant. Describe and the declared schema follow Closed::schema. No stale revision pin remains anywhere.

```findings
[{"file":"adapters/catalog/src/closed.rs","line":525,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"undecided","message":"a nested body_any_of parent is not required, so issue.edit body {} passes the declared schema and is refused by prepare"},
 {"file":"contracts/catalog/v1alpha1/semantics.md","line":279,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"section 7 names neither the new optional body_null_clears member nor the null-counts-as-missing rule for required and any-of keys"},
 {"file":"docs/local-catalog-provider.md","line":1055,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"docs say a null any-of key counts and is sent, but a typed key given as null is refused (summary, labels in catalog-jira.md:301 too)"},
 {"file":"adapters/catalog/tests/connector_declarations.rs","line":802,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"every Jira instance's configuration revision moves, which needs a CHANGELOG re-pin and revalidate note, and the pin comment omits the reason"},
 {"file":"crates/connectors-catalog/src/definition.rs","line":1672,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"definition-side body_null_clears refusals are untested and the false case reports a misleading any_of detail; the new refusal text at closed.rs:472 is unpinned too"}]
```
