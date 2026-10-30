---
format: aep.planning-md/3
id: approval-record:d261-verify-release-override-0600
kind: approval-record
status: approved
title: 'D261: verify-release override for two v0.60.0 test changes'
relations:
- decides: story:provider-ignored-tests-run-in-ci
- decides: story:connectors-do-short-invocation
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-10-30T05:34:03Z", actor: "human:timo", revision: 3}
---
## Approval

On 2026-10-30 the directing session `b10x-chef-orchestrator` decided item D261 under its grant:
"both justified; I record the override." `verify-release connectors v0.60.0` flagged two test
changes in the release diff, and D261 accepts both:

1. `describe_of_a_definition_needs_no_connection` (`apps/connectors/tests/adversary_d218_u1.rs:687`)
   is `#[ignore]` and needs a built catalog provider. It runs under `connectors-build ignored`,
   which no CI job runs yet. It passed by hand against v0.60.0.
   story:provider-ignored-tests-run-in-ci (proposed) carries it into the next connectors wave.
2. `root_help_after_a_global_output_carries_the_service_block` is inverted to
   `root_help_after_a_global_output_carries_no_service_block`
   (`apps/connectors/tests/adversary_legacy_edges.rs:42`), because 0.60.0 removes the service
   commands (`contracts/cli/v1alpha1/semantics.md:282`).

The CHANGELOG 0.60.0 section states both. Nothing is amended or retagged.
