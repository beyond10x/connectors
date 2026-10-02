---
format: aep.planning-md/3
id: story:postgres-unsupported-feature-classification
kind: story
status: active
title: Preserve PostgreSQL unsupported-feature refusal instead of reporting an outage
relations:
- informed_by: story:postgres-real-provider-acceptance
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/sql/contracts/reads/v1alpha1/semantics.md
- confidence: cited
  path: adapters/sql/src/lib.rs
- confidence: cited
  path: adapters/sql/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: adapters/sql/tests/protocol.rs
- confidence: cited
  path: docs/local-postgres-cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T15:51:24Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}, correlation: "wave-20261002d-provider-acceptance"}
- {from: "proposed", to: "active", at: "2026-10-02T15:51:24Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}, correlation: "wave-20261002d-provider-acceptance"}
---
## Problem and reproduced caller

The new real-provider read-only acceptance sends a data-modifying CTE through the
production CLI, Sql adapter, reader role and read-only transaction. PostgreSQL17.6
rejects the adapter's wrapped prepare with SQLSTATE0A000. The CLI reports
unavailable/readiness because adapters/sql/src/lib.rs:409-417 does not classify
that code. The child remains ready and a subsequent SELECT1 succeeds. This is
reachable from the supported query.read caller with an unsupported query, not a
database outage. The query never writes the fixture database.

The full acceptance loop failed twice, and a fresh first-CTE variant failed too,
excluding a dependency on the preceding invalid_input result. The owned reader-role
original/wrapped prepare diagnostic captured0A000; native mapping defaults it to
Unavailable while the private bridge preserves Unsupported
(crates/connectors-host/src/local/runtime.rs:207-211). The diagnostic is retained
as a patch/log and removed from the test source. Worker evidence is in the assigned
cb26d-pg scratch, especially sqlstate-diagnostic.log and the red read-only-case logs.
No production edit preceded these observations.

## Existing semantics and scope

Use the existing ErrorCode::Unsupported, preserving its current private-wire
projection and sanitized message. PostgreSQL's primary reference identifies exact
0A000 as feature_not_supported:
https://www.postgresql.org/docs/17/errcodes-appendix.html.
Pinned tokio-postgres also names FEATURE_NOT_SUPPORTED. This corrects one mapping;
no new entity, error code, transport, timing guarantee or read/write authority.

Cited source scope: adapters/sql/src/lib.rs sqlstate_error exact0A000 mapping;
adapters/sql/tests/protocol.rs existing public-invoke error classification case;
adapters/sql/tests/local_runtime/cli_journey.rs the current real read-only case;
adapters/sql/contracts/reads/v1alpha1/semantics.md one native mapping clarification;
docs/local-postgres-cli.md the selected live evidence/limitation text. Shared
contracts and generated descriptors remain unchanged. Keep every other SQLSTATE
mapping and upstream_answer behavior unchanged. Root alone owns AEP and classifier.

## Acceptance

- sql_feature_not_supported_is_not_unavailable: extend the existing
  authentication_database_errors_and_row_capacity_are_sanitized protocol case with
  exact0A000 => Unsupported through the public Sql invocation and actual wire
  fixture. Observe it fail before applying the mapping, then pass afterward;
  preserve no-provider-detail/no-password leakage assertions.
- postgres_cli_cannot_escape_read_only_transaction: correct the CTE expectation
  from the earlier mistaken invalid_input assumption to the source-grounded exact
  unsupported outcome; never accept unavailable. Run the complete original case
  with independent unchanged database contents, SELECT-only role and transaction
  guard checks. Preserve the other distinct refusal mappings.
- Existing package tests and protocol/error classifications remain green; the
  other five selected live cases retain their checks or rerun if relevant inputs
  changed. Package Clippy/formatting pass, and independent review checks both the
  correction and the parent acceptance suite before integration.

## Execution boundary

Accepted under the operator's standing milestone-delivery goal as a separately
scoped defect discovered by story:postgres-real-provider-acceptance. The same
assigned PostgreSQL worker/tree owns this small sequential correction, avoiding
concurrent authorship of its protocol/CLI tests. Root grants a new build/live
window; no new managed tree or publication occurs at this diagnostic step.
The original acceptance story remains active until the corrected full suite and
independent review pass. No broader SQLSTATE reclassification is authorized.
