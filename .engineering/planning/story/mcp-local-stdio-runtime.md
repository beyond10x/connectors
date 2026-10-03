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
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src
- confidence: cited
  path: crates/connectors-host/src/local
- confidence: cited
  path: docs/local-mcp-cli.md
revision: 23
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
source semantics are docs/local-mcp-cli.md and server/v1alpha1. ESS0.45 validates four
files and projects four selected values. The native parser fixture's package identity
must be distinct from the generic CLI contract; production takes only its server
subtree and validates through the generated shape. Shared ESS imports no native type.

Native protocol mechanics belong to adapters/mcp; application composition supplies
host admission and operation ports. Reuse shared sessions state/lease behavior,
existing local owner transport and mutation controls through explicit ports. Do not
invent a caller entity, persistent MCP session association or a provider secret route.
All UNMAPPED state relations and the two existing operator decisions remain open.

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
