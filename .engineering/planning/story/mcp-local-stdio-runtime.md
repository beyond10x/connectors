---
format: aep.planning-md/3
id: story:mcp-local-stdio-runtime
kind: story
status: active
title: Deliver local inbound MCP stdio over admitted operations
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-inbound-local-binding
- depends_on: story:mcp-inbound-capability-projection
- depends_on: story:mcp-inbound-mutation-replay
scope:
- confidence: cited
  path: .gitignore
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: adapters/README.md
- confidence: cited
  path: adapters/catalog/src/local.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/bounded_reads.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/oauth2_refresh.rs
- confidence: inferred
  path: adapters/mcp/conformance
- confidence: cited
  path: adapters/mcp/contracts/server/v1alpha1
- confidence: cited
  path: adapters/mcp/design.md
- confidence: inferred
  path: adapters/mcp/generated
- confidence: cited
  path: adapters/mcp/runtime
- confidence: cited
  path: adapters/mcp/spec/cli.yaml
- confidence: cited
  path: adapters/mcp/spec/ess
- confidence: cited
  path: apps/connectors/Cargo.toml
- confidence: cited
  path: apps/connectors/spec/compatibility.json
- confidence: cited
  path: apps/connectors/src
- confidence: inferred
  path: apps/connectors/tests/local_mcp_server.rs
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: contracts/cli/v1alpha1/private-bounded-read.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/local-operation-curation.md
- confidence: cited
  path: crates/connectors-build/src
- confidence: cited
  path: crates/connectors-core/Cargo.toml
- confidence: inferred
  path: crates/connectors-core/generated/operation-types
- confidence: cited
  path: crates/connectors-core/src
- confidence: cited
  path: crates/connectors-core/tests
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/src/local
- confidence: cited
  path: crates/connectors-host/src/local/keyring.rs
- confidence: cited
  path: crates/connectors-host/src/local/keyring/custody.rs
- confidence: cited
  path: crates/connectors-host/src/local/keyring/deadline.rs
- confidence: cited
  path: crates/connectors-host/src/local/operation_curation.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/governed/bounded.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/governed/tests/bounded_reads.rs
- confidence: cited
  path: crates/connectors-host/tests/http_budget.rs
- confidence: cited
  path: crates/connectors-host/tests/local_foundation.rs
- confidence: cited
  path: docs/development.md
- confidence: cited
  path: docs/local-mcp-cli.md
- confidence: cited
  path: docs/local-runtime-foundation.md
- confidence: cited
  path: ess/domains/service_wire.yaml
- confidence: cited
  path: ess/domains/transport.yaml
revision: 76
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:29:40Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-03T10:29:41Z", actor: "human:timo", revision: 6}
---
## Acceptance

Implement the selected connectors server --transport stdio entry, using the native
connectors_mcp.launch values validated before this story. It must compose generated
native parsing and shape/type validation with clap-derived process globals, never
send a CLI ProcessOutput envelope to protocol stdout, and admit the existing local
owner before naming capabilities or invoking a provider. Preserve grouped and legacy
commands. This is the complete local inbound runtime story, not acceptance of a
parser-only, empty-catalog or no-auth approximation.

Named actual-process conformance scenarios (both selected revisions where applicable):
- local-server-launch-selected: config/state-dir and required stdio parse; unsupported
  transport, output and protected-source flags refuse; help exposes only selected flags.
- local-server-owner-refused: missing/mismatched owner rejects before protocol
  capabilities or provider activity; stderr is safe and stdout has no CLI envelope.
- local-server-admitted-discovery: a real selected operation is listed, while disabled,
  unbound or metadata-denied operations are absent; unavailable policy refuses instead
  of pretending to be an empty catalog. Verify actual provider and owner call counts.
- local-server-typed-results: admitted tools/resources/prompts preserve exact target,
  schema, result/error, content and partialness under the existing projection contract.
- local-server-revision-and-capability: modern metadata is required per request;
  legacy initialize gates readiness; mismatch follows the contract's distinct routes.
- local-server-framing-and-loss: malformed JSON, malformed envelopes, over-bound and
  truncated frames have exact errors/terminal outcomes; partial output never completes.
- local-server-lease-and-cancel: shared admission/renewal/expiry clocks govern every
  data path; progress grants no renewal; explicit cancellation answers nothing and
  observes actual local cleanup within bounds without asserting remote rollback.
- local-server-mutation-replay: mutations remain unadvertised without the complete
  protected approval and business-key binding. Once enabled, lost replies retain
  uncertainty and repeated MCP ids never cause implicit effect replay.

Each scenario invokes the production entry and an independent peer/provider fixture.
A static contract census, generated reference runner or passing parser prototype is
not runtime evidence. Add ESS conformance bindings at the actual process/owner seam
and mutation controls for owner bypass, stdout pollution, hidden capability and replay.
The full repository gate and Rust1.88 checks must pass; then reconcile compatibility,
working CLI docs and static discovery with actual support. The parent CLI journey
still includes persistent outbound HTTP and cannot close from this story alone.

## Native/shared architecture

Native launch inputs/outcomes are immutable in adapters/mcp/spec/ess/domains/launch.yaml;
source semantics are docs/local-mcp-cli.md and server/v1alpha1. The native parser
fixture's package identity is distinct from the generic CLI contract; production
takes only its server subtree and validates through the generated shape. Shared ESS
imports no native type.

Native protocol mechanics belong to adapters/mcp; application composition supplies
host admission and operation ports. Reuse shared sessions state/lease behavior,
existing local owner transport and mutation controls through explicit ports. Do not
invent a caller entity, persistent MCP session association or a provider secret route.

On 2026-10-03 the operator resolved outbound stdio ownership: each outbound session
owns one explicitly configured pinned server process. The native state model now
represents that subset and validates as five files under ESS0.45.0; launch/configuration
projections were regenerated. story:mcp-outbound-stdio-runtime owns its separate
runtime acceptance and overlaps this story's integration scope. The cloud caller
assignment decision remains open. All remaining UNMAPPED relations stay unresolved.
Helm bounded execution was separately approved; no Helm runtime is delivered here.

## Scope

Cited: adapters/mcp/spec/ess; adapters/mcp/spec/cli.yaml;
adapters/mcp/contracts/server/v1alpha1; adapters/mcp/runtime;
adapters/mcp/generated; adapters/mcp/design.md; adapters/README.md;
Cargo.toml; Cargo.lock; .gitignore; apps/connectors/src;
apps/connectors/Cargo.toml; apps/connectors/spec/compatibility.json;
crates/connectors-host/src/local; crates/connectors-build/src;
docs/local-mcp-cli.md; contracts/cli/v1alpha1/semantics.md.
Inferred: adapters/mcp/conformance; apps/connectors/tests/local_mcp_server.rs;
remaining host and application composition wiring.
The runtime package is a sibling of its generated dependencies, preserving ESS-owned
workspace manifests without source edits. This shared surface must not be dispatched
concurrently with outbound wiring into the same host or application files. Worktree,
build and scratch ownership remain in specification:mcp-runtime-delivery-20261003.

## Review and scheduling boundary

The operator's full MCP delivery authorization starts this story in reviewed phases.
The coordinator has checked the independently specified qualified-name/resource-URI
codec and bounded newline framing against the landed projection/local-binding
contracts. These mechanics can be implemented now without selecting a new caller,
credential, persistent session or configuration ownership relation. The existing
native model already owns advertised_name and operation_ref; byte buffering is a
transport algorithm, not a parallel domain model.

Host admission, native deployment configuration and the complete session runtime
remain subsequent phases of this same story. They must receive explicit model/port
review before their integration. Four proposed native configuration values validate
in scratch, but are not accepted or promoted by this scheduling decision. The
existing shared sessions model synthesizes 33 generated capabilities and nine
implementation obligations, with no refusal; use its generated behavior interfaces,
not a hand-transcribed state enum. Full acceptance remains unchanged and no CLI
runtime support is advertised until the complete actual-process scenarios pass.

This is a coordinator acceptance/design/scope/parallel-safety review, performed as
separate readings because the existing workers remain quota-exhausted. It is not
independent review or human approval. Naming is injective and bounded; the frame
parser must resynchronize after oversized lines and discard truncated EOF without
execution. Work remains in cb26l-runtime with one source/store writer. The ongoing
full gate checks the preceding source snapshot; a later gate must cover added runtime.

## Native naming and framing implementation checkpoint

The first implementation files are adapters/mcp/src/names.rs and framing.rs, tested
by adapters/mcp/tests/framing_and_names.rs. They are not yet a Cargo workspace
package or a public CLI command. The preceding full gate was already running and
does not establish coverage of these added native files; integrate their package,
test selection and generation inventory before the next publication gate.

Qualified names use exact bounded lowercase UTF-8 hex, three nonempty components,
canonical re-encoding, and a 128-byte ceiling. Resource URIs retain one exact
nonempty descriptor revision and reject authority, fragments, extra/repeated query
members, percent decoding and normalization. These codecs convey no authority.

The incremental decoder consumes at most one LF-terminated frame per feed so its
caller can apply backpressure. Its admitted positive wire-byte ceiling includes LF.
It preserves invalid UTF-8 and exact large-number bytes for later protocol parsing.
An oversized line retains no payload and drains to LF before returning its refusal;
next-frame resynchronization survives. EOF discards partial input without a frame.
Clock/deadline checks, JSON interpretation and owner admission remain caller duties.

Rust1.88 standalone native tests: first run 0passed7failed against stubs; completed
implementation 7passed0failed0ignored. A copy-only author mutation removing EOF
buffer discard gives6passed1failed, exactly
long_unterminated_input_is_bounded_and_eof_never_executes_a_prefix. Production source
was never mutated; another unchanged-source run passes7. Logs and the bounded Rust
probe binaries are under .local/mcp-runtime/native-tests. These are native checks,
not an ESS conformance report or independent adversary approval.

The shared sessions source was copied byte-for-byte into a scratch selection with
its own exact header. ESS0.45 validates it and synthesizes42 capabilities:33generated,
9obligations,0refused. The generated types/lifecycles and eight behavior/one query
interfaces give the next runtime a generated owner. This does not execute leases
or complete any obligation. The earlier comment saying session-connection-binding
was unfiled is stale: that draft story exists and remains unchanged/unresolved.

A scratch native local-server configuration proposal validates as5files and projects
4types: family, exposure, limits and configuration. It explicitly selects an existing
host operation and Connection per exposure for the single owner; it grants no MCP
caller authority and closes no cloud assignment relation. It is still a proposal,
not promoted source. Source path .local/mcp-runtime/probe/local-server-spec; runtime
configuration location, admission serialization and lease integration need recorded
review before wiring. No shared native-type import or arbitrary JSON registry is used.

## Workspace and native launch integration — 2026-10-03

The native library now lives at adapters/mcp/runtime, beside generated/launch-cli
and generated/launch-types. The first nesting placed those generated workspaces
under a workspace member and Cargo refused multiple workspace roots. Moving the
runtime to a sibling resolved that refusal without modifying generated manifests.
The root Cargo workspace now includes connectors-mcp; the lockfile records its
exact generated path dependencies. The native package is also in the gate's
Rust1.88 all-target selection.

The native launch module extracts only the generated server leaf before Clap
global propagation. It validates input through both the generated callable shape
and generated launch type. Composition still owns process config/state selectors,
owner admission and I/O. It never calls the fixture's ProcessOutput renderer.
Four new tests exercise selected argument orders, missing/unknown/duplicate transport,
protected and output flags, and help. Alongside seven framing/naming tests, all11
pass under both current Rust and Rust1.88 with no ignored tests. Logs:
.local/mcp-runtime/native-tests/workspace.log and msrv.log.

The first full gate terminated during an MSRV check when it observed the temporary
nested-workspace error; it is not a successful publication gate. A new full gate is
required after all source changes. The new mcp-bindings command regenerates/checks
the native CLI projection and compares fresh type artifacts while excluding only
ESS's local output ledger. Its own compile/check is still running at this checkpoint.
No actual-process runtime conformance, host/session wiring, full MCP release or
independent adversary approval is claimed by these native results.

## Host port evidence for the next phase — 2026-10-03

The existing private owner read entry is not yet the complete governed MCP port.
owner::operation_snapshot and owner::admit_operation in
crates/connectors-host/src/local/owner.rs intentionally check private operation
existence before permissions/revision. MCP projection requires current lookup/scope
admission first, exact projection revision second, then private existence/enablement.
Reusing that entry alone would violate the selected refusal precedence.

owner::Client::invoke (owner/transport.rs) returns Result<Value> from the private
owner. Its Request::Invoke handler repeats the legacy ordering and returns the
pool's Output::Value. The public core::Response carrier currently contains only
version, request_id and outcome. The MCP projection requires the complete selected
service Response, including admitted audit/provenance and applicable mutation
observations. The separate mutation::Delivery already retains original request,
mutation and source_audit evidence; those observations must survive composition.
Do not invent an audit receipt while wrapping the read value.

crates/connectors-core/src/lib.rs explicitly preserves historical default numeric
behavior in its read_json decoder even when arbitrary_precision is enabled. The
new generated launch types enable that serde_json feature in the workspace. Existing
repository feature-invariance tests remain required; an MCP data port additionally
must demonstrate lossless selected request/result handling, not infer it from this
launch-only feature activation. Config/metadata selection and a bounded supervised
operation port must be reviewed against these source facts before public wiring.

Native projection checker completed successfully (exit0):10 CLI artifacts current;
4 selected model types regenerated and byte-identical to the promoted type artifacts.
The complete checker command was CONNECTORS_ESS=<pinned0.45.0> cargo run --locked
--offline -p connectors-build -- mcp-bindings --check. Output is retained in
.local/mcp-runtime/native-tests/generation.log. The new full repository gate including
MSRV is running under session39929, log .local/mcp-runtime/gate-native.log. It includes
these source changes, the native dependency boundary and exact generated projections;
its eventual result remains to be inspected. No publication has occurred.

## Session implementation phase review — 2026-10-03

The next phase implements the already-declared shared Session behavior interfaces
for the native local supervisor. Generate them by selecting connectors.sessions
from the existing shared system header and copying its domain bytes unchanged into
a temporary input root. The selected generated package belongs under native MCP
projections; shared ESS remains the model owner. Compare the entire generated tree
in the existing native binding gate. No parallel authored state enum, lifecycle,
command/event carrier or persistent relation is introduced.

The first implementation unit is the serialized state reducer behind the trusted
supervisor boundary: it takes trusted admission/cleanup decisions and a trusted
terminal clock, uses generated typestate transitions, assigns declared fields,
preserves the first terminal fact, clears lease authority on closure/loss, and
returns declared outcomes/events and read-your-writes views. It does not expose
a peer-callable decision API or claim actual timed I/O enforcement. Timed admission,
clock verification, resource accounting and host wiring remain part of this same
story and must be established before public runtime support.

Acceptance/design/scope/parallel-safety review by the coordinator: the unit maps
exactly to existing generated eight behavior interfaces and one query, avoids the
unresolved Session-to-Connection relation, changes only native implementation and
projection/build scope, and does not compete with an outbound author. Test first
terminal retention across denial/close/loss, denial preventing renewal, exact field
assignment, wrong-state/unknown-instance non-mutation, and local cleanup independent
of peer acknowledgement. This is a coordinator review, not an independent panel;
existing worker quota prevents independent dispatch. Full actual-process acceptance
is unchanged and the story remains active.

## Generated session reducer checkpoint — 2026-10-03

Pinned ESS0.45 synthesis of the byte-identical shared sessions domain validates
2 selected input files and emits42 capabilities (33 generated,9 obligations,
0 refused),8 artifacts. The generated output is now under
adapters/mcp/generated/supervision; no generated file was edited. Runtime source
adapters/mcp/runtime/src/supervision.rs implements all eight behavior traits and
the state query using generated typestate moves and generated fields/outcomes.
It remains unexported/unlinked until the running earlier gate completes.

An initial test run against the generated unmet-obligation stub gives0passed6failed.
The reducer passes all8 final tests on Rust1.88. They check full field preservation,
lease assignment/clearing, denials before and after readiness, no late renewal,
unknown/wrong-state non-mutation, first-terminal retention across continuity loss,
actual cleanup decision versus unconfirmed peer shutdown, and duplicate identity
port refusal without overwriting an existing session. A scratch-only mutation that
always overwrites the terminal yields7passed1failed, exactly
continuity_loss_preserves_the_first_terminal_and_never_reopens. Unchanged source
passes all8. Logs and Rust fixtures remain in
.local/mcp-runtime/probe/supervision/{red,green,mutation-terminal}.log.

The production dependency/module/test wiring and shared-source regeneration checker
are prepared but not yet integrated while gate session39929 checks the preceding
native snapshot. The reducer trusts local admission/cleanup/clock ports; it is not
an MCP wire parser or proof of timely data cutoff. No terminal clock, teardown or
lease enforcement claim is promoted by these state tests. Actual-process and full
inbound/outbound release acceptance remain outstanding.

## Reducer integration and gate follow-up — 2026-10-03

The preceding full gate finished its test stage:1267 passed,0failed,65ignored,
157 result summaries. It then exited1 on two Clippy findings in the native hex
decoder (manual is_multiple_of and chunks_exact_to_as_chunks). Both are corrected
using Rust1.88-compatible operations; the current native all-target Clippy check
passes with warnings denied. The failed gate is not publication evidence.

The shared-model dependency, module export and production-linked tests are now
integrated. All20 native tests pass in the Cargo workspace and under Rust1.88
(7 naming/framing,4 launch,9 supervision;0failed0ignored). The last supervision
test enumerates all59049 five-command sequences from nine actions and checks that
first termination is retained and no later command restores lease authority or
changes a Closed/Lost record. A copy-only first-terminal overwrite mutation fails
both the explicit continuity test and this sequence check:7passed2failed. It does
not alter the production source. These are native invariant checks, not complete
ESS actual-process conformance or an independent review.

The MCP generation helper now selects the shared sessions domain from the owning
header, copies its exact authored bytes and compares all generated supervision
artifacts. Its fresh pinned compile/check is running under session66746; inspect
.local/mcp-runtime/native-tests/supervision-generation.log. Native evidence logs are
supervision-workspace.log, supervision-msrv.log and supervision-clippy.log in the
same directory; mutation-sequences.log lives under probe/supervision. A new full
gate remains required after that generation check. Timed admission, actual cleanup,
protected host ports, public entry, outbound HTTP/custody and the final release
remain pending under the full delivery goal.

The fresh generation-check compile (session66746) terminated101 before executing
rustc for rustls: `could not execute process .../stable-x86_64-unknown-linux-gnu/bin/rustc`
with `No such file or directory (os error 2)`. The stable path was present again at
inspection, with a new file timestamp; the cause of its temporary absence is not
established. The installed explicit Rust1.98.1 reports the same repository CI pin
(.github/workflows/rust-gate.yml). The next run sets RUSTUP_TOOLCHAIN=1.98.1 rather
than installing or modifying toolchains. Original failure log is retained, and the
retry writes native-tests/supervision-generation-pinned.log. This is a build-environment
failure, not successful generation evidence or a new publication gate.

The explicit Rust1.98.1 generation retry82826 completed successfully (exit0).
All10 native CLI artifacts and4 launch model types match; selected shared-session
synthesis again reports42 capabilities/33generated/9obligations/0refused, and all8
supervision artifacts match byte-for-byte. The new complete gate is live under
session52870 with log .local/mcp-runtime/gate-supervision.log. It has repeated these
projection checks successfully and is checking the integrated native workspace;
its final result remains pending. The pinned generation log is retained separately
from the transient stable-toolchain failure.

## Timed lease-decision phase review — 2026-10-03

Implement a native lease gate consuming generated connectors.sessions.DataLease
values and returning generated GateDecision values. The clock is a trusted local
port yielding a qualified UTC interval in milliseconds, not MCP input, a wall-clock
string asserted by a peer, or a new persisted entity. The gate parses the generated
RFC3339 timestamps without dropping fractional precision, admits no lifetime over
2000ms, accounts for the full current clock interval and delivery delay, checks the
old lease before renewal, and permanently retains the first denial. Monotonic
sequence and issuance checks reject replay/reordering; a late renewal cannot revive
an expired gate. Progress/data checks never renew authority. A selected drain is
at least2000ms ahead of the conservative observation, caps later renewal deadlines,
and stops authority by the earlier applicable expiry/drain deadline.

This is the timed decision component behind the same serialized local supervisor;
it grants no new caller/Connection relation and no wire credential. The actual
clock qualification adapter, atomic host admission and final I/O-boundary use must
still be wired and tested before claiming the2s/5s runtime guarantees. Generated
session reduction receives these decisions; the component does not hand-transcribe
states or command types. The first denial clears the retained lease and later
observations cannot mint authority. Local resource accounting remains separate.

Coordinator review (not independent): source is sessions/v1alpha1 section4.1 and
server/v1alpha1 section2; test delay/uncertainty, exact expiry, replay/reordering,
clock loss/regression, progress non-renewal, drain caps and first-denial retention.
No new public CLI support or narrower acceptance is selected. Continue in the current
native scope; do not change the running gate's sources until its snapshot completes.

## Lease integration and gate repair — 2026-10-03

Native LeaseGate is integrated at adapters/mcp/runtime/src/lease.rs with the existing
workspace time dependency. It consumes generated DataLease and returns generated
GateDecision; no model or generated file was hand-edited. Nine lease tests cover
delay/uncertainty, exact deadlines, replay and issuance ordering, invalid intervals,
nonrenewing activity, clock loss, drains, first denial and nanosecond precision.
The isolated stub was0pass9fail; implementation9pass0fail; a copy-only expiry
mutation changing >= to > was6pass3fail. Linked native suite29pass0fail0ignored
on Rust1.98.1 and Rust1.88. Newer Clippy required two equivalent let chains;
Clippy is clean and final Rust1.88 verification is retained in lease-msrv.log.
Logs are .local/mcp-runtime/native-tests/lease-{workspace,msrv,clippy}.log and
probe/lease/{red,green,mutation-expiry,clippy}.log. These are component evidence,
not actual-process ESS conformance or a completed inbound/outbound delivery.

Full gate52870 is terminal failed: local_foundation's sidecar fixture expected a
pooled WAL to disappear. story:metadata-sidecar-fixture-isolation owns the bounded
repair and evidence; no production timeout or security admission was weakened.
Generation checks had passed before that failure. The next full gate must cover
both the integrated lease gate and the repaired fixture. Cleanup removed unused
root build output and retired merged MCP tree cb26f-mcp with archived evidence;
Connectors runtime tree remains active and all actual I/O/public launch/outbound
acceptance stays open.

## Local configuration draft refinement — 2026-10-03

ESS0.45 validates the scratch model (5files) and generates four selected Rust
values from Configuration. Existing Family, Exposure, Limits and Configuration
remain immutable native deployment inputs, not new persisted entities or relations.
The refined Exposure adds required enabled:Boolean and optional approval_file:String.
The exact draft domain is reproduced below so the proposed semantics do not live
only in scratch. It is not yet selected as shipped configuration or linked code.

The enabled flag separates explicit owner selection from current host permission.
An approval source is private local configuration, never MCP arguments, discovery,
audit or protocol metadata. Its presence makes a protected source selectable; it
asserts neither file existence nor current proof validity. The admitted coordinator
uses the existing protected file path only for a new candidate needing approval.
An admitted retained replay must not open, validate or spend proof, even when the
file was removed or the original proof has expired. Missing source selection keeps
required-approval mutation projections unbound; a selected but unavailable proof
at execution follows existing approval errors, not discovery withdrawal.

Before promotion, select the configuration location and admitted snapshot/fencing
rules, enforce positive compatible limits, and test duplicate/unknown fields,
duplicate exposures, invalid selectors, family eligibility and proof-channel
non-disclosure. Existing Config is closed and generic: do not add an MCP value bag.
The draft does not resolve cloud/multi-caller Connection assignment or outbound
stdio ownership. Those operator decisions remain open.

The next protected host port must check current lookup/target/result policy before
projection revision, then bound existence/enablement and input before key lookup.
Read dispatch requires a real acknowledged audit anchor and final observation via
the existing audit Store; owner::Client::invoke returns only Value and cannot be
wrapped into a fabricated complete response. Source: owner.rs::admit_operation,
owner/transport.rs Request::Invoke, audit.rs::Store::anchor/append, and service
compatibility sections4-5. Preserve existing legacy callers while adding the governed
port. The full safe service Response must retain applicable source/mutation audit
and nullable request correlation according to its codec obligations. No new ledger
or caller identity is authorized by this draft.

Validation logs: probe/local-server-refined-validation.log and
probe/local-server-refined-projection.log. Generated output has closed structs,
required enabled and optional-presence approval_file. Structural generation does
not prove protected-file admission, authority, protocol behavior or conformance.

```yaml
domain: connectors_mcp.local_server
summary: Explicit local deployment projection inputs, not caller or credential ownership.
types:
  # The selected inbound families in the existing selection/projection contracts.
  - name: connectors_mcp.local_server.Family
    kind: enum
    variants: [tools, resources, prompts]
  # One configured projection of an existing host operation. The adapter alias
  # resolves through the ordinary owner configuration; instance/adapter identity
  # comes from its admitted descriptor, never from an MCP request. connection_ref
  # selects an existing host Connection for that operation. This is immutable
  # deployment input, not a relation on McpCaller or McpInboundSession, and creates
  # no Connection or cascading ownership. No secret bytes/locator are admitted.
  - name: connectors_mcp.local_server.Exposure
    kind: struct
    fields:
      - {name: adapter_alias, type: String}
      - {name: operation_ref, type: String}
      - {name: connection_ref, type: String}
      - {name: families, type: "List<connectors_mcp.local_server.Family>"}
      - {name: enabled, type: Boolean}
      # Local owner configuration only. Never projected into MCP metadata or
      # passed as caller authority. The existing protected-file reader opens
      # this only for an admitted candidate attempt that needs proof. Retained
      # replay does not read or spend it. Its presence selects a binding, not
      # evidence that the file exists, is valid or authorizes this invocation.
      - {name: approval_file, type: "Optional<String>"}
  # Runtime validates every positive ceiling and the supported format before
  # exposing capabilities. A decoded structural carrier alone grants nothing.
  # Exact lease <=2000ms and cleanup <=5000ms are fixed shared requirements,
  # not configurable relaxations. Native configuration cannot set caller UID.
  - name: connectors_mcp.local_server.Limits
    kind: struct
    fields:
      - {name: frame_octets, type: Integer}
      - {name: response_octets, type: Integer}
      - {name: concurrent_requests, type: Integer}
      - {name: request_milliseconds, type: Integer}
  - name: connectors_mcp.local_server.Configuration
    kind: struct
    fields:
      - {name: format, type: String}
      - {name: exposures, type: "List<connectors_mcp.local_server.Exposure>"}
      - {name: limits, type: connectors_mcp.local_server.Limits}
# UNMAPPED: cloud caller-to-Connection assignment remains its existing blocker.
# These inputs are only for the single configured owner. They do not model a
# durable MCP caller, session, snapshot, provider credential or rotating alias.
```

## Lossless JSON codec phase — 2026-10-03

The private read worker retains provider output as raw JSON text in Output::Value
(owner/supervisor.rs, Task::Invoke). connectors_core::read_json explicitly preserves
legacy f64 conversions despite arbitrary_precision feature unification. MCP cannot
reuse it for native frames or complete service payloads without changing values.
Implement a separate native bounded-depth decoder using serde_json raw_value and
arbitrary_precision. Decode literal objects with a duplicate-key check before any
Value projection; arbitrary operation-local keys, including strings that coincide
with serde_json's private tokens, must remain ordinary keys. Decode scalar numbers
without a binary floating-point round trip. Preserve legacy core reader behavior.

Acceptance: exact huge integer/decimal values and scalar/array/null payloads; duplicate
keys at every nesting level including escaped-equivalent keys refused; ordinary
private-token-looking keys preserved as objects; malformed/trailing/non-UTF8 input
refused; explicit positive depth bound enforced for objects and arrays. The existing
frame ceiling owns bytes; this codec owns JSON structure and depth. No new domain
entity, authority or public error code is introduced. Host audit and actual protected
I/O remain mandatory. Scope is the already selected native runtime and dependencies.

Current gate83384 remains live on the preceding source; build/test the isolated
probe under .local/mcp-runtime/probe/json, then integrate after that gate completes.
Coordinator review is not independent; no substitute agents are dispatched around quota.

## Lossless JSON probe evidence — 2026-10-03

Isolated Rust1.88 probe has six tests: initial stub0pass6fail, implementation6pass0fail,
Clippy clean. Duplicate-check removal yields5pass1fail. Introducing a binary-float
round trip yields3pass3fail, including the precision test; restored6pass0fail.
Logs: probe/json/{red,green,clippy,mutation-duplicates,mutation-precision,restored}.log.

Objects are decoded as literal key/raw-value pairs, checked for duplicate decoded
keys and reconstructed directly; no private serde token can reinterpret an object.
Scalar numbers use arbitrary_precision, avoiding f64. Structural depth counts
containers and is explicitly bounded1..128, with exact128/129 boundary coverage.
Whitespace-wrapped duplicates, escaped-equivalent keys, malformed Unicode/trailing
input, huge integer/decimal/exponent values and ordinary private-token-looking keys
are covered. Numeric value preservation allows equivalent exponent normalization
(1e400 serializes as1e+400); byte-canonicalization is not claimed.

The complete workspace already enables serde_json arbitrary_precision and raw_value
(cargo tree --locked --offline -e features -i serde_json). Native dependency features
will name both explicitly, so correctness does not rely on unrelated feature unification.
The core legacy reader remains unchanged. Probe is not linked until the preceding
full gate finishes; its tests cannot establish protected host I/O or release readiness.

## Verified native checkpoint — 2026-10-03

Full gate83384 completed exit0: explicit Rust1.98.1, pinned ESS0.45/AEP0.65,
connectors-build gate --msrv, bounded2jobs, wrapper disabled, task TMPDIR.
Workspace test log:1285passed0failed65ignored159summaries. Authored fmt, workspace
build/test/Clippy, independent adapter library boundaries, Rust1.88 selected libraries
and Rust1.91 workspace/all-targets passed. Native launch10/type4/session8 regenerated
artifacts match. ESS scenario synthesis writes498scenarios (43authored),21refusals;
its view-coverage notes and runtime obligations remain explicit, not executed I/O
conformance or proof of full MCP support. Log: .local/mcp-runtime/gate-lease-restored.log.

Publish this native checkpoint on unit/mcp-local-runtime-20261003. Keep the story
active and CLI compatibility deferred. It contains launch parsing, names/framing,
generated session reducer, timed lease decisions and the host fixture repair.
The six-test lossless JSON probe remains separate and will be integrated next.
Actual protected host admission/audit port, process supervision/I/O, inbound families,
outbound HTTP/custody and final release acceptance remain required.

Pinned generated output includes whitespace in CLI help and an EOF blank line in
types.rs reported by git diff --check. Preserve exact generator bytes; authored
source whitespace is clean. No generated file is hand-repaired to hide those findings.

## Published checkpoint and linked JSON decoder — 2026-10-03

Native foundations are published on unit/mcp-local-runtime-20261003 at commit
4dde21d3fac77b15bd122a1be6e287558d09762f (bot author and committer verified), draft
https://github.com/beyond10x/connectors/pull/84 (bot author verified). Full local gate
log SHA256:077d7917acec7a5addb7647e60c1efa6428e866bf848fac6dfc3c17d95708978.
The first commit attempt was refused for a personal home path in the planning
specification; replaced it through AEP with the worktree id and repository-relative
locations, revalidated, then the same Gates commit/push route passed. No hook bypass.
PR common security, docs and planning checks passed; repository gate is still running
as of this observation. Nothing is merged or released by this checkpoint.

Following publication, the tested lossless decoder is linked as runtime/src/json.rs,
exported with explicit serde/arbitrary_precision/raw_value dependencies. Native tests
now35pass0fail0ignored on Rust1.98.1 and1.88; Clippy1.98.1 and authored native formatting
pass. Logs native-tests/json-{linked,msrv,clippy}.log. The complete workspace already
selected both serde_json features before this change; core legacy behavior is untouched.

The JSON addition remains uncommitted and is not part of PR84's published head or its
preceding full-gate result. Cover the complete next implementation with the required
gate before its next commit. Continue the protected host port and actual process
journeys; component tests, draft PR and native generation do not complete this story.

## Protected governed read phase — 2026-10-03

Implement the first real protected host read interface behind the existing verified
local-owner socket/build handshake. It returns complete service-response JSON bytes,
not an unaudited business Value. The native MCP facade will decode those bytes with
the lossless codec; existing CLI/private invoke remains unchanged.

Move the provider-independent JSON decoder into connectors-core::json and re-export
it from connectors-mcp::json. The host must not depend on a native adapter. Explicit
serde_json precision/raw-value features remain declared by the owning generic crate;
legacy read_json and canonicalization are unchanged. This is a generic wire-codec
obligation already recorded by service_wire, not a new domain entity or authority.

Current owner config and executable-bound cached descriptor are checked for each
read. The operations allowlist supplies local lookup admission before descriptor
revision and private existence. Local single-owner target/result scope remains the
verified configured owner; an MCP request cannot choose another Connection. Profile
permissions govern credential use at execution preflight, not a fabricated provider
credential probe during discovery. Recheck operation admission in the serialized
worker before startup/dispatch; preserve legacy error order on the legacy path.

Use existing execution_audit Store/Anchor/FinalObservation: acknowledge an admission
before provider work, confirm its live receipt, and append final observation separately.
No acknowledged anchor means no dispatch and unavailable audit with null reference.
A failed final append retains the known business outcome with incomplete audit and
the real acknowledged ref. Early refusals retain only verified coordinates. No read
retry or synthetic audit reference. Audit policy/metadata is local and actual; test
these failure seams with the real store and controlled provider callback.

Add a distinct private governed-read request and raw response reader. Refuse malformed
or oversized private frames and mismatched build as existing protected calls do.
Preserve exact provider JSON text through its child and owner transports until strict
lossless decoding, and retain complete safe response/audit metadata. Existing generated
shared public observation values and compatibility.md sections4-5 own the envelope;
no second approval/attempt/audit ledger or caller relation is introduced.

Coordinator acceptance/design/scope review: this generic seam is prerequisite to
real MCP invocation, not a server-support claim. Scope now includes core's generic
JSON codec and tests as well as the already selected host/native/runtime surfaces.
No parallel writer; independent reviewers remain quota unavailable. Required tests:
policy beats stale/existence, stale beats absence/input, no dispatch on audit failure,
known outcome survives final-audit failure, raw numeric precision and legacy behavior.

## Governed read implementation checkpoint — 2026-10-03

Implemented the selected protected read seam in the existing host owner: a distinct
GovernedRead request, same-build client check, raw complete response bytes, serialized
worker policy recheck before child startup, lossless child output validation, and the
existing registry capture/custody/final dispatch sequence. The legacy Invoke decoder,
error precedence and result shape remain unchanged. Governed registry NotReady and
InsufficientScope now retain connection_not_ready and insufficient_scope rather than
using the legacy not_granted collapse.

The real execution-audit Store acknowledges and confirms an anchor before invoking
the provider callback. Current lookup policy precedes revision/existence/input errors.
Only verified coordinates enter refusal records; no caller alias or guessed connection
is turned into a registry namespace. If the required namespace or audit acknowledgement
is unavailable, the response has unavailable audit and null ref, with no dispatch.
Bounded recovery reuses the exact final observation under the original deadline;
failed recovery preserves the known business result with incomplete audit and the
acknowledged ref. Complete-envelope overflow returns capacity, never partial JSON.

Rust1.98.1 targeted checks (bounded jobs, locked/offline, task TMPDIR):
- cargo test -p connectors-core -p connectors-mcp: 49 passed, 0 failed, 0 ignored;
  includes six lossless tests in each owner/re-export and eight legacy core tests.
- cargo test -p connectors-host --lib governed: restored source 9 passed, 0 failed,
  0 ignored. Seven real-store coordinator tests, one private socket/client test,
  and one actual owned child-process transport test. This is not an end-to-end
  production MCP server, OAuth connection or live-provider acceptance run.
- cargo clippy -p connectors-core -p connectors-host -p connectors-mcp --all-targets
  -- -D warnings: exit0.
- Audit guard mutation allowed dispatch after an unacknowledged anchor: the exact
  negative test failed with `unacknowledged read dispatched` (0 passed, 1 failed,
  exit101). Original source restored byte-identically; the nine tests then passed.

Task evidence under .local/mcp-runtime/native-tests: shared-json.log,
governed-wire-tests.log, governed-recovery-tests.log, governed-clippy.log,
governed-mutation-audit.log, governed-restored.log. Mutation log SHA256
 e269cc84827d175d4ee3cf195dd788ccc8363e93a158bb676b344614860f2234.
These logs are local supporting evidence; the named tests and this AEP record are
retained source evidence. The complete repository gate is next, not yet claimed.

PR84 foundation commit 4dde21d3fac77b15bd122a1be6e287558d09762f has all four checks
passing, including repository gate run37121814005 (23m46s). Current JSON/governed
changes are not on that published commit. No merge, new release, independent review
or finished runtime is claimed. Independent workers remain unavailable on quota;
this implementation and review were the coordinator's.

The full local inbound and outbound HTTP goals remain active. Next: validate this
batch with the full gate, then finish the generated projection configuration,
audited discovery and actual server I/O/clock boundary; deliver outbound protected
OAuth/persistent invocation and the final verified source release. Existing cloud
assignment/outbound child ownership decisions remain explicit rather than implied.

## Local projection configuration selection — implementation decision, 2026-10-03

The local inbound binding selects an owner-only UTF-8 JSON companion named by
appending `.mcp-stdio.json` to the exact configured owner config path. For example,
`/private/config.toml` selects `/private/config.toml.mcp-stdio.json`. There is no
implicit default exposure and no extra launch flag. The application composes this
native configuration with the generic owner; generic Config remains closed and
contains no native provider value bag. The file is limited to 1 MiB and opened
through the existing descriptor-relative, no-symlink private-file admission.
Missing, unreadable, ambiguous, oversized or malformed configuration refuses startup
or the current request. It never means a successful empty projection.

The already validated native local_server.Configuration/Exposure/Family/Limits
values own the shape. Select format `connectors-mcp-local/1`. Limits must be positive
unsigned integer JSON tokens (no fractional/exponent spelling), bounded for this binding: frame_octets 256..1048576, response_octets 1024..33554432,
concurrent_requests 1..16, request_milliseconds 1..120000. These are deployment
ceilings; they cannot relax the shared 2000ms data lease or 5000ms teardown bounds.
Unlike the owner transport's 8 MiB service document ceiling, the native response
ceiling allows 32 MiB so a tool can retain its complete structured Response and
its escaped JSON text representation with framing overhead.
Count the framing newline and all duplicated text/structured representations in
the wire budgets. Complete-binding eligibility still checks every operation's
own declared budgets before advertisement and dispatch.

Allow at most 1000 exposures, each with valid existing owner selectors, a nonempty
set of unique selected families, and no duplicate (adapter_alias,operation_ref)
entry. Different configured connections may not alias one advertised operation.
An optional approval_file is an absolute bounded local path; its mere selection
never opens it, proves its existence, or authorizes a write. It remains private and
is consulted only by the admitted new-mutation path. The replay path never reads it.
JSON member duplicates, unknown members, explicit null for optional String,
private-token-shaped objects pretending to be numbers, and invalid limits refuse.

Read the admitted configuration anew for every discovery page and invocation.
Changing selection, enablement or configured target invalidates the affected
projection revision. Do not implement a startup-only snapshot: projection.md
requires withdrawal at the next admitted list. The serialized owner must repeat
or fence the same selection at dispatch. This is an application-composed metadata
policy seam, not a new provider grant, caller identity or ledger. The exact generic
callback/request shape remains to be implemented and held to TOCTOU tests before
this configuration is advertised as usable. Local single-owner semantics do not
answer the separate cloud caller-assignment question.

Promotion order: add the validated native domain to its owning ESS header, generate
the four types into a dedicated sibling package, extend the pinned drift gate,
implement bounded validation, then compose protected file admission and the owner
metadata/discovery/dispatch fence. Do not publish a server command while these
admission and I/O obligations are incomplete.

## Governed read full-gate checkpoint — 2026-10-03

The required gate completed with exit0 in session98514:
`RUSTUP_TOOLCHAIN=1.98.1 RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 TMPDIR=$PWD/.local/tmp
CONNECTORS_ESS=<pinned0.45.0> CONNECTORS_AEP=<pinned0.65.0>
cargo run --locked --offline -p connectors-build -- gate --msrv`.

Observed 1306 passed, 0 failed, 65 ignored across161 test summaries. Native and
shared projections match pinned ESS; formatting, workspace build/tests/Clippy,
isolated adapter builds and Rust1.88 selected libraries/Rust1.91 all workspace
all-target checks passed. Shared ESS synthesis retains498 scenarios (43 authored),
21 named synthesis refusals and its existing unpublished-view-field notes; the local
metadata authority conformance ran289 scenarios. This does not convert synthesis
obligations into actual MCP runtime conformance or clear the65 ignored tests.

Full log .local/mcp-runtime/gate-governed.log, SHA256
4afa0df75d9fffd4a77d486431d8ad3c49f455a149364a1cdfaba5d6b03f148d.
Publish the governed read/core JSON source on the existing draft PR84. Keep the
runtime story active and the CLI compatibility deferred. No new release is claimed.

The next configuration validator has an isolated Rust1.88 prototype with6 passed,
0 failed, 0 ignored (probe/configuration/refined-tests.log). It uses the four
already validated generated native values, protects Number deserialization from
literal private-token objects, rejects duplicate/unknown input and conflicting
exposures, and never opens the selected approval source. Its32MiB native ceiling
allows room for both tool result representations around the8MiB owner envelope.
This prototype is not linked source and is not included in the completed gate.
Promotion and application/owner admission remain the next implementation batch.

An asynchronous operator question about host-owned cloud connections with explicit
caller grants is pending. No answer has been assumed and no blocker was cleared.
Local inbound stdio and outbound HTTP continue independently.

## Native configuration integration checkpoint — 2026-10-03

The governed-read checkpoint was bot-committed and pushed on the existing branch:
f77dcd61224b31474dc102c2dbe847333c8fb4fa. Both author and committer were verified as
b10x-bot[bot], the advertised branch matches, and draft PR84 names that exact head.
The bot API updated its title/description to the combined final scope. Current CI:
common source, documentation and planning checks succeeded; repository gate
run37124861921/job111207994769 is still in progress. No merge or release was made.

Continued the already selected native configuration phase after that publication:
- Added local_server to the native ESS header: 5 files validate under ESS0.45.0.
- Generated four configuration values into the dedicated configuration-types sibling
  package; root workspace exclusion and native dependency keep generated manifests
  intact. The build now regenerates and compares this projection with the existing
  launch/session projections. Local output ledgers remain ignored.
- Added native Configuration validation and six tests for bounded input, closed JSON,
  literal private-number objects, numeric limits, duplicate/conflicting exposures,
  exact selectors and local approval source shape. Parsing never opens a proof file.
- The CLI intent guide now documents the selected companion filename and bounds,
  while explicitly retaining production runtime unavailability. Application protected
  file loading, per-request metadata admission and final dispatch fencing are next.

Linked native tests: 41 passed, 0 failed, 0 ignored under Rust1.98.1 and Rust1.88.0.
Pinned `mcp-bindings --check` passed. Native Clippy all-targets with -D warnings
passed. Logs: native-tests/configuration-{linked,msrv,drift,clippy}.log under the
existing task evidence directory. A first generator invocation used an unsupported
--ess flag and exited2; its help named only --check, and rerunning the same command
with the documented CONNECTORS_ESS environment pin succeeded. No generated file
was repaired by hand. The full gate on f77dcd612 predates this configuration batch;
these new changes are local and await the next complete integration gate.

The story remains active. Neither typed configuration parsing nor the earlier
protected read API is the actual-process MCP acceptance promised by this story.
The pending cloud caller-assignment question has no answer yet and remains open.

## Configuration Clippy evidence correction — 2026-10-03

Evidence20261003T131003Z-000-e41ccd04991d prematurely said native Clippy passed.
The completed initial command actually exited101 on clippy::collapsible_if in
configuration.rs. The coordinator wrote that claim before checking the returned
exit status; it was not valid evidence of Clippy success. The 41-test and ESS/drift
results in that record were observed and remain accurate. Keep the earlier record
as history rather than editing an immutable evidence record.

Collapsed the named conditional without suppressing the lint. The same native
all-targets Clippy command now completed with exit0, observed directly. The failed
log remains native-tests/configuration-clippy.log; the successful retry is
native-tests/configuration-clippy-fixed.log. This correction supersedes only the
premature Clippy claim. Full integration/release acceptance remains unfinished.

## Protected metadata and native selection seam — 2026-10-03

Continue the active local runtime story with a generic owner ReadPolicy callback
installed by application composition. The host must not depend on a native adapter.
A projected private read supplies an additional expected projection revision; the
callback rereads the admitted native companion file and checks the exact configured
operation/connection and enablement after generic current lookup policy and before
private operation lookup/input. The original generic governed-read call retains its
behavior, and the default owner refuses a projected call when no policy is installed.
This extra constraint grants nothing and cannot bypass generic host policy, registry
admission, audit acknowledgement, credential custody or the final dispatch gate.

Pass the same application-owned policy to the serialized worker. Recheck at dequeue
before startup, and again at final dispatch while holding the existing lifecycle
control guard. A changed projection revision refuses stale input; a mismatched
configured connection refuses scope before revealing revision/operation information.
Policy file failure is unavailable rather than empty metadata. Only composition knows
the native filename and typed configuration; no native value bag enters generic Config.

Add a protected metadata query over the existing runtime Bootstrap carrier. It reads
current generic config and exact executable-bound cached metadata, filters operations
by current metadata policy, and returns only admitted metadata, without a provider
probe, credential access, child startup or approval source read. Discovery audit uses
the existing audit store and no synthetic reference. Application projection owns the
safe MCP presentation; private metadata is not blindly copied into protocol output.

Required checks cover actual protected file admission, unknown/duplicate/excess input,
revocation/change between discovery and worker dispatch, policy-before-stale refusal,
no child startup on policy refusal, metadata queries with absent credentials/provider,
and unchanged legacy invocation behavior. New runtime helper ports are implementation
composition of existing typed declarations; they introduce no caller, grant or ledger.
No production server availability or completed process conformance is claimed yet.

## Protected policy and metadata implementation checkpoint — 2026-10-03

Implemented the application-owned ReadPolicy port through the protected owner
socket, read coordinator and serialized worker. The application reloads the
private native configuration for each check. Host operation policy precedes native
policy; selected connection scope precedes projection revision; current revision
precedes enablement and operation input. The worker checks before provider startup
and again at final dispatch. No native adapter dependency enters the host library.

Added protected GovernedDescribe and Client::governed_describe. It validates the
exact executable-bound cached Bootstrap, filters operations and requirements by
current metadata policy, acknowledges a real Describe audit anchor, rereads policy
and metadata, and appends the final observation. Public correlation is null for
describe. Audit admission failure withholds metadata and returns unavailable with
no invented reference. Final audit failure retains the known result and real
reference as incomplete. This private carrier is not an MCP list or public codec;
application-owned field projection remains to be implemented.

Observed focused checks, Rust 1.98.1, locked/offline and bounded build settings:

- Application mcp::tests: 4 passed. Initial 4 failures were invalid test setup:
  Config::initialize writes connectors-local/2, requiring explicit private_protocol.
  The fixture now selects V2. Production validation was not relaxed.
  Log .local/mcp-runtime/native-tests/policy-app-tests-fixed.log,
  SHA256 608dbbc60c32656a9e7711d4b209d512ea3497965e6e44011862695df3515b45.
- Host governed filter: 16 passed, including socket framing/build admission,
  real child JSON transport, 3 new policy checks and 3 metadata checks.
  Log .local/mcp-runtime/native-tests/metadata-tests-fixed.log,
  SHA256 d476a76ee1a499eec430222eb9cf793a074174d777c1d50f17e8cd91d6e3cc66.
  A test initially expected EarlyRefusal's Refused outcome after an already
  acknowledged admission; corrected to the existing Error final observation,
  retaining its no-dispatch and real-record assertions. Metadata final-write
  fault injection initially also hit admission; moved injection after confirmed
  admission so it exercises the intended final-write seam. Failures remain in logs.

The extra metadata-policy-change-after-admission test was added after that focused
run and is not covered by those 16 passes. The full gate including MSRV is now
running in .local/mcp-runtime/gate-policy-metadata.log; no passing claim yet.
PR84 remote repository gate 37124861921 completed success on published f77dcd6;
it does not cover these uncommitted changes.

Remaining: a real final-dispatch withdrawal fixture with custody/child, native
list/result projection, production stdio launch and process conformance, outbound
HTTP/OAuth composition, mutation binding, cloud assignment decision and release.
No server runtime or completed provider batch is claimed. Independent workers
remain unavailable under the previously observed quota; these checks are the
coordinator's work, not independent review. Story remains active.

## Policy and metadata full gate — 2026-10-03

The full repository gate finished with exit 0 on the native configuration,
application policy port and audited metadata implementation. Command:
`RUSTUP_TOOLCHAIN=1.98.1 RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 TMPDIR="$PWD/.local/tmp" CONNECTORS_ESS="$HOME/.cache/ess/toolchains/0.45.0/ess" CONNECTORS_AEP="$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep" cargo run --locked --offline -p connectors-build -- gate --msrv`.

Observed 162 test summaries: 1324 passed, 0 failed, 65 ignored. Includes the
metadata-policy-change-after-admission test added after the focused run. Pinned
ESS validation and generation/drift, authored formatting, workspace build/tests,
adapter isolation, Clippy all-targets with warnings denied, Rust 1.88 selected
libraries and Rust 1.91 workspace all-target checks all passed. AEP validation is
valid with existing historical review warnings. Full unedited output is retained
at .local/mcp-runtime/gate-policy-metadata.log, SHA256
8efade3cc9917a8859c338e9d47c78620ce4cd4a189765e393cc1b44924ec94f.

This checkpoint is to be published on the existing draft PR84 branch. No merge,
tag, release or MCP process conformance is claimed by a green repository gate.
The story remains active and retains all runtime, provider and cloud decision gaps
listed in the preceding checkpoint.

The first commit attempt was refused by Gates for a personal absolute toolchain
path in this command transcript. The transcript now uses HOME-relative paths;
code, generated output and validation inputs are unchanged. The same bot/Gates
route is retried without changing policy or suppressing its check.

## Final dispatch fence proof — 2026-10-03

Prove the selected worker final-dispatch policy check with the existing disposable
GNOME Secret Service fixture, real registry publication and an owned Rust adapter
child. Admit at dequeue, withdraw before final dispatch, assert no provider marker,
then allow the same connection and assert one provider invocation. Keep this
environment-dependent scenario explicitly classified in the ignored-suite runner.
Plant the defect by omitting the final policy recheck only, require this exact
test to fail, restore byte-identical source and observe green. This proves the
worker constraint, not the unfinished MCP wire runtime or full conformance suite.

Continue native MCP discovery/result projection after this check. Public fields
must be explicitly projected from admitted metadata; private Bootstrap, approval
paths and credential locators are never copied wholesale to MCP output. Preserve
the full server, outbound, mutation and release scope and the pending cloud choice.

## Verified final fence and result projection phase — 2026-10-03

The real final-dispatch fixture now passes with a qualified disposable Secret
Service, actual registry/custody acknowledgements and a real owned Rust adapter.
It denies the second (final) admission, sees no provider invocation and verifies
the captured use is released without dispatch. Allowing the next call proves the
same connection executes exactly once and releases its use. Removing only the
worker final policy callback makes this test fail with "withdrawn projection
reached the real provider". Restored supervisor SHA256 is
f855aef1ec5fed762f5beab83b46f5b70250f2b9560a995160ed8dc8e0c9c969.
Logs: native-tests/final-fence-mutation.log and final-fence-released.log under
.local/mcp-runtime. The initial long-checkout TMPDIR failed before testing admission
because the fixture bus could not open; a task-owned short private cache directory
ran the qualified fixture. No production timeout or custody admission was relaxed.
The ignored runner classifies this exact test as disposable with CUSTODY required.

Next implement pure native tools/resources result projection and whole JSON-RPC
response encoding from an already admitted safe v1alpha2 service envelope. Preserve
all applicable service/audit/source/mutation fields without regenerating correlation;
tool text and structured content must decode identically, including arbitrary JSON
number tokens. Primary cacheable resource replies get ttlMs:0/cacheScope:private;
legacy omits primary-only members. Budget the complete frame including id, wrappers,
duplicate representations and newline; refuse capacity rather than truncate.
Protocol request admission, public list eligibility and launch remain separate.

Source constraint found during discovery wiring: core::Operation's current legacy
carrier has no extended effects/risk/idempotency/approval/limits declarations, while
contracts/service/compatibility.md section4 and MCP projection require those facts.
The runtime must obtain complete reviewed curation before advertising a selected
operation; do not infer missing declarations from a package name or fill guessed
defaults. The protected metadata query is a private Bootstrap, not a complete
extended public descriptor. This remains part of full delivery, not a scope deferral.

## Result codecs and final fence checkpoint — 2026-10-03

Added native results.rs for admitted protected-owner replies. The envelope shell
rejects duplicate keys, legacy versions, inconsistent audit status/reference pairs,
invalid correlation and dual result/error objects. Business payload and admitted
source/mutation observations are preserved rather than reduced. The owner remains
responsible for admitting those observations; the helper is not a public authority
or a substitute for their complete schema validation.

Resolved tools retain identical structuredContent and JSON text, with original
host/attempt correlation intact. Resources carry the canonical URI and full successful
response as JSON text. Declared prompts preserve ordered messages/description and the
complete response in the selected metadata coordinate; their exact message/content
schema and family eligibility must already be admitted. No prompt is synthesized
from an arbitrary dataset. Resource/prompt failures use the safe JSON-RPC error data
carrier; pre-resolution tool failures use that same channel. Primary-only complete
and private/zero-TTL cache fields are omitted for interoperability replies.

Whole response encoding reserves the newline and accounts for all JSON-RPC overhead,
IDs, escape expansion and duplicate representations. A bounded writer refuses rather
than truncates. Numeric request IDs and business JSON remain lossless.

Rust 1.98.1 locked/offline native tests:47 passed,0 failed. Six result tests cover
representational equality including 1e400/large integers/literal private-number objects,
scalar resources, declared prompts, error channels, malformed envelopes and exact byte
boundaries. Temporarily omitting only the newline reservation made the exact boundary
test fail by returning an oversized frame. Restored result source SHA256:
421781ec53d6200803a898c521cd1efdc3bb81583eae990b1ee81fdd6db8dd37.

Evidence logs under .local/mcp-runtime/native-tests:
- results-budget-mutation.log SHA256 7132a8bb8e01a81e8f8da7f3f47fea5dacedb21810a96a41dc9cb3cb0ffeddb6
- results-restored.log SHA256 7a83097eeec6aa97e4f24efa4907462fc17a7144934fb798588b845e1b33e7f0
- final-fence-mutation.log SHA256 7730f3716c0350e18019d454ca1e847184e600deb342ecadbe294fc81a994232
- final-fence-released.log SHA256 b1418a4024d31f4216e096ab2e97dddb6d226b44d3766abede61602c9fdf4dd8

The final-dispatch fixture passes with real disposable custody/child and verifies
registry uses (1 captured,0 dispatched,1 released) after refusal and (2,1,2) after
the subsequent allowed request. Its short fixture directory was removed after
processes stopped. The exact ignored test is classified with the CUSTODY prerequisite.

Full repository gate including MSRV is running in gate-results-fence.log; no passing
claim for that run yet. Remote PR84 repository run37127847891 succeeded on13d9184e4,
which predates this batch. Public request admission, declaration/limits curation,
native discovery, actual process supervision, outbound auth/runtime and release remain
unfinished. No lifecycle move, process-conformance claim or scope reduction.

## Exact private discovery revision — 2026-10-03

The next private discovery step must preserve the projection revision over the exact
unfiltered cached Bootstrap used by execution. Current governed_describe filters
operations and requirements before returning Bootstrap; recomputing its digest in
an MCP caller would therefore disagree whenever host policy hides an operation.
The app already owns the revision function and the generic owner must not acquire
native adapter dependencies.

Add an explicitly selected protected projected_describe query. Its application
ReadPolicy computes a bounded revision from the original validated adapter/Bootstrap
snapshot. Generic metadata filtering still precedes release. Return the private
filtered Bootstrap and projection revision together only after acknowledged audit,
then repeat both host and projection snapshots and refuse if either changed. The
ordinary governed_describe carrier and legacy adapter Bootstrap codecs stay intact;
the existing same-executable owner handshake gates the new private request member.
An unbound projection policy must refuse, not return an empty successful revision.

Tests must cover a hidden operation (returned revision equals execution's original
snapshot), companion change after audit acknowledgement, host permission withdrawal,
unavailable projection policy, malformed revision, same-build socket selection and
no provider/credential/approval access. This is necessary discovery plumbing, not
complete public extended declarations. Operation curation and exact receiver limits
remain mandatory before any MCP operation is advertised; missing risk/idempotency
facts must not be manufactured from a legacy package or profile name.

## Verified result and final-fence gate — 2026-10-03

The complete repository gate finished successfully (exit 0) after the result codecs
and real final-dispatch fixture were added. Command: Rust 1.98.1, bounded Cargo jobs,
locked/offline connectors-build gate --msrv, with the repository-pinned ESS 0.45.0
and AEP 0.65.0 and task-owned TMPDIR. It includes generated-output drift, native/shared
ESS checks, workspace tests/build/Clippy, adapter boundaries, Rust 1.88 selected
packages and Rust 1.91 workspace all-target checks. Aggregate libtest output:
1331 passed, 0 failed, 66 ignored across 163 summaries. Ignored tests are not counted
as passing; the new real final-fence fixture was explicitly run and mutation-checked
as recorded separately. No MCP process conformance or independent review is claimed.

Gate log: .local/mcp-runtime/gate-results-fence.log
SHA256: 53a7626686e5bf4de7910dc73692414871b2bf0edd5a1643b7a3dfa21f21ab97.
AEP validation ends valid, with existing historical review-outcome warnings retained.
The next protected metadata revision change is planned but not included in this run.

## Published checkpoint and compiled ignored inventory — 2026-10-03

Published checkpoint ae895e5021c7dc856b58498e31af3375c945c053 on
unit/mcp-local-runtime-20261003; author and committer are b10x-bot[bot], and
origin's exact advertised branch SHA was verified after the Gates bot push.
PR84 was updated through the App API. The repository gate is running remotely;
security/privacy, planning and documentation checks have passed for this checkpoint.

The compiled ignored-suite inventory completed with exit 0: 148 test binaries,
66 ignored entries, 42 selected disposable entries, 0 unclassified, 0 executed.
This was inventory-only, not a live-suite result. It reported 39 missing prerequisites,
including the long checkout TMPDIR for private Unix sockets. The exact new final-fence
scenario is classified disposable and requires qualified custody; its actual execution
and restored green result used the separately recorded short private fixture directory.
Report .local/mcp-runtime/ignored-results-fence.json SHA256
 a9e131b3f56df824917afa291f0c0e91eff7163c4c9062844ccc3480439f4828.
No missing prerequisite was relabeled as a passing test.

## Protected discovery revision verification — 2026-10-03

Implemented an explicitly selected ProjectedDescribe private request and
Client::projected_describe. Existing GovernedDescribe and legacy Bootstrap shapes
remain unchanged. Both queries require the exact owner executable build before any
request bytes are sent. The application ReadPolicy now supplies metadata_revision
from the original validated adapter/Bootstrap and freshly admitted native companion.
An unbound implementation returns Unsupported; malformed/empty revisions fail closed.

The generic owner computes the private projection before filtering operations, pairs
its revision with the filtered Bootstrap, and binds both host and projection snapshots
in its admission fingerprint. After the real audit acknowledgement it reloads both;
any changed fingerprint returns stale_description, and unavailable policy returns
unavailable without metadata. Public discovery still must explicitly project safe
complete declarations; neither private carrier may be copied into MCP output.

Focused Rust 1.98.1 locked/offline verification:
- owner governed tests: 19 passed,0 failed,1 explicitly ignored real-custody fixture;
- application MCP tests: 5 passed,0 failed, including an extra hidden cached operation
  with a revision accepted by execution and rejection of the filtered-cache digest;
- protected discovery socket/build test: 1 passed, exercises both query variants.
These are implementation tests, not completed MCP process conformance.

Planted defect: omit only projection_revision from the metadata admission fingerprint.
The exact projection_change_after_real_audit_acknowledgement_releases_no_metadata test
failed: it expected stale_description, but the response had no error. Restored the
source byte-for-byte and observed the 19-test governed suite green. Restored metadata.rs
SHA256 d5595daab70246625a0b5cbbaedad616fd40cbcc5af3f9984d808222bbdbb5bb.
Evidence under .local/mcp-runtime/native-tests:
- metadata-revision-mutation.log SHA256 59b56ae8f63084d371abb7fcf75c87a4085c27b1c374fab90dce7a9c6206c690
- metadata-projection-restored.log SHA256 6049f4c597bab6d1fc4b99671c63706f00bf3cc8598d5cb728cefa0106aa8647
- metadata-projection-app.log SHA256 d076d0ad021ffa9195b6da2f464d7d79d660d3bb556ff15bb3695eeb5cf48f83
- metadata-projection-socket.log SHA256 7a15d77dbfc86e74641ca23e873918f5d07a1a9f4cf3ab4f8987375c33ffc364

The full gate including MSRV is running in gate-metadata-revision.log. No passing
claim for that run yet. Full operation curation/limits, public wire/lifecycle and
outbound delivery remain open. The operator was asked to resolve the existing
outbound stdio and Helm execution-family choices; no answer is assumed and no blocker
is cleared. Local inbound and outbound HTTP work remain independent of those choices.

## Private metadata revision full gate — 2026-10-03

The private metadata revision change completed its full required gate at exit0.
Command: RUSTUP_TOOLCHAIN=1.98.1 with bounded Cargo jobs and task-owned TMPDIR,
CONNECTORS_ESS=$HOME/.cache/ess/toolchains/0.45.0/ess and
CONNECTORS_AEP=$HOME/.cache/aep/toolchains/0.65.0/aep-0.65.0-x86_64-unknown-linux-gnu/aep,
cargo run --locked --offline -p connectors-build -- gate --msrv.
The workspace commands inherit the selected Rust1.98.1 toolchain; the runner selects
its explicit Rust1.88 and Rust1.91 MSRV checks internally.
Result: 1336passed,0failed,66ignored across163 summaries, plus specification,
generation drift, isolation, Clippy and required MSRV checks. Log retained locally as
.local/mcp-runtime/gate-metadata-revision.log; SHA256
 dda0789227b7ca166fe6d876286ef374e4d5fcded594f79e8458e7dbeaaca719.

This gate precedes the subsequent outbound process model/selection reconciliation;
it is not evidence for those later edits. Those require another combined gate.
The projection revision mutation failed at the intended post-acknowledgment race
assertion and the restored suite passed, as recorded in the preceding checkpoint.

Published predecessor ae895e5021c7dc856b58498e31af3375c945c053 has now completed
repository CI37130050243 successfully, verified against that exact head. Docs,
planning and security checks were also successful. PR84 remains draft, full inbound
and outbound MCP runtime remains incomplete, and no new release has been cut.

## Combined metadata and process-model gate — 2026-10-03

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

## Shared operation metadata phase — 2026-10-03

The next required runtime phase is complete safe operation metadata and enforcement
of its declared limits before MCP discovery is advertised. Current core::Operation
has only the six legacy fields, while private Bootstrap/Requirement adds only auth
requirements and Read/Write/Unknown. It cannot supply the extended service metadata
required by contracts/service/compatibility.md §4 and the native projection contract.
Do not infer risk, approval, idempotency or budgets from descriptive text or effect alone.

First implement the shared strict metadata value codec from its owning ESS types.
The added connectors.service_wire.OperationMetadata and OperationIdempotency values
reuse shared effect, risk, approval, realization and limit types; they introduce no
entity or ownership relation. ESS0.45 validation: connectors v1 —24 file(s),valid.
Generated Rust types live under crates/connectors-core/generated/operation-types;
connectors-build service-metadata owns generation and the full gate checks drift.
The core codec must reject missing/unknown fields, forged numeric objects, duplicate
members/effects, incompatible read/mutation effects, unresolved realizations and
invalid keyed-only metadata. Optional absence is distinct from null. Parsing does
not prove current grants, native natural-idempotency assumptions, effect completeness
or actual budget enforcement; the receiver's selected binding must establish those.

Named checks for this phase: operation-metadata-required-fields,
operation-metadata-lossless-numbers, operation-metadata-effect-discriminator,
operation-metadata-keyed-shape, operation-metadata-optional-presence and
operation-metadata-canonical-roundtrip. A planted read/external-write discriminator
bypass must fail before restored tests and the required gate. Existing legacy
Operation/Descriptor/Bootstrap codecs remain closed and unchanged by this phase.
A complete admitted metadata source and provider/request/result/time enforcement
remain subsequent integration work within this same runtime story, not optional
follow-ups or reasons to accept an empty catalog.

Implementation evidence: the new six-case suite first failed0passed/6failed against
an always-refusing stub, then passed with the generated-type-backed codec. The
complete restored core suite passes21 tests,0failed,0ignored, including the added
closed legacy-codec compatibility case. Omitting only the external_write/mutation
iff check caused operation_metadata_effect_discriminator_cannot_be_replaced_by_a_hint
to fail on a read carrying external_write; the source was restored byte-for-byte.

Retained logs/SHA256 under .local/mcp-runtime/:
- operation-metadata-red.log:
  d177b1eeaab7fc53a29cd32c160579547ce60e855d4bfa855899b4972ccdfdd6
- operation-metadata-mutation.log:
  64ff3ae6742caaeea6b4ac940fb40d44d63fc64a60ad96ea8577b463f5d80e2c
- operation-metadata-core-restored.log:
  c030bbb05acfb6260a0144dd02e79aa9079f112872a34d88b994d4b83ef45ab7
- operation-metadata-drift.log:
  60372ebb7b387b466210f9341f2016d765f220f1072a90755df1d28094e5c033
The existing CLI/MCP projections still match. Metadata Entity Runtime definitions
were regenerated through their owner: only source_digest and synthesis_digest
changed; definitions and command bindings are byte-value identical under parsed
JSON comparison. No persisted entity schema or command was added or changed.
The full required repository gate completed with exit0: 1344passed,0failed,
66ignored across164test summaries. Command: cargo run --locked --offline
-p connectors-build -- gate --msrv with RUSTUP_TOOLCHAIN=1.98.1,
CONNECTORS_ESS pinned0.45.0, CONNECTORS_AEP pinned0.65.0, RUSTC_WRAPPER empty,
CARGO_BUILD_JOBS=2, CARGO_INCREMENTAL=0 and task-owned TMPDIR=.local/tmp.
The gate also checks its declared MSRV lanes and projection drift.
Log: .local/mcp-runtime/gate-operation-metadata.log; SHA256:
8d6f05e3f98055b2a1595952cd66816af142ece1552001eda39e63c77e44243e.
AEP validation is valid with historical review-outcome warnings retained in the
full log. This is coordinator verification, not an independent adversary verdict.

Next integration constraint, verified in source: crates/connectors-host/src/http.rs
currently builds HTTP clients with fixed5-second connect and15-second request
limits, while service/compatibility.md §7 gives generic operations a30-second
provider budget inside40-second execution. The private runtime permits1MiB input
and8MiB result, unlike either public profile. A metadata parser cannot fix that
mismatch. Carry and enforce the selected receiver budgets through owner dispatch,
native child and provider I/O; do not merely label existing fixed limits as generic
support. Explicitly select any private-protocol evolution and preserve old-reader
refusal instead of silently adding fields to the closed Bootstrap carrier.

The published predecessor ad39b5754e04e3f4227e23b396a96db8c152f85c has now passed
repository CI37133928989, verified against that exact head. Docs, planning and
shared-source checks also passed. This remains draft PR84, with full MCP runtime
and the provider/release objective incomplete.

## Shared provider deadline phase — 2026-10-03

Implement the HTTP portion of service/compatibility.md §7 before selecting an
extended operation binding. ScopedHttp will offer an explicit trusted-composition
budget constructor: a provider interval bounded by30seconds plus the existing
absolute execution cutoff. It retains the earliest cutoff across credential,
probe and write capability derivation and repeated requests. Credential resolution,
request transmission and response body reading consume that one budget. Connect
remains bounded by5seconds and the remaining overall provider interval. Explicit
per-request remaining time must override the legacy15second client default so a
selected30second generic budget is executable. Existing callers retain their
legacy behavior until they explicitly select this capability.

This is a transport enforcement step, not public metadata admission. The owning
composition must still select exact declared budgets, carry the original total
execution deadline across processes, enforce complete service envelope byte limits,
and provide authoritative operation curation before MCP advertisement. No new
private protocol is selected by this step and no native adapter is implicitly
converted to the extended binding. The ephemeral cutoff is not a persisted entity.

Named checks: provider-budget-credential-bound, provider-budget-shared-cutoff,
provider-budget-capability-derivation and provider-budget-generic-not-clipped.
Exercise the actual HTTP transport with controlled fixture clocks where possible;
observe credential/provider call counts. First fail against a stub; plant a
deadline-reset or clipping defect and verify the targeted refusal before restoring.
Coordinator-only verification remains necessary: the three existing workers are
still quota-errored and no independent review is claimed.

Implementation verification: the initial five tests failed against the refusing
stub (0passed,5failed). With the implementation restored, the new six cases plus
existing HTTP, prefix, write and timeout-origin suites passed22tests,0failed,
0ignored. The real fixture answered after16seconds under a30second selected
provider budget. Stalled bodies hit the original cutoff for all five capabilities
(GET, prefix, consuming write, probe and form); no truncated body became success.
Deleting only the inherited-cutoff minimum caused the capability-derivation case
to fail when its write reached the provider after the original cutoff. The source
was restored byte-for-byte before the22-test run. Credential stalls terminated
without HTTP dispatch, and the existing legacy TLS/body timeout distinctions pass.

Logs/SHA256 under .local/mcp-runtime/:
- http-budget-red.log: 2b2f1a8858d707c270bca22c235fae8f192ec83234163d0dbea09124340473ac
- http-budget-mutation.log: a97c347a5fa4f3e7c23618f69d1cd366e72e9835714d5a4454343fe9acef324e
- http-budget-restored.log: f3859420dd8e37cdedecfceef0ef4f5b536424cc9e4ada0aba387cb719b05e70

The required full gate passed with exit0: 1350passed,0failed,66ignored across
165test summaries, including workspace Clippy, projection drift, independent
adapter builds and Rust1.88/1.91 checks. Command: cargo run --locked --offline
-p connectors-build -- gate --msrv, with the same pinned ESS0.45/AEP0.65,
RUSTUP_TOOLCHAIN=1.98.1 and bounded build settings recorded in the metadata phase.
Log: .local/mcp-runtime/gate-http-budget.log; SHA256:
5d63c13f237b13771f201f9a0ac2bfb5df706391a82e3b17a46f5729ba4d6132.
AEP is valid; full historical warnings remain in the log. Scope is now cited for
both crates/connectors-host/src/http.rs and tests/http_budget.rs.
The preceding metadata checkpoint54e4bed6c0678f41fa722738f1db3bf022cc1574 is published
on draftPR84; docs37135930021,planning37135929967 and shared-source37135928707 passed.
Rust gate37135930002 also completed successfully at that exact head. The worktree
cb26l-runtime remains active for unfinished story:mcp-local-stdio-runtime; root
owns the next integration and the retained logs. Actual metadata curation and
owner/native selection of these budgets remain required before advertisement.
The closed legacy Bootstrap lacks the complete declarations and original
execution context; do not manufacture them from its Read/Write flag or silently
extend its wire carrier. Explicitly select and model that integration next.

## Protected curation and coherent projection — 2026-10-03

Selected integration: protected CONFIG.operation-curation.json supplies complete
reviewed operation declarations, pinned to both Adapter::selection() and the
canonical digest of the original validated Bootstrap. The normative source is
contracts/service/local-operation-curation.md, modeled as immutable values in
ess/domains/service_wire.yaml. ESS0.45 validates24files. No operation or persistent
entity is introduced. Native ESS remains independent from shared declarations.

This selection avoids altering the existing persisted LocalRuntimeRecord.bootstrap
schema and closed legacy Bootstrap codec; it does not waive private execution
support. No metadata is guessed from Read/Write. An operation missing explicit
curation is unbound. A protected policy read failure is not an empty catalog.
The application policy returns one coherent revision plus metadata snapshot, and
the owner filters both metadata and descriptor entries by current permissions.
The authenticated same-build ProjectedDescribe carrier gains a distinct metadata
map; ordinary GovernedDescribe and legacy codecs stay closed and unchanged.

Named checks: curation-exact-pins, curation-closed-document, curation-profile-and-auth,
curation-private-file, curation-coherent-revision, curation-hidden-omission and
curation-audit-race. Check a failing stub before implementation. Plant a changed-pin
or hidden-metadata bypass and prove it fails before restored tests and full gate.
Explicit private execution/budget binding remains the next required step before
advertisement; the curation parser alone is not a native conformance claim.

The prior HTTP budget checkpoint8e02d6f2573f4c5c75049989d0239374d2c73175 is published
on draftPR84. Its local full gate passed1350tests with66ignored; CI was still
running when this phase began. The three existing workers remain quota-errored,
so root remains implementation/store writer and claims no independent review.

Implemented: generated curation types, strict core document parsing, protected
owner loading and pin/profile/effect/auth checks, coherent application snapshots,
and permission filtering of both the descriptor and the metadata map. The owner
fingerprint includes metadata values independently of the policy's declared
revision, so changing metadata behind an incorrectly constant revision after the
real audit acknowledgement still refuses. Legacy cached Bootstrap fields and
ordinary GovernedDescribe remain unchanged. The public guide now distinguishes
this internal implemented policy from the still-unavailable MCP serving loop.

The core positive case failed against a refusing stub (1passed,1failed), and the
application suite failed against the loader stub (1passed,7failed). Bypassing the
pin guard failed with `mismatched pin released metadata`; bypassing only metadata
permission filtering leaked item.read in the targeted hidden-operation test and
failed its exact empty-map assertion. Both sources were restored. The restored
core/application/owner suites passed51tests,0failed,1existingignored fixture.
Pin testing calls metadata selection directly, so a stale-revision refusal cannot
conceal a missing pin comparison. The loader also checks cached identity against
the selected adapter before using curation.

Retained logs/SHA256 under .local/mcp-runtime/:
- curation-red.log: a08f62bf0c7ae4e0d17055aa5257fcf589dc8e98415234aedc15bf05478c13e7
- curation-pin-mutation.log: 769395e47a1a82c8006b649dc07394e7b2beebaef81b4d2a0f8bd1f9552caa34
- curation-filter-mutation.log: f288e5c9c411b55eaad76b66845c462601c3e139e2825789b3cb016f93383c15
- curation-owner-restored.log: a596215e89cf75c55294086c2649b5f6a7c6028a3a58c7acef496ddc4ab30b45
- curation-application-restored.log: d69e29e175e8062d4888bdfb720ec264d450f908d09a21c0710cfc472dd1ca31
- curation-core-restored.log: fe1a0ece4e2943e4f572e8f04435b7fbe69124dd029f2f24fbce46512397bd54
- curation-docs-restored.log: 70a6cfdd181136c6bfb74c64de8aa4123aabd28e3388e2f2da459b0b2bbda3c3

Entity Runtime definitions were regenerated through their owning command; parsed
definitions and commands equal HEAD exactly, with only provenance digests changed.
The website reference check initially had no local cache; generating it through
connectors-build docs and then checking produced43contract/97total pages,no drift.
The complete connectors-build gate --msrv passed: 1356 passed, 0 failed,
66 ignored across 166 summaries, including Clippy, drift, adapter isolation
and Rust 1.88/1.91 checks. Log: .local/mcp-runtime/gate-curation.log; SHA256
b377ceaaca632372f7a4fc1aab09d8790c217fb82c8e239a1a1a62555bd274c2.
All repository CI for the preceding published 8e02d6f checkpoint is green.
This checkpoint remains a draft foundation; production serving, native budget
binding, full MCP, provider acceptance and the new release remain incomplete.

## Explicit private bounded reads — 2026-10-03

Select connectors-private/3 explicitly for native bounded reads, as owned by
contracts/cli/v1alpha1/private-bounded-read.md. The immutable PrivateReadBudget
value in ess/domains/transport.yaml validates with ESS 0.45 (24 files). It carries
the original same-boot monotonic deadline, declared execution/provider ceilings
and remaining private payload byte allowances. No persisted entity changes.
Legacy private/1 and private/2 remain closed; version 3 must opt in before
credentials and refuses legacy invokes and mutations. No silent fallback.

Implement the actual parent/child binding and catalog native composition so OAuth
exchange and business HTTP share the same provider cutoff. Named checks:
bounded-read-explicit-selection, bounded-read-original-deadline,
bounded-read-byte-limits, bounded-read-native-context and
bounded-read-shared-oauth-cutoff. Establish a failing stub and a planted cutoff
reset before the restored suite and repository gate. The public envelope limits,
owner admission binding and production MCP loop remain required integration;
this phase does not satisfy full MCP acceptance. The existing quota-errored
workers are unavailable; coordinator implementation and review only.

The prior curation checkpoint 2eb6408e69d910fd28dda851d9168ae2240b4bd1 is published
on draft PR84, with exact remote/PR head verified. Its full gate passed 1356 tests,
0 failed,66 ignored. The managed tree remains active for this integration.

Implemented the explicit V3 handshake, read-only closed request carrier,
checked monotonic budget, parent/child payload checks and catalog native HTTP
composition. Legacy invoke refuses on V3 before transferring credentials; V1/V2
refuse the bounded read port. Catalog OAuth and business HTTP use the same cutoff.
The new transport reaps an in-flight timed-out child, preserving ProviderTimeout
classification. The existing configuration codec accepts only the now-selected
three versions and still refuses future/unknown versions; each selection has a
different executable digest. No production owner selection or MCP serving claim.

Budget tests failed against the refusing stub (0 passed,3 failed). Removing only
the business HTTP cutoff after OAuth made the actual pinned child return success
past the shared deadline, failing the exact timeout assertion. The mutant was
restored byte-for-byte. Restored runtime tests passed28/28 and the actual catalog
process suite passed5/5, with no ignored tests in either run. They include expiry,
consumed original execution time, input/result bounds, strict selection, OAuth
and API execution, and exact child cleanup. The targeted run first exposed the
ProviderTimeout cleanup case, which was corrected without remapping its origin.

Retained logs/SHA256 under .local/mcp-runtime/:
- bounded-read-red.log: 334841eb2bd5b990ebb51ef0c61218cc94def891333958ff9a0a1cf0696aedc9
- bounded-read-cutoff-mutation.log: 69dcf61203f6feec1b395aa6a57049b3c81694f25d9c5d43fb4d842183e86a22
- bounded-read-host-restored.log: 166c2d6c04045ae3f81bcf7dd0e1e83a1f2709111901a1d9f2dad52d4c52bc3c
- bounded-read-native-final.log: 543ae1b2ac79b29fb6c2565cc6eacb9932bf25d7219aad7a5c9fcf8bdab1aa42

Generation used the owning metadata-entities and service-metadata commands.
Parsed Entity Runtime definitions/commands remain identical; only provenance
changed. Docs generation and check passed43contract/97total reference pages
without drift. The complete connectors-build gate --msrv passed: 1364 passed, 0 failed,
66 ignored across 166 summaries; Clippy, projection drift, adapter isolation
and Rust 1.88/1.91 checks passed. Log: .local/mcp-runtime/gate-bounded-read.log;
SHA256 3abfc81be4308fcace5fc8d1a4f2ab609c1f5b55f02c4b49ac6ffbe8b4730111.
The next integration must select these limits through current owner admission,
preserve the original monotonic cutoff through queue/startup/custody/audit,
and enforce whole public envelopes. Full MCP, provider acceptance and release
remain incomplete. Cleanup dry-run returned no eligible assessment for active
cb26l-runtime; preserve its lease and the retained evidence for continued work.

## Owner-selected bounded reads — 2026-10-03

The explicit authenticated same-build bounded_read port now carries the original
PrivateReadBudget and required projection revision through owner admission and the
serialized worker. Exact current curation selects implemented (20s/15s/64KiB/4MiB)
or supported generic (40s/30s/256KiB/4MiB) limits with 5s connect, no required
approval and none/natural idempotency. Caller overrides, absent realization and
private/1 or private/2 bindings refuse. Complete canonical service request and
response envelopes are measured; oversized results become audited capacity errors.

Queue/lifecycle mutex and launch waits, child startup, custody, native work and
reply writes retain the original cutoff. The Secret Service read owns one socket
guard covering setup, qualification, read and Close; expiry shuts down that socket
and joins the guard thread. Credential validity can only shorten native execution.
Both worker admission and the final dispatch fence recheck the selected declaration.

Initial bounded owner selection and queue suite:12 passed,0 failed. Planted selection
bypass:2 passed,2 failed (caller limits admitted and oversized envelope dispatched).
Restored before further work. Custody stub red:1 passed,2 failed; restored custody
suite:4 passed. Removing only the socket shutdown made the exact cutoff regression
fail with WouldBlock after its one-second safety timeout; the source was restored
byte-for-byte. These are coordinator-run checks, not independent review.

The real disposable qualified GNOME custody/native-child final-fence test passes
for both legacy and bounded paths:withdrawal yields no provider invocation and
registry use counts (1 captured,0 dispatched,1 released); allowing the next request
yields audited success and counts (2,1,2). Its first invocation failed before any
owner behavior because the long checkout's fixture socket exceeded the Unix path
limit; a short physical owner-private fixture directory resolved that invocation
failure, and was removed after all owned processes stopped. No desktop bus was used.

Socket checks cover exact original budget serialization, lossless result bytes,
build/expiry refusal before request bytes, and the original reply cutoff. The first
timed fixture consumed its budget measuring the test executable; it now performs
the same pre-budget build measurement as the actual connection handshake.

Owning contract:contracts/cli/v1alpha1/private-bounded-read.md#owner-selection;
owner.md and docs/local-runtime-foundation.md link the implemented scope. Documentation
reference generation/check reports43 contract and97 total pages,no drift.

The complete connectors-build gate --msrv passed:1376 passed,0 failed,66 ignored
across166 libtest summaries; Clippy with warnings denied, ESS/projection drift,
adapter boundaries and Rust1.88/1.91 checks passed. The initial gate stopped on
test-module ordering; formatting that authored file resolved it. The restored
gate includes all12 new owner/socket/custody tests passing. The explicit qualified
final-fence test above is additional to the gate's ignored inventory. This
checkpoint does not claim production MCP serving, extended mutation budgets,
outbound MCP, Helm, complete provider acceptance or a new release. All remain open.
Future MCP ingress must bound its own framing/owner connection before admission.
All three existing agents remain quota-errored; root is the sole implementation and
store writer. No independent-review result is asserted.

Next integration boundary: start one selected deadline before MCP frame/admission
and owner connection; expose only declarations whose native binding and complete
service budgets the owner can actually honor; compose the admitted discovery/read
ports with native request/result codecs and session cleanup. Do not advertise an
operation merely because its curation is well-formed. The existing full production
entry scenarios remain the acceptance target, including mutation replay and exact
process evidence. Cloud caller assignment is still a separate open decision.
Evidence is retained in .local/mcp-runtime (SHA256):
- gate-owner-budget-restored.log:2d2005873a308886cb0350b96d63d11a2cfe375de75a414893b20bfe4a922bee
- owner-budget-selection-mutation.log:6847aa2f21dac8e40472dd0ae1c1d9f7ec5ef03ab69904b4755bf22a1e6a844e
- custody-cutoff-restored.log:dafb55a09d67e85141ea0086eea1adbe689305dc33b928a27813c4c5d9c71425
- custody-deadline-mutation.log:76c0d0e7cd5a0b6eca9b2ddc4b97b7f62895bdf98f7dc317128f803c68bb894c
- owner-budget-final-fence-short.log:f41efdedbfa4afc98909d8cbe389c99525941caa8b2ab3d3e893127bff373dfa
- owner-budget-docs.log:9c84eaa2f40eca5f3ea1f20f0309ef4692ce198188c29da8bb00f95725d0698c
