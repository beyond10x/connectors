---
format: aep.planning-md/3
id: review-result:wave-b-alias-contract-adversary-2
kind: review-result
status: active
title: Adversary 2 on the legacy-alias-sharing-an-instance-name correction (single defect wave)
relations:
- reviews: story:legacy-alias-sharing-an-instance-name
revision: 1
---
unit: story:legacy-alias-sharing-an-instance-name (b90f3b349 + 5b9131c73 on bd8343ae3, tree cb-alias-contract)
verdict: green
cases: executed 139→142, red 2
origin: introduced 2, pre-existing 2, undecided 0
wrote-outside-worktree: <scratch>/adv2/ (case_a.rs, case_c.rs, add_endpoint_case.py, red.sh, red_c.sh, suite.sh, red.log, red_c.log, suite.log, rr-adv-2.json, rr-adv-2.md)
needs-coordinator: yes

Green means no finding blocks the Acceptance. The two red cases are warnings. The coordinator decides whether to route them (F1 to the implementor, F2 out of the wave).

1. diff --stat (uncommitted, test files only): `apps/connectors/tests/adversary_alias_contract.rs | 147 +`, `apps/connectors/tests/connect_endpoint_adversary.rs | 31 +`.
2. Cases added. Each was run alone before the suite.
   - `adversary_alias_contract::a_shared_name_reads_no_document_before_its_refusal`. On a shared name, each command gets a document path that does not exist and must answer instance_ambiguous. RED: `["approvals","prepare"] exit Some(2): {"code":"invalid_input","field":"input",...,"stage":"arguments"}`, and the same for `["approvals","issue"]`. `invoke jira/main --approval-file`, `invoke jira`, `approvals policy-set` and `connections connect --credential-file` answer instance_ambiguous.
   - `adversary_alias_contract::a_legacy_entry_is_connected_by_its_name_without_an_endpoint`. RED: `["jira/main"]` answered `{"code":"invalid_input","field":"endpoint","stage":"arguments"}`. `--adapter main` gets past that check.
   - `connect_endpoint_adversary::an_endpoint_beside_legacy_entries_of_other_names_is_written`. GREEN. It catches a `held_by_legacy` mutant that compares only the connector or only the instance: legacy `ref-1` derives jira, legacy `other` derives gitlab, and `gitlab/ref-1 --endpoint` is still written. I did not build the mutant.
3. Suite: `cargo-unit.sh <tree> test --locked --no-fail-fast -p connectors` with --test adversary_d218_u1, u2 and u3, connector_instance_selectors and its adversary, invoke_short and its adversary, local_cli, setup_migrate, setup_migrate_short, short_form_adversary, connect_endpoint_adversary and adversary_alias_contract. EXIT 101.
   - adversary_alias_contract: 6 passed, 2 failed (my two).
   - All others pass: u1 12 (1 ignored), u2 16, u3 13, connect_endpoint_adversary 12, selectors 5, selectors_adversary 1, invoke_short 21, invoke_short_adversary 6, local_cli 33 (1 ignored), setup_migrate 9, setup_migrate_short 2, short_form_adversary 4.
   - The before count, 139, is the implementor's reported counts.
   - After the suite, rustfmt re-wrapped one `for` header in my case C. Layout only, not re-run.
4. Findings:
   - F1, session.rs:185. warning, CONFIRMED, introduced (contract text). On approvals prepare and issue, the approval branch of `acquire` reads `--input-file` before selection. semantics.md:761-765 (b90f3b349) extends "instance_ambiguous ... Nothing was read beyond the configuration" to `--connector`/`--instance` on every command. Reached by `approvals prepare|issue --connector c --instance i --input-file <path>` on a shared name. The Acceptance holds: the document is not secret, no credential is read and no request is sent. Fix: resolve `select::alias` before the approval-document branch, or narrow the sentence.
   - F2, endpoint.rs:141 `required`. warning, CONFIRMED, pre-existing. A name only a legacy entry holds is "not listed", so endpoint is required. Since 5b9131c73, `--endpoint` is refused on that name too (endpoint.rs:126). So `connections connect jira/<alias>` on a pinned Atlassian legacy entry works only through `--adapter`, against docs:1389 "they keep working". At base, `--endpoint` instead created a shadowing instance. Reached by any legacy entry of a connector with no `default_endpoint` (jira). The Acceptance holds.
   - F3, select.rs:179. note, pre-existing. The fallback lets `default_instance='catalog/<alias>'` select the legacy entry, even when that entry is half of a shared name. `--connector catalog --instance <alias>` answers not_found or legacy_entry_unmigrated, against semantics.md:764 "select by the same rule". The entry it reaches is the one named, so the story outcome holds. The Unmigrated arm has no test. Reached only by a hand-written default; `setup migrate` copies the defaults unchanged. Overlaps story:default-instance-accepts-a-legacy-entrys-derived-name.
   - F4, local.rs:756. note, introduced. `instances list` shows a shared name as two items with the same connector/instance and no alias. Only the refusal names the alias the operator must rename.
5. Attacked, not broken:
   - Pass-1 #1: `--endpoint` on a legacy-held name, by target or flags, leaves the file unchanged.
   - Pass-1 #2: an admitted `catalog/<alias>` default selects again.
   - Pass-1 #3: connect and reconnect with `--credential-file` answer instance_ambiguous, with selection before `admit_capture`.
   - Pass-1 #5: no "(no entry)" form remains in src or contracts.
   - The fallback with `default_instance` set to the shared name stays refused (Ambiguous is not in the fallback arm; u1 tests it).
   - `held_by_legacy`:
     - No false positive on a name only an instance holds, or beside legacy entries of another connector or instance.
     - Case is compared exactly, as selection does.
     - An unreadable config skips the guard, and `write` then refuses it itself.
   - `invoke --approval-file` on a shared name is refused before the file is read.
   - `reconnect_revision` drops out on a shared name (`.ok()?`).
6. Outside the worktree: `<scratch>/adv2/` holds case_a.rs, case_c.rs, add_endpoint_case.py, red.sh, red_c.sh, suite.sh, red.log, red_c.log, suite.log, rr-adv-2.json and rr-adv-2.md. Test tempdirs went to `<tree>/.local/tmp`.
7. findings: rr-adv-2.json

```findings
[
  {"file": "apps/connectors/src/local/session.rs", "line": 185, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "approvals prepare and issue with --connector/--instance on a shared name read --input-file before selection and answer invalid_input field=input when the file is missing, yet semantics.md:761-765 (b90f3b349) says that selection answers instance_ambiguous with nothing read beyond the configuration."},
  {"file": "apps/connectors/src/local/endpoint.rs", "line": 141, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "connections connect jira/<alias> (or --connector/--instance) on a pinned legacy entry whose connector has no default endpoint is refused as invalid_input field=endpoint, --endpoint is now refused for that name too, so only the deprecated --adapter connects it, against docs/local-catalog-provider.md:1389 'they keep working'."},
  {"file": "apps/connectors/src/local/select.rs", "line": 179, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "default_instance='catalog/<alias>' selects the legacy entry through the Config::select fallback (also when it is half of a shared name) while --connector catalog --instance <alias> answers not_found or legacy_entry_unmigrated, contradicting semantics.md:764 'select by the same rule'; the Unmigrated arm of the fallback has no test."},
  {"file": "apps/connectors/src/local.rs", "line": 756, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "instances list gives a shared name as two items with the same connector and instance and no alias, so only the instance_ambiguous refusal tells the operator which alias to remove or rename."}
]
```
