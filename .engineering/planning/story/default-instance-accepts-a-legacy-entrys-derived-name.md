---
format: aep.planning-md/3
id: story:default-instance-accepts-a-legacy-entrys-derived-name
kind: story
status: draft
title: default_instance accepts a legacy entry by the derived name the contract gives it
summary: check_selectors checks default_instance by catalog/<alias>, so the contract's jira/main refuses the file
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- informed_by: review-result:wave-b-alias-contract-adversary-1
revision: 1
---
## Defect

Found by `review-result:wave-b-alias-contract-adversary-1`, finding 4. The defect predates this
wave.

`contracts/cli/v1alpha1/semantics.md` says a legacy `[adapters.<alias>]` entry is named by its
alias, under the connector `setup migrate` derives for it (for example `jira/main`). It also says
a `default_instance` that does not name a configured instance refuses the file.

`Config::check_selectors` (`crates/connectors-host/src/local/config.rs:1008`) checks
`default_instance` against `Config::names` (`catalog/<alias>`), not against the derived name.
As a result:

- `default_instance = 'jira/main'`, the name the contract gives that entry, refuses the whole file
  at load;
- `default_instance = 'catalog/main'` is admitted at load and selects the entry.

Deriving the connector needs the app's declarations and native-config reads (`migrate.rs`). The
host crate's load check does not have them.

## Acceptance

- `default_instance` naming a legacy entry by its derived `<connector>/<alias>` is admitted at load
  and selects that entry.
- A name that both a legacy entry and a `[connectors]` instance derive is refused at selection as
  `instance_ambiguous`, as for every other selector.
- `catalog/<alias>` keeps working, or its retirement is stated in the contract.
- The case written by the adversary, `a_default_instance_naming_a_legacy_entry_by_its_derived_name_selects_it`, passes:
  it is the test removed from `apps/connectors/tests/adversary_alias_contract.rs` before the merge.
