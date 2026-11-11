---
format: aep.planning-md/3
id: review-result:wave-b-alias-contract-adversary-1
kind: review-result
status: active
title: Adversary 1 on the legacy-alias-sharing-an-instance-name correction (single defect wave)
relations:
- reviews: story:legacy-alias-sharing-an-instance-name
revision: 1
---
unit: story:legacy-alias-sharing-an-instance-name (b90f3b349 on bd8343ae3, tree cb-alias-contract)
verdict: red
cases: executed 66→74, red 5
origin: introduced 5, pre-existing 2, undecided 0
wrote-outside-worktree: <scratch>/units/legacy-alias-sharing-an-instance-name/scratch/adv/ (logs, 3 patch scripts, rr-adv-1.*)
needs-coordinator: yes

1. diff --stat: `apps/connectors/tests/connect_endpoint_adversary.rs | 26 +` (modified), plus untracked `apps/connectors/tests/adversary_alias_contract.rs` (new). Test files only, uncommitted.
2. Cases added. Each was run alone before the suite.
   - connect_endpoint_adversary `an_endpoint_on_a_name_a_legacy_entry_holds_writes_nothing`. RED: the file gained `[connectors.gitlab.instances.ref-1] endpoint = "https://gitlab.example.test"`, and the connect answered `{"code":"instance_ambiguous","instances":["gitlab/ref-1 (alias gitlab.ref-1)","gitlab/ref-1 (alias ref-1)"],...}`.
   - adversary_alias_contract `an_admitted_default_instance_still_selects_its_entry`. RED: `{"code":"not_found","kind":"operational","next_action":"check_configuration","stage":"admission"}`.
   - `every_command_taking_connector_and_instance_refuses_a_shared_name` (18 commands). RED for `connections connect --credential-file` and `connections reconnect --credential-file`: `{"code":"invalid_input","kind":"usage","next_action":"none","stage":"arguments"}`, exit 2, no `instances`.
   - `a_connect_target_naming_a_shared_name_is_refused_before_the_credential`. RED: `left: "invalid_input" right: "instance_ambiguous"`.
   - `a_default_instance_naming_a_legacy_entry_by_its_derived_name_selects_it`. RED, pre-existing: `{"code":"invalid_configuration",...,"stage":"configuration"}`.
   - Green: `a_write_through_a_shared_name_is_refused_at_selection`, `the_entries_beside_a_shared_name_are_selected_as_before`, `a_default_instance_without_a_shared_name_selects_as_before`.
3. Suite: cargo-unit.sh test --locked --no-fail-fast -p connectors with --test adversary_d218_u1, adversary_d218_u3, connector_instance_selectors, connector_instance_selectors_adversary, invoke_short, short_form_adversary, connect_endpoint_adversary and adversary_alias_contract. EXIT 101. u1 12 ok (1 ignored), u3 13 ok, selectors 5 ok, selectors_adversary 1 ok, invoke_short 21 ok, short_form_adversary 4 ok, connect_endpoint_adversary 10 passed 1 failed, adversary_alias_contract 3 passed 4 failed. The before count is 66, the same targets without my cases.
4. Findings:
   - endpoint.rs:120, blocker, introduced. The `--endpoint` write runs before selection. On a name that only a legacy entry holds (the docs' own `gitlab/forge` example), it creates the second entry and then refuses. The file stays changed, and the legacy name that worked is now ambiguous for every command. Reached by `connections connect <c>/<i> --endpoint`, the documented way to give an instance its endpoint. Fix: refuse a name that a legacy entry derives before `write`.
   - select.rs:170, blocker, introduced. Reached by a hand-written `default_instance='catalog/<alias>'`, which is still admitted at load and selected the entry before this commit. Configurations with no shared name changed behaviour: they now answer not_found.
   - select.rs:82 together with session.rs:220, warning, introduced. On connect and reconnect with a credential source, protected-source acquisition refuses first and gives `invalid_input` instead of `instance_ambiguous`. That is contract drift against the unit's own semantics.md text. No credential is read, because selection comes before `admit_capture`.
   - config.rs:1008, warning, pre-existing. The load check and the selection disagree on a legacy entry's name: `jira/main` is refused at load, and `catalog/main` is admitted at load but now not_found at selection.
   - invoke.rs:224, note, introduced. The `(no entry)` form is not in the contract. Untested: it needs a derived instance that awaits a profile.
   - config.rs:1015, note, introduced. The claimed `default_instance` to profile-unnamed side effect is unreachable, because the file is refused at load first. Nothing tests it.
   - migrate.rs:522, note, pre-existing. `setup migrate` names a kept legacy entry `catalog/<alias>`, and no selector reaches that name.
5. Attacked, not broken:
   - `invoke` read and write with or without an instance, `--describe`, and the dynamic validator.
   - 16 other `--connector`/`--instance` commands: approvals policy-status, policy-set, prepare and issue; instances describe, status and stop; connections list, describe, status, revalidate, token and revoke; connect without a credential; operations list and describe.
   - The MCP server, which selects by alias, not by name.
   - `--adapter` on either entry.
   - The other instance beside a shared name.
   - `default_instance` and several instances without a shared name.
   - Docs: semantics.md and website cli.md are identical; local-catalog-provider.md agrees except on the endpoint write.
6. Outside the worktree: the scratch directory above holds build.log, red-cases.log, red-cases-2.log, red-endpoint.log, red-derived.log, suite.log, add_endpoint_case.py, fix_args.py, add_derived_default.py, rr-adv-1.json and rr-adv-1.md. Test tempdirs were under the worktree's .local/tmp.
7. findings: rr-adv-1.json

```findings
[
{"file":"apps/connectors/src/local/endpoint.rs","line":120,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"connections connect gitlab/ref-1 --endpoint <url> on a name only a legacy entry holds writes [connectors.gitlab.instances.ref-1] and then refuses as instance_ambiguous, so a refused command leaves the file changed and makes the name it refuses; red: connect_endpoint_adversary an_endpoint_on_a_name_a_legacy_entry_holds_writes_nothing"},
{"file":"apps/connectors/src/local/select.rs","line":170,"category":"boundary","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"no shared name: default_instance='catalog/main' naming a legacy catalog entry that derives jira is still admitted at load (Config::check_selectors) and selected it before b90f3b349, and now every selector-less command answers not_found; red: adversary_alias_contract an_admitted_default_instance_still_selects_its_entry"},
{"file":"apps/connectors/src/local/select.rs","line":82,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"connections connect and reconnect with --connector/--instance (or the connect target) and a credential source answer invalid_input without instances, not instance_ambiguous, because session.rs:220 refuses through select::alias_at and Refused::Ambiguous maps to Code::InvalidInput; no credential is read; red: adversary_alias_contract every_command_taking_connector_and_instance_refuses_a_shared_name and a_connect_target_naming_a_shared_name_is_refused_before_the_credential"},
{"file":"crates/connectors-host/src/local/config.rs","line":1008,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"check_selectors validates default_instance through Config::select (catalog/<alias>), while selection now uses the derived name, so default_instance='jira/main' for a legacy entry the contract names jira/main refuses the whole file; red: adversary_alias_contract a_default_instance_naming_a_legacy_entry_by_its_derived_name_selects_it"},
{"file":"apps/connectors/src/local/invoke.rs","line":224,"category":"contract-drift","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"a shared name whose [connectors] instance awaits a profile is listed as '<connector>/<instance> (no entry)', a form semantics.md does not name (it says '(alias <alias>)' for both); not tested, needs a derived instance with several profiles"},
{"file":"crates/connectors-host/src/local/config.rs","line":1015,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the claimed side effect (default_instance on an instance awaiting a profile now answers invalid_configuration via ProfileUnnamed) is unreachable: check_selectors refuses such a file at load because Config::select finds no entry and deferred() excludes awaiting instances; no test asserts it"},
{"file":"apps/connectors/src/local/migrate.rs","line":522,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"setup migrate reports a kept legacy catalog entry (instance_conflict, the remedy the new docs point to) as connector catalog / instance <alias>, a name no selector reaches once a connector is derived for it"}
]
```

