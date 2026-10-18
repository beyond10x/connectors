---
format: aep.planning-md/3
id: review-result:connection-binding-adversary-r1
kind: review-result
status: active
title: Adversary, round 1, on the credential binding change
relations:
- reviews: story:connection-binding-excludes-operations
revision: 1
---
needs-change

Adversarial review of the green change for story:connection-binding-excludes-operations, on the
uncommitted tree of branch unit/connection-binding-excludes-operations (base 82a681bc4), run
2026-10-18.

Cases added (test files only): three in crates/connectors-host/src/local/registry/binding_scope_tests.rs
(`adv_a_stale_connection_stays_repairable_after_an_executable_only_change`,
`adv_a_credential_refused_under_a_new_executable_leaves_a_repair`,
`adv_an_executable_identity_mismatch_does_not_send_connect_back_to_revalidation`), and two journeys in
adapters/catalog/tests/local_runtime/revision_recovery.rs
(`adv_the_added_read_succeeds_after_a_selection_only_change_without_revalidation`,
`adv_a_read_under_a_replaced_executable_validates_once_and_is_served`). Each was red when first run
alone except the last, which reaches the owner's automatic validation and is green.

Attacked and not broken: the classification with one side absent, both present and different, and
an absent configured executable (consumer launch); the catalog credential-side digest (the `auth`
block, `hosts` credential placement, token CA and request prefix are inside it; a selection's
per-operation `credential` naming strips a parameter and does not route the credential); the ER copy
and projection of both new fields; the pin mismatch check; the refusal record for the operation-only
and executable-only classes.

Outcomes, recorded separately: B, C and D fixed (repair and connect admit an executable-only change);
A escalated to the coordinator as a decision; E noted.

```findings
[
{"file":"adapters/catalog/tests/local_runtime/revision_recovery.rs","line":727,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"after a selection-only change the added read answers description_unavailable until an admitted launch, and the only launch for a held ready connection is connections revalidate, which the acceptance excludes"},
{"file":"crates/connectors-host/src/local/registry/lifecycle.rs","line":58,"category":"boundary","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"begin_repair refused Follow::Executable, so a stale connection could not be reconnected after a provider executable change, where before D12 it could"},
{"file":"crates/connectors-host/src/local/registry/revalidation.rs","line":524,"category":"boundary","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a credential refused under a new executable records nothing and repair answered UpgradeRequired, so revalidate and reconnect looped with revoke as the only exit"},
{"file":"crates/connectors-host/src/local/registry/lifecycle.rs","line":530,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"an executable identity mismatch names create_connection, and connect then answered UpgradeRequired, sending the operator back to the revalidation that had just failed"},
{"file":"crates/connectors-host/src/local/owner/mutation/execution.rs","line":200,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"writes do not run the automatic validation, so a write after an executable-only change is refused with revalidate_connection on a connection listed ready"}
]
```
