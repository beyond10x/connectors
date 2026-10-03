---
format: aep.planning-md/3
id: story:mcp-outbound-stdio-runtime
kind: story
status: draft
title: Deliver outbound MCP stdio with session-owned pinned processes
relations:
- decomposes: epic:mcp-contracts
- informed_by: decision-blocker:mcp-outbound-stdio-process-ownership
- depends_on: story:mcp-outbound-auth-lifecycle
- depends_on: story:mcp-outbound-invocation-results
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: adapters/mcp/conformance
- confidence: cited
  path: adapters/mcp/contracts/client/v1alpha1
- confidence: cited
  path: adapters/mcp/contracts/protocol/v1alpha1
- confidence: cited
  path: adapters/mcp/design.md
- confidence: cited
  path: adapters/mcp/generated
- confidence: cited
  path: adapters/mcp/runtime
- confidence: cited
  path: adapters/mcp/spec/ess
- confidence: cited
  path: apps/connectors/src
- confidence: cited
  path: crates/connectors-build/src
- confidence: cited
  path: crates/connectors-host/src/local
revision: 6
---
## Acceptance

Deliver outbound MCP stdio through one explicitly configured, pinned server process
owned by each live outbound session. This implements the operator's 2026-10-03
resolution of decision-blocker:mcp-outbound-stdio-process-ownership, using the
validated connectors_mcp.state.McpStdioProcess entity, its session ownership and
selected-server reference. Both selected protocol revisions and tools, resources
and prompts must work through the actual production entry and a real child peer.
A process launcher, empty catalog or specification alone does not satisfy this story.

Named conformance scenarios, to be authored in the native client contract and bound
to the production process/owner seam before implementation is accepted:

- outbound-stdio-pin-refused: absent, changed or substituted executable bytes refuse
  before child execution; verify the execution-time identity, not only a prior hash.
- outbound-stdio-explicit-launch: only the selected absolute executable, constructed
  argument vector, admitted working directory and explicitly bound environment are
  used; no shell, PATH fallback or inherited provider credentials select authority.
- outbound-stdio-session-ownership: two sessions receive distinct supervised children;
  one session cannot reuse or terminate the other's child or rebind its selected server.
- outbound-stdio-bounded-lifecycle: startup, frame size, queued output, stderr capture,
  request duration and termination/reap waits have enforced limits; EOF, cancellation,
  revocation, timeout and owner loss leave no live owned child after bounded cleanup.
- outbound-stdio-revision-and-framing: legacy initialization and modern per-request
  metadata obey the selected contracts; malformed, oversized and truncated frames,
  capability/version mismatch and partial output have distinct observable outcomes.
- outbound-stdio-admitted-results: tools, resources and prompts preserve declared
  input/output shapes, content, errors and partialness, with exact peer call counts.
- outbound-stdio-authority-and-audit: current grants, declared server credentials,
  admission, audit acknowledgment and the final dispatch fence govern the real write
  to the child; inbound caller and underlying provider credentials stay separate.
- outbound-stdio-effect-uncertainty: lost mutation replies retain unknown outcomes;
  repeated protocol ids, restarts or reconnects never cause implicit business replay.

Use disposable Rust child fixtures and planted defects at pin, ownership, cleanup,
final-fence and replay boundaries. Run the affected ESS validation, projection drift,
conformance and full repository gate, including applicable MSRVs. Update public
support claims only from passing actual-process evidence. This is one delivery slice;
HTTP, inbound runtime, cloud authorization and full MCP acceptance remain separate.

## Model and contract sources

Cited: adapters/mcp/spec/ess/domains/state.yaml declares McpStdioProcess with
process_ref, session_ref, binding_ref, executable_path and executable_sha256, owned
by McpOutboundSession and referencing McpServerBinding. The root validates under
ESS0.45.0. Its Recorded state is a declaration, not evidence of process lifecycle.
The native contract must add lifecycle commands/outcomes and executable scenarios
before claiming launch, pin verification, deadline or cleanup conformance.

Cited: adapters/mcp/contracts/protocol/v1alpha1/selection.md selects stdio for delivery;
client/v1alpha1 contains the existing HTTP lifecycle, authorization and result rules.
Extend the native client contract for stdio without pretending HTTP semantics already
specify it. General durable/HTTP binding lifetime remains UNMAPPED. This story does
not answer decision-blocker:mcp-caller-connection-assignment.

The shared host owns reusable bounded execution mechanics and no MCP vocabulary.
Native MCP owns protocol behavior; application composition connects admitted ports.
Helm is a separately authorized consumer requiring its own native binding and
rollback evidence, not an implicit capability granted by this story. Any new shared
typed entity must be modeled and validated before decomposing implementation around it.

## Scope and scheduling

Cited: adapters/mcp/spec/ess; adapters/mcp/contracts/client/v1alpha1;
adapters/mcp/contracts/protocol/v1alpha1; adapters/mcp/runtime;
adapters/mcp/generated; apps/connectors/src; crates/connectors-host/src/local;
crates/connectors-build/src; Cargo.toml; Cargo.lock; adapters/mcp/design.md.
Inferred: adapters/mcp/conformance; actual-process fixture locations and additional
shared execution port files, to be resolved before implementation dispatch.

This scope overlaps story:mcp-local-stdio-runtime at the native runtime, host,
application, build and Cargo surfaces. Do not dispatch their integration edits in
parallel. Isolated protocol/fixture work may be split only after exact file ownership
is recorded. Keep the story draft until its model/port review and scheduling pass.
Existing workers are quota-exhausted; no independent review is claimed by this draft.

## Model reconciliation evidence — 2026-10-03

The operator's stdio decision is reflected in the authored native model before this
story was created: McpOutboundSession owns at most one McpStdioProcess; that process
references the selected McpServerBinding and carries executable_path and
executable_sha256. The original census now has one stated, one absent, seven
UNMAPPED and one RESOLVED edge. The caller assignment question remains open.
Selection now includes outbound stdio for contract/runtime delivery, not a claim
that the transport is implemented. Native launch/configuration outputs were
regenerated through connectors-build mcp-bindings, not edited by hand.

ESS0.45.0 output: connectors_mcp v1 — 5 file(s), valid (exit0).
The three focused build integration targets passed20 tests,0failed,0ignored.
The first run exposed a stale adversary invariant requiring at least one deferred
blocker-backed row. It now checks every decision-backed row, including resolved
ones, for a feature actually named in the record; the companion guard reads the
record's lifecycle state rather than assuming every cited decision stays open.

Two planted defects failed at their intended assertions (each exit101):
1. Replacing the owned-process cardinality one with many failed the new ownership
   case with left Some(many), right Some(one).
2. Replacing the stdio row's resolved decision citation with the still-open caller
   assignment decision failed the selection guard: one supported row rests on an
   open blocker. Planning records were not altered for this mutation.
Both source files were restored byte-for-byte before the20-test green run and ESS
validation. These are coordinator-authored checks, not independent adversary review.

Retained local evidence and SHA256:
- .local/mcp-runtime/process-ownership-mutation.log:
  4823ca25f3c22ff24ea5691e2509f9e825a065ed71a5302428236b124dd5ac2d
- .local/mcp-runtime/process-decision-mutation.log:
  f8416e76ed4b53b2fd15740f7cf3d71bfcb0f5e8aa1d70eed803f44bf5d91365
- .local/mcp-runtime/process-model-tests-final.log:
  82b9b3fa5ace511e43699df5d0e8b6a60bd4dd07451f6e3fa19271d29fb8fd9a
- .local/mcp-runtime/process-model-validation-final.log:
  7e02c8bf3da80ced204d0ad121f6cab19718f52ea9b65733330f220aa73d7ef1

No process runtime, pin-verification implementation, Helm binding or full MCP
acceptance follows from this structural model. This story remains draft.

## Dependent contract reconciliation — 2026-10-03

The first combined gate stopped at the composition census guard: it still required
8 UNMAPPED rows (observed7 after the operator's resolution). Retained log
.local/mcp-runtime/gate-process-model.log, SHA256
4d990e1b0c5c49c9b2c2ae25ef1e767878cbd5c0debf1867110dce30e695c68a.
This was a stale contract/test assumption, not passing verification.

Reconciled HTTP lifecycle, authorization, invocation and composition prose with the
resolved stdio decision. The HTTP authorization profile still refuses stdio until
its own environment/credential binding is authored; selecting the transport does
not make HTTP OAuth its process credential binding. The composition guard now
checks the actual census footer:7 UNMAPPED entries and the one exact RESOLVED process
edge. The historical adversary reads both decision states and requires the matching
UNMAPPED or RESOLVED marker. HTTP lifecycle checks retain the distinct transport
families and require the separate stdio delivery owner.

All five affected integration targets then passed39 tests,0failed,0ignored:
mcp_composition_provenance, mcp_domain_model_adversary_pass2,
mcp_outbound_connection_lifecycle, mcp_outbound_connection_lifecycle_adversary,
mcp_outbound_auth_lifecycle. Retained log
.local/mcp-runtime/process-model-dependent-tests-restored.log, SHA256
8dcc180167e41b883476ba1715fb00c005852fc0068f8c8f3eb4e89708563567.
The selection adversary diagnostic now includes the disposition, retaining its
useful parsed field and removing the unused-field warning; its3 cases pass, log
.local/mcp-runtime/process-model-adversary-diagnostic.log, SHA256
7b562945204fc50ca178472093b1d9c474038cf8435a550b011dc89ae7b893c9.

The corrected combined full gate is running; no success is recorded yet. Four
coordinator draft-review records are retained separately, all with zero findings;
they are not independent reviews and do not advance the story beyond draft.

## Combined verification — 2026-10-03

The corrected combined gate completed at exit0:1337passed,0failed,66ignored across
163 test summaries. It includes pinned ESS/model/generation checks, formatting,
workspace build/tests/Clippy, native/generic isolation, Rust1.88 selected targets
and Rust1.91 workspace/all-target checks. Command: bounded two-job
RUSTUP_TOOLCHAIN=1.98.1 cargo run --locked --offline -p connectors-build -- gate --msrv,
with RUSTC_WRAPPER empty, incremental/debug outputs disabled, task-owned TMPDIR,
CONNECTORS_ESS=$HOME/.cache/ess/toolchains/0.45.0/ess and
CONNECTORS_AEP=$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep.
Retained log .local/mcp-runtime/gate-process-model-restored.log, SHA256
32e7e85418dfcd65d372bcbc12207dfa5000272af113d19d6e9cb0a6ed1fb34a.
AEP validation is valid and retains101 historical missing review-outcome warnings
in .local/mcp-runtime/aep-validation-process-model.log; no warning cleanup is claimed.

This covers the metadata revision fix, native process association, regenerated
projections and reconciled transport/HTTP-only contract guards. The66 ignored tests
were not executed by this gate. The previously recorded real final-fence fixture
remains separate evidence. Both ownership and open-decision mutations failed at
the intended assertions and restored focused tests passed; no mutants remain.

Publish this verified checkpoint to the existing draft PR84. It does not complete
inbound or outbound stdio runtime, public operation curation, real I/O deadlines,
inbound mutation binding, authenticated outbound HTTP, Helm execution or the full
provider milestones. No merge or new source release follows from this checkpoint.
Cloud caller assignment stays open. Keep cb26l-runtime active and leased: exact-id
cleanup finds no candidate; the broader workspace dry-run found12 refused candidates
and none eligible, all preserved. No local verification process remains running.
