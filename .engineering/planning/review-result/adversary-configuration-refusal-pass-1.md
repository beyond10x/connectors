---
format: aep.planning-md/3
id: review-result:adversary-configuration-refusal-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the configuration refusal data
relations:
- reviews: story:configuration-refusal-names-the-entry
revision: 1
---
unit: story:configuration-refusal-names-the-entry, uncommitted tree wave0929b-refusal on 361d4dbc7
verdict: NEEDS-CHANGE
cases: executed 306→309, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary
needs-coordinator: four CLI commands load config through owner::selected; in this unit or not

Cases (all red): apps/connectors/tests/configuration_refusal_adversary.rs
every_cli_command_refusing_a_mismatched_entry_names_format_and_entry (4 of 12 commands plain);
crates/connectors-host/tests/configuration_refusal_adversary.rs a_file_invalid_for_another_reason_names_no_mismatched_entry;
crates/connectors-conformance/tests/configuration_refusal_adversary.rs the_contract_refuses_configuration_coordinates_that_are_not_the_declared_ones.

Coordinator routing: all three to the implementor. 1: a CLI Config::read preflight, owner and Failure unchanged.
2: mismatch checked after every other entry check. 3: configuration_format becomes an enum; instance_id keeps
String with an ESS-LIMIT note; any case the contract still cannot refuse comes back for a decision.

```findings
[
  {"file": "crates/connectors-host/src/local/owner.rs", "line": 161, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "operations invoke, approvals policy-status, approvals prepare and connections connect load config in the CLI process via owner::selected -> Config::load and refuse a mismatched entry as plain invalid_configuration without format or instance_id"},
  {"file": "crates/connectors-host/src/local/config.rs", "line": 292, "category": "property", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the mismatch returns inside the per-entry loop, so a file with another invalid entry is named or not depending on alias sort order, contrary to scenarios.md T04 'each otherwise admissible'"},
  {"file": "ess/domains/cli.yaml", "line": 279, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "configuration_format and instance_id are unconstrained Optional<String> on every Failure code, so the contract admits parser text, control characters, 5000-byte ids and outcome_unknown carriers that semantics.md rules out, with no ESS-LIMIT note recording why"}
]
```
