---
format: aep.planning-md/3
id: story:postgres-real-provider-acceptance
kind: story
status: active
title: Close PostgreSQL C06 and local lifecycle acceptance gaps
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:postgres-local-cli-journey
- depends_on: story:sql-fixture-accepts-stray-connections
- depends_on: story:bridge-drop-waits-for-dispatched-batch
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/sql/tests/local_runtime.rs
- confidence: cited
  path: adapters/sql/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: adapters/sql/tests/protocol.rs
- confidence: cited
  path: docs/local-postgres-cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T15:18:28Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}, correlation: "wave-20261002d-provider-acceptance"}
- {from: "proposed", to: "active", at: "2026-10-02T15:18:28Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}, correlation: "wave-20261002d-provider-acceptance"}
---
## Problem and evidence

The real restart journey and direct/federated conformance run passed on 2026-10-02
(docs/evidence/provider-restarts-20261002). They prove saved-credential reuse,
parameterized filtering, exact numeric/null/array/JSON values, bounds and several
read-only refusals. They do not establish every PostgreSQL C06 incident query or
real-backend cancellation and lifecycle races. The existing journey's write
assertion checks only failure; strengthen it to exact classification and unchanged
database state rather than treating any transport error as a valid refusal.

## Acceptance

All five named real-provider conformance cases pass against a task-owned disposable
PostgreSQL instance. Four CLI cases use the production CLI/adapter binaries and
qualified custody binding; cancellation uses the production SQL adapter library's
invocation future directly, matching its native contract. Missing provider prerequisites produce an explicit
unexecuted result, never acceptance. Record exact database fixture and executable
identities, commands, outcomes and cleanup. The existing successful evidence is
retained rather than replaced by the new cases.

- postgres_cli_preserves_join_group_utc_and_quoted_parameters: parameterized joins
  and aggregation return exact expected rows/counts; UTC timestamp boundaries and
  a quoted value remain data with no SQL interpolation.
- postgres_cli_distinguishes_empty_truncated_capacity_and_timeout: empty results
  retain their schema, row truncation is explicit, oversized values refuse by the
  declared capacity outcome, and a sleeping query produces the declared timeout.
- postgres_cli_cannot_escape_read_only_transaction: write, multiple-statement and
  transaction-control attempts return the specified refusal; independently read
  fixture contents remain unchanged and no write reaches a privileged role.
- postgres_dropped_invocation_cancels_its_backend: start a uniquely marked
  `pg_sleep(60)` through the production `Sql` adapter's invocation future and
  observe its exact backend PID/query from a separate fixture-admin connection
  within two seconds of starting that invocation; then abort/drop that future
  immediately and require that backend to stop executing the marked query within
  five seconds of the drop. The paired no-drop control must still execute after
  five seconds, before explicit fixture cleanup. This controlled-loopback deadline
  is an acceptance observation, not a promise that remote cancellation always
  terminates within the adapter's two-second local cleanup budget. Retain timing,
  PID/query correlation and both control outcomes; a late initial observation
  does not satisfy the case. Killing a CLI process is not this cancellation trigger.
- postgres_repair_revoke_and_busy_stop_preserve_authority: wrong-identity repair
  cannot replace current authority, revocation prevents new dispatch, and stop
  waits/refuses according to existing lifecycle rules while admitted work is busy.

## Scope and ownership

Cited: adapters/sql/tests/local_runtime/cli_journey.rs,
adapters/sql/tests/local_runtime.rs, adapters/sql/tests/protocol.rs,
adapters/sql/src/lib.rs, adapters/sql/contracts/reads/v1alpha1/semantics.md,
docs/local-postgres-cli.md. Initial implementation edits are tests and guide only;
source/contracts are reading owners. Reproduced product defects return for a
separately scoped correction before changing runtime semantics. No new entity.
Root alone updates ignored-runner classification and AEP evidence; test names are
handed back before adding entries to crates/connectors-build/src/ignored.rs.

## Dependencies and limits

Uses the existing SQL fixture correction and timeout ownership fix from the
reliability batch. Preserve statement_timeout, lock_timeout, the fixed search_path,
parameter binding, read-only transaction and credential authority. SQL remains
read-only; MySQL is outside this unit. This closes selected PostgreSQL evidence,
not the cross-provider initiative or sustained-read cost blocker.

Cancellation boundary sources: adapters/sql/src/lib.rs:150-167 drops the native
receive future and wakes the supervising sender; adapters/sql/tests/protocol.rs
already tests that invocation-drop trigger against a fake server. The real-provider
case supplies missing backend evidence. contracts/cli/v1alpha1/owner.md permits
already dispatched reads to finish after CLI disconnect, and the SQL read contract
explicitly disclaims guaranteed remote termination after bounded local cleanup.
