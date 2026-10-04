# mcp adapter design

**Status:** native implementation in progress. It holds a pinned upstream specification
and, since `story:mcp-domain-model`, an authored native ESS model at
[`spec/ess/`](spec/ess/system.yaml), and, since `story:mcp-profile-selection-matrix`,
the protocol selection at
[`contracts/protocol/v1alpha1/selection.md`](contracts/protocol/v1alpha1/selection.md).
The initial [Rust library](runtime/Cargo.toml) implements canonical qualified names/resource
URIs, bounded incremental stdio line framing, lossless JSON decoding, generated launch-input validation and
serialized reduction of the shared session state and absolute lease decisions. Native
tests exercise those components under Rust 1.88; EOF-retention, first-terminal and
expiry-boundary mutations fail the intended checks.
The production application now composes these pieces into the
[local stdio read-tools slice](../../docs/local-mcp-stdio.md). Its real-process
tests cover both selected revisions through the protected owner and a pinned
provider. Complete inbound projection and authenticated outbound MCP remain open.

The native [result codecs](runtime/src/results.rs) project an already admitted owner
reply into tools, resources or a declared prompt-message result, retaining service
correlation and complete audit/source/mutation observations. Tool text and structured
content carry the same lossless JSON value. Primary-only completion/cache fields are
separate from legacy replies, and whole-frame encoding budgets the JSON-RPC wrapper,
correlation, duplicated representations and final newline. These helpers do not
establish family eligibility, provider declaration support or request admission.

The protected owner also checks the application-provided projection constraint at
dequeue and final dispatch. A disposable real-custody/owned-child test confirms that
withdrawal at final dispatch cancels the captured credential use without calling the
provider; removing that last check makes the test reach the provider and fail. This
worker evidence remains separate from unfinished MCP process conformance.

Protected `projected_describe` pairs filtered private metadata with the application
revision computed over the original validated cache. Hashing the filtered result in
the caller would disagree with execution when an operation is hidden. The owner
checks both host policy and the projection revision again after audit acknowledgement;
changed or unavailable projection policy releases no metadata. An unbound application
refuses the projected query. Ordinary `governed_describe` and legacy adapter Bootstrap
codecs retain their existing shapes. Neither private carrier is a public descriptor:
complete operation curation, safe field selection and receiver limits remain required
before any operation can be advertised through MCP.

The runtime and generated projection packages are siblings: generated manifests retain
their own workspace declarations, and the root workspace excludes those packages while
consuming their unchanged libraries. The repository gate checks both native launch
projections with its exact ESS pin (`connectors-build mcp-bindings --check`) and includes
the runtime in workspace tests and the Rust 1.88 selection. Application composition must
still supply config/state selectors, protected owner admission and protocol I/O; the
native parser does not call the generated finite-output renderer.

The [session reducer](runtime/src/supervision.rs) implements the eight behavior traits
and state query generated from the unchanged [shared session domain](../../ess/domains/sessions.yaml).
It uses generated transitions, retains the first terminal fact, removes lease authority
on closure/loss and distinguishes local resource release from peer acknowledgement.
The generation check selects that domain from the shared header and compares its whole
projection. The enclosing supervisor must still derive admission decisions, verify clocks
and enforce cutoff/accounting at the actual I/O boundary. The reducer's injected local
clock and decision interfaces are trusted ports, never peer-controlled MCP fields.

The [lease gate](runtime/src/lease.rs) consumes generated DataLease values and a trusted
qualified UTC interval. It accounts for delay and uncertainty against the original
expiry, rejects lifetimes over two seconds, checks existing authority before renewal,
and retains the first denial. Activity never extends authority; a planned drain caps
future renewals. Timestamp comparisons preserve fractional precision. These decision
tests do not qualify an operating-system clock or prove the final transport cutoff:
the enclosing supervisor still needs serialized admission, fresh checks at each I/O
boundary and verified resource cleanup.

The [JSON decoder](runtime/src/json.rs) preserves large integers and decimal values,
rejects duplicate decoded keys and enforces an explicit container-depth limit.
Literal objects retain arbitrary member names, including names that resemble Serde
implementation tokens. Equivalent numeric spellings may normalize; this is not a
canonical byte encoder. The frame reader owns the byte ceiling. The existing core
JSON reader retains its legacy numeric behavior.

The decoder is owned by `connectors-core::json` and re-exported by the native
library, so the generic host never depends on MCP. The protected owner has a
distinct governed read request that acknowledges a real audit anchor before
queueing provider work, rechecks policy in the serialized worker, and returns
the complete service response without passing its numbers through the legacy
reader. Final-observation recovery reuses the same observation and never retries
business work; an unconfirmed final append preserves the known result and the
acknowledged reference with `incomplete` audit. These host, socket and child-process
tests establish the read seam. MCP discovery, projection configuration, real
session clock/I/O supervision and the production launch remain to be connected.

The [local configuration domain](spec/ess/domains/local_server.yaml) now owns four
immutable deployment values. The pinned build projects them into
[`generated/configuration-types`](generated/configuration-types/types.rs) and checks
drift beside the launch/session projections. The [native validator](runtime/src/configuration.rs)
rejects duplicate/unknown JSON members, invalid selectors, conflicting exposures,
unsupported format and out-of-bound integer limits before any file or host access.
It validates an approval source path without opening it. Application composition
admits the private companion file on each projected query/read and fences its selection
at dispatch. The [CLI intent guide](../../docs/local-mcp-cli.md#selected-local-projection-configuration)
records its location and bounds; this parser does not make the server available.

## Scope and dependencies

`adapters/mcp/` owns this repository's native Model Context Protocol material: the pinned
upstream specification revisions, the authored native model that reads them, and later
the authored protocol contracts that cite both. `epic:mcp-contracts` owns the contract
programme; this directory is where its authored output lands. Both directions the epic
names — outbound use of a remote MCP server, and inbound MCP exposed by this product —
are authored against the same pinned revisions. The model names the nouns of both;
both directions' selected contracts now live under `contracts/client` and
`contracts/server`. Runtime work must satisfy their complete admission and lifecycle
requirements before it advertises a supported binding.

The owner directory is a sibling of every other adapter and depends on none of them. It
carries no shared-contract selection yet, because no operation, profile or binding has
been authored to select one.

## Pinned specification revisions

| Revision | Role | Upstream tag | Commit |
|---|---|---|---|
| `2026-07-28` | primary | `2026-07-28` | `5f5440bb26a62e2cf3440b92da5a667efa03b267` |
| `2025-11-25` | interoperability | `2025-11-25` | `38c84e9f93ad191d9eb26d92b945d17bd0efcaf3` |

[The pinned-source record](contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md)
holds the exact URLs, uncompressed SHA-256 values, byte lengths and archived bytes, the
command that reproduces them, and the sections later contract statements will cite. The
authority is the upstream specification repository `modelcontextprotocol/modelcontextprotocol`,
not the sibling `../mcp` repository, which is a consumer of these revisions and is not
pinned here. The two revisions are named by `initiative:complete-local-connectors` line
33; they are not selected by this document.

## The authored native model

`story:mcp-domain-model` authored [`spec/ess/`](spec/ess/system.yaml) as the ESS root
`connectors_mcp v1`. It validates and compiles independently under the pinned toolchain
and is the seventh adapter model the boundary gate reads. **A story in this epic that
needs an MCP noun reads this root rather than declaring one.**

| Domain | Holds |
|---|---|
| [`connectors_mcp.protocol`](spec/ess/domains/protocol.yaml) | Protocol values read from the pinned archives: the two negotiable revision strings, the three eras, the four transport bindings, server and client capability kinds, the legacy session phases, three error codes, the two cache scopes, the acquisition modes and the authorization roles. |
| [`connectors_mcp.state`](spec/ess/domains/state.yaml) | Five entities — `McpServerBinding`, `McpAdvertisedCapability`, `McpOutboundSession`, `McpInboundSession`, `McpStdioProcess` — the `McpCapabilitySnapshot` value, and the three credential kinds `epic:mcp-contracts` keeps apart, as three distinct types. The selected outbound stdio session owns at most one process, which references its server binding and records its executable pin. |
| [`connectors_mcp.launch`](spec/ess/domains/launch.yaml) | Immutable input, transport and final process outcomes for the selected local stdio launch. They introduce no caller, Connection or persistent session relation. |
| [`connectors_mcp.local_server`](spec/ess/domains/local_server.yaml) | Immutable exposure, family, limits and configuration values for the local projection. Existing connection references are explicit deployment selections, not new caller ownership. |

Four things about it are load-bearing and are stated in the files themselves:

- **Relations require a recorded decision.** The original foundation declared no ESS
  relation. The operator's 2026-10-03 resolution of
  `decision-blocker:mcp-outbound-stdio-process-ownership` now supplies the session-owned
  process and its binding reference. The census at the foot of `domains/state.yaml`
  has one stated edge, one cited refusal, seven `UNMAPPED:` markers and one `RESOLVED:`
  marker. General HTTP binding lifetime and cloud caller assignment remain unresolved.
- **Every lifecycle has one state.** ESS 0.22.2 refuses a transition no command outcome
  causes (`ESS-ENTITY-005`), and this story declares no behaviour, so each entity carries
  the single-state shape `connectors.auth_bindings.AuthProfile` uses.
- **It selects nothing.** The four transport bindings and the capability kinds are what
  the pinned revisions declare, not what this repository supports. That selection now
  exists, in [`contracts/protocol/v1alpha1/selection.md`](contracts/protocol/v1alpha1/selection.md),
  and it reads the markers row by row rather than being made here.
- **No carrier closes a set the pinned schema leaves open.** Every enum in
  `domains/protocol.yaml` was checked against its own source and every carrier typed to
  one was enumerated; the sweep and its per-enum verdicts are recorded in that file's
  header. A capability key or a version string the specification permits a server to
  invent is carried as a `String`, not squeezed into a variant list.

**What holds each of those, exactly — because they are not held by the same thing, and
one of them is not held at all:**

| Property | Held by |
|---|---|
| Resolved stdio ownership matches its recorded decision; every unreadable edge remains marked at every site, at either end | `crates/connectors-build/tests/mcp_domain_model_census.rs`. `ess specify validate` cannot: a guessed cardinality on an `UNMAPPED:` row validates at exit 0, and so does a flat carrier that means the same thing. The census also checks the selected process ownership, binding reference and executable pin fields. |
| Single-state lifecycles | **ESS itself**, not that case — `ESS-ENTITY-005 missing_causation` and `ESS-ENTITY-011 dead_end_state`/`unreachable_state` refuse the alternatives. |
| No carrier closes an open set | `mcp_domain_model_adversary_pass2.rs`, which reads the openness out of `domains/protocol.yaml` rather than hard-coding it. |
| **It selects nothing** | **Partly.** A new `SelectedTransport` enum would still validate, compile and pass every case in this repository, and the census case only asserts that `domains/protocol.yaml` still carries its no-selection disclaimer. What changed with `story:mcp-profile-selection-matrix` is that the selection now exists elsewhere and is itself checked: `crates/connectors-build/tests/mcp_profile_selection_matrix.rs` derives every revision, transport and capability from the archived bytes and fails if `contracts/protocol/v1alpha1/selection.md` does not disposition each exactly once per direction. Nothing still stops a second, contradicting selection being added to this domain. |

## What is deliberately not decided here

- The pinned revisions specify **four** transport bindings, not two. Which of them this
  repository selects is no longer open — see
  [`contracts/protocol/v1alpha1/selection.md`](contracts/protocol/v1alpha1/selection.md),
  which carries a row per binding per direction — but the option space this document
  tabulates is what that selection had to cover:

  | Binding | Status in the pinned revisions | Cited in the archives |
  |---|---|---|
  | stdio | current | `2026-07-28 basic/transports/stdio.mdx`; `2025-11-25 basic/transports.mdx` §stdio (22) |
  | Streamable HTTP | current | `2026-07-28 basic/transports/streamable-http.mdx`; `2025-11-25 basic/transports.mdx` §Streamable HTTP (54) |
  | custom transports | permitted, MUST-level constraints | `2026-07-28 basic/transports/index.mdx` §Custom Transports (60): "Implementers who choose to support custom transports **MUST** preserve the … metadata model"; `2025-11-25 basic/transports.mdx` §Custom Transports (313) |
  | HTTP+SSE, from `2024-11-05` | **deprecated, not removed** | `2026-07-28 basic/transports/streamable-http.mdx` §HTTP+SSE Transport (2024-11-05) (695), "New implementations **SHOULD NOT** adopt it; existing implementations **SHOULD** migrate" (703-705) and the fallback procedure (710-737); `2026-07-28 deprecated.mdx` (31) lists it under `## Deprecated` (22) and above `## Removed` (37); `2025-11-25 basic/transports.mdx` §Backwards Compatibility (284-311), whose fallback ends "should use that transport for all subsequent communication" (309-311) |

  The initiative names stdio and Streamable HTTP as implementation targets; that is a
  target list, not the option space. HTTP+SSE matters precisely because `2025-11-25` is
  pinned for the legacy seam: that revision's documented fallback terminates in HTTP+SSE,
  so an outbound client that follows it lands on a binding the target list does not
  mention. Selecting, bounding and specifying transports — including an explicit refusal
  of custom transports or of HTTP+SSE, if that is the selection — together with framing,
  streaming, cancellation, progress, session loss and version mismatch, is a separate
  authoring obligation that reads this pin. A selection matrix built from this bullet has
  a row for each of the four.
- Every relation still marked `UNMAPPED:`. The original `story:mcp-domain-model`
  foundation declared no ESS relations. The 2026-10-03 operator decision resolves
  only the outbound stdio process subset, as described above. Seven census edges
  remain unmapped, including general outbound HTTP binding lifetime and cloud caller
  assignment; none may be answered by inference from the pinned documents.
- Any mapping onto shared service, records, session, auth, discovery or mutation
  contracts.

A capability listed in a pinned specification document is a protocol declaration. It is
not evidence that any MCP server implements it and it is not a support claim by this
repository. That distinction is the reason the revisions are pinned before anything is
authored.

## Citing this document: use a heading, not a line number

**For the nine stories still to come.** A `file:line` citation into this document has a
half-life measured in units, and this epic has now spent three findings on it. The last
one was created by `story:mcp-domain-model`: inserting
[the authored native model](#the-authored-native-model) section moved the four-transport
table from lines 36-42 to 82-87 and left a sibling unit's case citing the old range,
which by then pointed into the new section. Nothing caught it — that case matches text
rather than lines, so it stayed green while its own comment had become false.

So: **cite a heading, a quoted phrase or a stable identifier, and cite a line number only
into an immutable archived file** — the `vendor/*.gz` bytes under
`contracts/protocol/v1alpha1/evidence/`, whose line numbers are pinned by a digest and
cannot move. When you do edit this document, grep the repository for citations to it
before you finish; `adapters/mcp/design.md:` finds them.

## The boundary obligation this directory creates

Creating `adapters/mcp/` made `mcp` a forbidden term in **shared** ESS, silently and as a
side effect of the directory name. `crates/connectors-build/src/ess_boundary.rs` lines
300-302 insert every adapter owner's words into the boundary gate's forbidden term set
unless that name is listed in the reviewed `shared_protocol_names` policy, and
`crates/connectors-build/ess-boundary.json` line 3 lists exactly one name there: `sip`.

Nothing is red today — shared `ess/` and `contracts/` name MCP nowhere, and the gate
exits 0 with this directory present. The obligation is for later: a story in this epic
that needs *shared* vocabulary to say MCP will be refused by the boundary gate, and the
refusal will read as that story's own bug rather than as this one's consequence. `sip` is
the precedent for what the exception has to look like — an entry in the reviewed
`shared_protocol_names` policy, justified by the term being genuine shared protocol
vocabulary rather than native provider semantics. Per
[the adapter index](../README.md#boundary-gate), such an entry cannot suppress a
forbidden native term or an adapter-path dependency, and no per-file suppression exists.
Native MCP vocabulary stays in this directory and needs no exception at all.

## Sources and remaining obligations

The original foundation note below is retained as history. It predates the
contracts, generated packages and native runtime library now described at the top
of this document; it is not the current implementation inventory.

The only retained source is the specification pin above; it is research provenance, not
adopted upstream source/license packaging, a build dependency or a vendored library.
`spec/ess/` exists and is described under
[the authored native model](#the-authored-native-model). `upstream/`, `generated/` and
`src/` do not exist here and are not implied.

The transport and capability profile selection is no longer an obligation:
[`contracts/protocol/v1alpha1/selection.md`](contracts/protocol/v1alpha1/selection.md)
holds it. Remaining obligations, all unstarted: the rest of the protocol contract
documents under `contracts/`, and every runtime, fixture and conformance artifact. No
MCP connection has been made from this repository and no executable has been added.
