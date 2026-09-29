---
format: aep.planning-md/3
id: review-result:adversary-setup-init-format-pass-1
kind: review-result
status: active
title: Adversary pass 1 on setup init format
relations:
- reviews: story:setup-init-writes-current-config-format
revision: 1
---
unit: story:setup-init-writes-current-config-format, uncommitted tree wave0929b-setup on 1d4df135b
verdict: NEEDS-CHANGE
cases: executed 291→297, red 2
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary (suite logs)
needs-coordinator: none

Cases in apps/connectors/tests/setup_init_format_adversary.rs: documented_postgres_entry_passes_setup_check_after_init
and documented_kubernetes_entry_passes_setup_check_after_init (red: setup check refuses the documented entry
with invalid_configuration); documented_catalog_entry_passes_setup_check_after_init,
existing_v1_files_load_and_are_never_rewritten, initialized_empty_configuration_is_byte_stable_apart_from_owner
(green); documented_entries_were_admitted_by_a_v1_initialized_file (origin control, green).

Coordinator routing: both findings fixed by adding `private_protocol = "connectors-private/1"` to the TOML
blocks in docs/local-postgres-cli.md and docs/local-kubernetes-cli.md; the two red cases pass. The origin
control case asserted the base behaviour with the old guide text and was removed once the guides changed.
`cargo test -p connectors` exit 0 afterwards.

```findings
[
  {"file": "docs/local-postgres-cli.md", "line": 65, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The documented adapter entry has no private_protocol, so after setup init now writes connectors-local/2 the documented journey fails setup check with invalid_configuration."},
  {"file": "docs/local-kubernetes-cli.md", "line": 86, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The documented adapter entry has no private_protocol, so after setup init now writes connectors-local/2 the documented journey fails setup check with invalid_configuration."}
]
```
