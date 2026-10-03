---
format: aep.planning-md/3
id: story:metadata-sidecar-fixture-isolation
kind: story
status: active
title: Isolate the sidecar symlink fixture from pooled metadata handles
relations:
- informed_by: story:mcp-local-stdio-runtime
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/tests/local_foundation.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T11:31:42Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-03T11:31:43Z", actor: "human:timo", revision: 4}
---
## Outcome

The host sidecar-symlink security test reliably reaches its refusal assertion with
an initialized independent SQLite store, regardless of pooled Entity Runtime handles.

## Evidence and acceptance

The MCP runtime gate (gate-supervision.log, session52870, Rust1.98.1) failed at
crates/connectors-host/tests/local_foundation.rs:378 before the hostile symlink was
installed. Its five-second wait for the WAL to disappear is not a security assertion.
The bounded reproducer is cargo test --locked --offline -p connectors-host --test
local_foundation sidecar_symlink_is_refused_without_touching_its_target -- --exact.
Record baseline, targeted regression and the complete local_foundation suite.

Preserve the named test's existing contract: Metadata::inspect returns
MetadataUnavailable for a symlink sidecar and the external sentinel is untouched.
Prove the prepared fixture is otherwise valid by inspecting that same snapshot
after removing only the hostile symlink. No production admission or timeout changes.
This is test-fixture repair, not a new runtime entity or conformance-suite claim.

## Scope

Cited: crates/connectors-host/tests/local_foundation.rs owns the failing setup.
Cited reference only: metadata_reopen_adversary.rs:58-74 already builds a closed
logical snapshot using VACUUM INTO and explicitly sets WAL mode and private files.
Coordinator performs implementation and review serially because delegated workers
are quota-blocked. No independent review is claimed; no critic panel is needed for
this single repair story. Full repository gate is required before publication.

## Diagnosis and correction — 2026-10-03

Baseline reproduces3/3 in isolated runs (~5.4s each), always before installing the
hostile link. Ranked hypotheses were (1) intentionally pooled authority outlives
setup; (2) delayed asynchronous retirement; (3) another leaked connection. The
first is confirmed by metadata/er.rs::release_handle/release and IDLE: reusable
handles enter a process-wide pool of four rather than retire. The original test's
assertion therefore depended on unrelated tests evicting that handle.

Changed only fixture construction: VACUUM INTO an independent private directory,
set WAL mode and close its direct SQLite connection, then install the hostile
WAL symlink. No source WAL is unlinked; committed source WAL content enters the
snapshot through SQLite. The same fixture succeeds in Metadata::inspect after
removing only the hostile link, ruling out malformed-database false positives.
The sentinel remains unchanged before and after normal admission.

Targeted test passes1/1 in0.33s; complete local_foundation suite passes10/10 with
no skips. Production metadata admission and timeouts unchanged. Logs:
.local/mcp-runtime/fixture-{repro,repro-2,repro-3,green,suite}.log.
Full repository gate and publication remain required; this story stays active.
