---
format: aep.planning-md/3
id: review-result:connection-binding-adversary-r2
kind: review-result
status: active
title: Adversary, round 2, on launching for a description and writes
relations:
- reviews: story:connection-binding-excludes-operations
revision: 1
---
confirmed

Adversarial review, round 2, of the delta on c53353d975 for story:connection-binding-excludes-operations:
launching for a description, writes following an executable-only change, the restored acceptance
journeys. Run 2026-10-18 on the uncommitted tree.

Cases added (test files only): `adv2_launch_admits_only_a_current_usable_connection_of_the_instance`
(green; the unit coverage of `launch_admits`), `adv2_a_connection_whose_identity_the_new_executable_refused_admits_no_launch`
(red) in crates/connectors-host/src/local/registry/binding_scope_tests.rs, and
`adv2_a_stopped_instance_answers_description_unavailable_for_a_new_selection` (red) in
adapters/catalog/tests/local_runtime/revision_recovery.rs.

Attacked and not broken: the launch condition for revoked, Stale, expired, unpublished, another
instance's, no-digest-under-a-new-revision and changed-credential-side connections; no custody read and
no provider contact on a launch; older owners (absent fields default; another build is gated); concurrent
follows (publication fence); the kept public revision (a proof issued before the change is still refused);
the mutation gate after a follow; a write whose new executable answers another identity creates nothing.

Outcomes, recorded separately: F1 fixed (a stopped instance is not launched, owner and CLI side, and
answers description_unavailable); F3 fixed (command table, owner.md, the passive-commands sentence); N1
fixed (the write path holds the control lock until the validation's dispatch is recorded); F2 resolved
by narrowing the contract sentence, its case restated as
`adv2_an_identity_refused_under_a_new_executable_admits_a_launch_but_no_read`; N2 and N3 noted.

```findings
[
{"file":"crates/connectors-host/src/local/owner/transport.rs","line":2069,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"after instances stop the description launch is refused by suppression and describe answers unavailable with retry_status instead of description_unavailable"},
{"file":"crates/connectors-host/src/local/registry/observation.rs","line":137,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"launch_admits admits a connection whose identity the configured executable refused, because an executable-only identity refusal is recorded nowhere, against semantics.md starts nothing"},
{"file":"contracts/cli/v1alpha1/semantics.md","line":97,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the command inventory still says operations list/describe start no host and no adapter, and owner.md omits them from the commands that may start the owner"},
{"file":"crates/connectors-host/src/local/owner/mutation/execution.rs","line":224,"category":"concurrency","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the write path's admit releases the control lock before dispatch_revalidation, contrary to the follow_executable comment"},
{"file":"crates/connectors-host/src/local/owner/mutation/execution.rs","line":215,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the write's executable validation is a credential-bearing provider exchange that runs before the audit is admitted"},
{"file":"apps/connectors/src/local/operations.rs","line":131,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"operation_snapshot can now start the owner and an adapter before the exact-key observation that the comment says precedes any owner start"}
]
```
