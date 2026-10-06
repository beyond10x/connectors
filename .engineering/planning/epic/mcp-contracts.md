---
format: aep.planning-md/3
id: epic:mcp-contracts
kind: epic
status: draft
title: Contractualize outbound and inbound MCP, including auth and local/cloud server bindings
relations:
- informed_by: specification:recent-agent-adapter-usage-20260909
- informed_by: specification:contract-driven-connectors-design
- informed_by: review-result:concept-stack-integration-20260908
revision: 9
---
## Operator request

Track full MCP contractualization in this repository. The local CLI must connect to MCP servers, including authentication. The CLI must also expose MCP to local clients and, when deployed in a cloud, through `$BIN server`. This is an explicit product requirement recorded on 2026-09-09, not an inference that any current adapter already implements MCP.

The local CLI with persistent local credentials remains the baseline. Cloud serving is an optional placement; it must not impose a hosted identity service or federation on local use. Outbound use of a remote MCP server and serving inbound MCP are separate directions with separate trust boundaries.

## Required contract deliverables

- Outbound MCP: discovery and explicit selection of a server; authenticated connection and persistent local credential use/repair; initialization/version and capability negotiation; invoking selected tools and accessing other selected MCP capabilities; truthful results, errors, limits and shutdown.
- Inbound MCP: an explicitly selected local-client binding and a cloud-capable server binding exposed by `$BIN server`; caller authentication/authorization where required by the placement; discoverable Connectors capabilities and invocations with their existing target, permission, mutation and credential-locality guarantees.
- Pin the authoritative MCP specification revision and select supported transport/capability profiles at authoring. Evaluate local stdio and deployed HTTP profiles explicitly; specify framing, streaming, cancellation, progress, connection/session loss and version mismatch for the selected profiles. Do not equate a protocol capability listing with implemented support.
- Specify tools, resources, prompts and optional reverse-direction capabilities separately: supported, explicitly refused or deferred, with reasons. Tool annotations or remote descriptions cannot establish local write authority. Preserve provider errors versus protocol errors, result content/structured output and output bounds.
- Specify auth for both directions, including protected credential entry/acquisition, persistence, expiry/refresh/repair, revocation and failure. Credential custody is replaceable and local by default. Caller credentials for inbound access, credentials for an outbound MCP server and underlying provider credentials are distinct.
- Define mappings to existing Connectors service, records, session/execution, auth, discovery and mutation contracts where their guarantees apply. Reuse established owners instead of making MCP the universal internal contract. Keep MCP wire/native details with the owning adapter/binding so it can be extracted independently.
- Describe composition when a local or deployed MCP server exposes an outbound MCP-backed capability. Preserve caller/target provenance, limits and admission through the mapping; prohibit implicit credential forwarding, account fallback or authority escalation from discovered metadata.
- Provide a human CLI journey and machine-readable discovery/error contract. `$BIN server` is the requested entry-point intent; finalize exact flags and transports during contract authoring rather than treating this tracking record as a implemented CLI promise.

## ESS and review workflow

Read the design handoff and reuse existing typed ownership/lifecycles. Author the selected textual profiles and applicable ESS values, entities, commands, events, relations and lifecycle scenarios before implementation decomposition. UNMAPPED: durable MCP-specific state, ownership, transport-profile selection and any unsupported ESS semantics must be resolved by that work, not guessed in this epic. No new domain/entity or executable adapter is introduced by this tracking action.

Review wire/protocol fidelity and security/credential boundaries, then prove mapping and runtime conformance separately. A schema compile or lifecycle simulation does not prove network interoperability, provider effects, persistent credentials or cloud authentication.

## Acceptance

1. Outbound local CLI connects to a selected MCP fixture, authenticates, invokes an advertised capability, exits/restarts and reuses its persisted credential binding. Missing, expired, revoked or unavailable credentials produce distinct safe outcomes.
2. A local MCP client connects to the selected `$BIN server` local binding, discovers only supported/admitted capabilities and receives correctly mapped results and errors. No cloud control plane is necessary.
3. The cloud server profile specifies authenticated inbound access, transport/session lifetime, streaming and failure behavior. A fixture verifies no provider-secret disclosure; this epic does not authorize deployment. Caller isolation is not in this epic (decided 2026-10-07): the inbound MCP caller is the configured owner, a single principal, and the isolation fixture for a multi-caller placement is `story:mcp-cloud-caller-isolation`, a draft that waits on a cloud placement being selected.
4. Selected protocol versions, transports and optional capabilities have a coverage matrix. Unsupported functionality is explicitly refused, not silently approximated.
5. Cancellation, malformed input, version/capability mismatch, partial output, lost replies and mutation uncertainty have reviewed scenarios. A repeated MCP request cannot automatically duplicate an uncertain business effect.
6. All selected contractual gaps have owners, applicable ESS declarations/scenarios and independent review evidence before runtime stories are scheduled. CLI/docs describe the final selected behavior for humans and agents.

## Scope and status

Backlog recording only. No MCP connection, server startup, cloud deployment, credential migration or provider operation is authorized by this record. Implementation remains future work after the contracts are selected. The broader recent-usage analysis continues independently.

## Current delivery preparation — 2026-10-02

The operator's later approval-record:milestones-delivery-20261002 authorizes
selected implementation and verified source release. The original Scope and status
paragraph is the historical recording boundary, not a new request for permission.
MCP follows the initial provider acceptance work; no MCP runtime is accepted yet.

The landed contract/model stories establish directional profiles, not runtime
ownership for every census noun. All eight unresolved relations remain unresolved
by accepted contracts. Five concern configuration/Connection/custody, snapshot
retention and legacy session lifetime and require explicit reviewed design; they
are not five additional operator approval gates. The inbound-session/caller model
belongs with the existing caller-assignment decision. The separate outbound
process-ownership blocker remains open. Single-owner inbound stdio can avoid a
new durable caller model, and outbound HTTP executes no provider process.

The next independent contract slices are outbound invocation/results and inbound
single-owner capability projection. They require no new durable relation merely
to specify mappings. Outbound auth/runtime binding then needs a cohesive recorded
mapping to the existing configuration, Connection, acquisition, credential generation
and custody owners. Native/shared ESS roots remain independent; use qualified
projection carriers instead of importing cross-root entity targets. Snapshot and
legacy-session persistence must be selected explicitly before runtime uses them.
The projection draft now cites its actual scalar operation_ref edge rather than
inferring it from the different AdapterSpecification ownership relation.

The sibling MCP library has reusable tools/HTTP/stdio/OAuth mechanics but lacks
resources/prompts/server APIs and some required bounded collection/atomic auth
seams. Retrofit its shipped behavior and establish its governed plan before
changing runtime. Preserve tools-only consumers, add consumer-owned transport and
auth coordination, publish the independently verified sibling revision, then pin
it here. No Harness repin or hosted deployment is included automatically.

Preparation is retained privately in `.local/next-milestones/mcp-preflight.md` and
`mcp-relations-preflight.md`. These source audits establish work scope, not passing
wire interoperability or persistent-auth conformance.

## Source inventory for the first sibling retrofit — 2026-10-02

Read-only inventory verified clean sibling MCP primary at
51b9c7969def3dc0fbee9026cd26f4ce5837d62c and already-fetched origin/main at
0fdfbafa7c130caa76d74825680d72af3280e21b. The latter changes documentation authority
only; runtime inputs are identical. No fetch, build, test or source/spec/store
mutation was performed. No ESS root, governed plan or ESS opt-out exists there.
MCP's current controlled transport testkit is Unreleased relative to local tag
0.1.2; neither hosted release state nor complete compatibility is inferred.

The smallest independent retrofit is the shipped tools value/operation boundary
and frozen connected-handle snapshot, using the public HTTP client as the first
real conformance target. It is not a durable server/session/request model. Sources:
crates/b10x-mcp-client/src/lib.rs:21-96,101-150,196-228 and
crates/b10x-mcp-types/src/lib.rs:80-105,162-319 in the sibling. Current configured
protocol versions are not an executed legacy-fallback matrix. Existing HTTP/stdio
transport tests are useful seeds; the sibling gate is cargo xtask gate and does
not yet validate or execute ESS. A managed sibling checkout, governed adoption,
minimal validated specification and an actual conformance target are required
before claiming retrofit completion. No entities or relation semantics are
introduced into this Connectors record by the inventory.

Preserve these source disagreements explicitly in the future draft: sibling
AGENTS says types carries no credential value, while types::SecretString exists;
Limits.max_pages is declared but the client does not read it; max_frame_bytes is
wired only to HTTP SSE. SDK behavior and executable cases must establish effective
bounds. Session lifecycle, registry removal/custody cascade, snapshot persistence,
OAuth one-use/refresh coordination, omitted-capability behavior and consumer
ownership relations remain UNMAPPED rather than invented from field composition.
Opaque wire values are not automatically persistent entities, and tools/call is
not automatically a read merely because this library keeps no local ledger.

This first HTTP/tools specification and conformance work can proceed after the
provider handoff without the caller-assignment, outbound-process or execution-family
decisions. Those blockers stay open for their dependent consumer work. New inbound
server/resources/prompts capabilities remain new specification and implementation,
not behavior that may be asserted by the retrofit. Detailed source citations are
retained in the task-owned mcp-retrofit-inventory/report.md, SHA256
cb744d5de42ca204e8c72095787c494383150b244c83b127486e9f359eaac242.

## Parallel draft preparation — 2026-10-02

While the provider acceptance fixture is completed, prepare a scratch-only minimal
ESS system/domain and named scenarios for the source-inventoried sibling tools/HTTP
boundary. This uses the existing Connectors ESS0.45.0 executable solely to check
draft syntax; it selects no sibling toolchain or release dependency. No MCP checkout,
runtime/store/source mutation, actual conformance target or delivery is authorized
by this preparation step. Sibling adoption and integration still follow provider
handoff under the existing delivery authorization.

Scope the shipped public value constructors and connected-handle snapshot/call
behavior cited in the mcp-retrofit-inventory report. Keep all UNMAPPED facts and
actual caller/process decisions open. No invented durable server/session/request
entities, credential lifecycle, new capability, schema guarantee on arbitrary
public serde values or universal wire-losslessness. Name actual failures and
source citations; label structural checks separately from executed conformance.
Root's assigned scratch mcp-retrofit-draft owns all draft files. Return the precise
remaining conformance-adapter seam, with no new runtime implementation/build.

## Scratch structural draft result — 2026-10-02

The scoped draft now contains six source-cited public value types and eighteen
named constructor/HTTP behavioral obligations. The supplied ESS0.45.0 command
`ess specify validate --path .local/provider-wave-briefs/mcp-retrofit-draft/spec`
exited0 with `mcp v1 — 2 file(s), valid`. This is structural validation only.
The manifest has no executable scenarios; the eighteen obligations are prose and
no conformance synthesis or execution occurred. No sibling source/store/runtime
was changed. Draft report SHA256
a31627567bf3987d1919be1308ad80d53b9666759f5e18528c61796e7c5f4676.

The draft does not invent persistent Connection/Session/Request entities to fit
opaque live handles. A reviewed direct-return and target-local handle routing
binding is still needed before authoring executable scenarios and their Rust
target over real constructors/connect_http_with_client. Target routing tokens
are not new persistent product identities. Constructor guarantees are kept distinct
from public serde shapes, and decoded SDK JSON from original wire bytes. Limits/
error representation is an explicit additional binding gap; all original eight
source/model disagreements remain. This is preparation, not MCP delivery.

## Direct-return mapping resolved — 2026-10-02

Source inspection of exact ESS0.45.0 tag eff6aa70085ffd341c503834582acfe1088c8d86
establishes that source format ess/17, typed command response, returns:true and
scenario4 response assertions model direct library functions without invented
entities/events/views. The earlier scratch ess/15 header was unnecessarily narrow;
no upstream ESS direct-function blocker exists. A successor draft should bind real
constructors plus a scenario-scoped actual HTTP Connection in begin/end_scenario.
The supplied draft is not yet revised, synthesized or executed.

No general direct-response-to-handle capture was found in scenario4. Use the actual
single receiver or an explicitly target-local fixture slot; do not invent product
handle IDs or persistent ownership. Provider-call counters are not product return
fields. Keep their zero-dispatch/positive-control checks explicit Rust integration
obligations until a separate observation declaration is reviewed. Preserve exact
JSON/numeric observation limits, absent/null projection distinctions, real error
payloads and all existing UNMAPPED lifecycle/bounds/consumer decisions.

Detailed source-cited mapping report in task scratch mcp-target-mapping/report.md,
SHA256d7bfe91a04cb1176a62627cd45026c87cd930b372036bf2267ac22ea8dc413ec.
No sibling source/store/toolchain adoption or runtime/conformance execution occurred.
This prepares the already-authorized MCP delivery after the provider release handoff.

## Measured direct-return limits — 2026-10-02

The successor ess/17 scratch draft validates three files and preserves all six
nominal value declarations. Full selection of eight authored constructor scenarios
fails synthesis with three constrained-response refusals. Partial selection emits
nine scenarios (four generated and five authored), with four retained refusals;
none were executed. This qualifies the preceding general mapping finding:
direct-return support exists, but unchanged constrained ConnectionId/ToolSnapshot
returns require invariant observation the pinned tool does not support. The count
invariant projection is also refused. Neither constraints nor assertions were
removed to make this appear complete.

ESS0.50.0 reproduced ESS0.45.0's results on byte-identical inputs, including identical
partial suite bytes. Read-only inspection of local ESS0.51.0 tag
0347ffa222939e3791e574d2dbe42d4b4b02d979 found both refusal boundaries still present;
0.51 was not executed or installed for this probe. An ambient tool upgrade is not
a measured remedy. Existing implemented direct-library-return-observations and
active input guard work do not claim constrained output invariant observation.
No exact open owner was found in the searched local planning snapshots; this is
not a claim about every remote issue or private plan.

Retained task-owned reports and SHA256:
- mcp-direct-return-draft/report.md:
  f5cda4fbb3193ba460dd23140a74ee3a33f5463aaff17248fc67d468a5c9615e.
- mcp-newer-ess-probe/report.md:
  84b3980f5b7870e640c201d75a228645ac9e6602a35fd8c080bd092bc7904f16.
- mcp-ess-blocker-owner/report.md:
  4b96e6f2c5a83a2438e5ec864683cc507845617c2981b3f6bb92ff421a1b33a9.

Full unchanged declaration conformance remains incomplete. Independent MCP source
adoption, actual runtime target and contract work can proceed after the provider
release. A separately scoped faithful observation projection would retain its
uncovered native invariants explicitly; it is not selected by this scratch work.
No new upstream implementation, sibling dependency pin, invented persistent entity,
source release or claim of MCP delivery follows from these probes.

## Delivery route after specification refinement — 2026-10-03

Operator clarified that ESS refinement means the MCP specifications, not skill instructions. The next two waves are explicitly authorized by approval-record:mcp-next-two-waves-20261003. Invocation/projection/framing prerequisites merged in Connectors PR80 at fcef8b18a58c16e8af07c0a04aece5e8415164d2 after required checks; exact candidate/merge tree matched. Auth/replay final reviewed sources are integrated and their fullgate is running. Composition plus the separate CLI selected-intent task follow from that verified published base while CI runs; merging still waits for required checks. The parent working CLI journey stays open.

Sibling beyond10x/mcp PR5 refined eleven immutable HTTP observation values after the earlier constructor/gate foundation. Candidate110fc10edcfcc90bb9b56d4f1fcf2c7e5c22cda7 merged atc45e1a74125ac1c4cbba79fa90fe1cb098cb18c8, identical treec831e8e4b24ae24e8f5c812396707781028b0642. PR and merged-main checks passed; Rust1.88 taskcheck26passed. Constructor suite still has nine same scenario bodies, six explicit partial refusals, three authored refusals and inconclusive coverage. New HTTP execution coverage is zero. Model H1–H4 preserve byte/length/fidelity/instrumentation obligations; generated codecs expose nine duties rather than claiming structural decoding enforces them. Design-review planted missing peer data was detected, not a runtime mutation score. Frozen modelworker was archived and exact-GC removed.

The shortest delivery sequence remains:
1. Close the two authorized contract waves with retained negative controls, one integration gate each and source publication; no runtime completion inferred.
2. Use sibling specification:strict-http-exchange-scope and its validated immutable domain for the first actual bounded HTTP tool-exchange slice: raw observations, selected negotiation, no automatic redispatch, actual owned wire fixture and independent counters. Preserve the existing API/digest path through explicit opt-in. Model any further input/output types before runtime stories. Resources and prompts follow that boundary rather than duplicating transports.
3. Resolve the existing native binding/Connection/custody/snapshot ownership markers by reviewed ESS design and actual source owners before Connectors runtime decomposition. These design questions do not create extra operator approval gates. Persistent authenticated outbound acceptance needs real restart/custody evidence, not another document checker.
4. Implement single-owner local inbound stdio against existing admitted read operations, then the explicit mutation binding against existing approval/attempt/replay owners. Model the selected CLI launch inputs/results/errors and prove pinned ESS0.45 expressibility before parser changes; stdout must be reserved for protocol frames.
5. Cut a verified usable MCP source release only after the chosen runtime acceptance passes. Keep multi-caller cloud assignment, outbound stdio child ownership and external execution decisions open until answered; none is chosen by the HTTP model or CLI selected-intent task.

Parallelism follows independent source ownership, not direction labels alone. Fixture/runtime API work touching the same sibling files is serialized; composition and static CLI docs own disjoint paths. Run authors and fresh adversaries in isolated managed trees, keep a single planning writer, and overlap published-candidate CI with the next independent authoring wave. Root integration caches remain available; completed worker targets are disposable only after their evidence is retained and source is recoverable.
