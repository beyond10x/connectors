# mcp adapter design

**Status:** specification-only owner directory. It holds a pinned upstream specification
and, since `story:mcp-domain-model`, an authored native ESS model at
[`spec/ess/`](spec/ess/system.yaml), and, since `story:mcp-profile-selection-matrix`,
the protocol selection at
[`contracts/protocol/v1alpha1/selection.md`](contracts/protocol/v1alpha1/selection.md).
There is no adapter service, package, generated projection or runtime declaration, and
no behaviour here is implemented.

## Scope and dependencies

`adapters/mcp/` owns this repository's native Model Context Protocol material: the pinned
upstream specification revisions, the authored native model that reads them, and later
the authored protocol contracts that cite both. `epic:mcp-contracts` owns the contract
programme; this directory is where its authored output lands. Both directions the epic
names — outbound use of a remote MCP server, and inbound MCP exposed by this product —
are authored against the same pinned revisions. The model names the nouns of both;
neither direction's behaviour is specified yet.

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
| [`connectors_mcp.state`](spec/ess/domains/state.yaml) | Four entities — `McpServerBinding`, `McpAdvertisedCapability`, `McpOutboundSession`, `McpInboundSession` — the `McpCapabilitySnapshot` value, and the three credential kinds `epic:mcp-contracts` keeps apart, as three distinct types. |

Four things about it are load-bearing and are stated in the files themselves:

- **It declares no ESS relation.** The whole relation census of `epic:mcp-contracts`
  lands at the foot of `domains/state.yaml`: one stated edge, one refusal repeated from
  `ess/domains/sessions.yaml:89-91`, eight `UNMAPPED:` markers — two of which are also
  filed as decision-blockers. A cardinality nobody could read was not chosen.
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
| No ESS relation; every unreadable edge marked at every site, at either end | `crates/connectors-build/tests/mcp_domain_model_census.rs`. `ess specify validate` cannot: a guessed cardinality on an `UNMAPPED:` row validates at exit 0, and so does a flat carrier that means the same thing. |
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
- Every relation the model could not read. `story:mcp-domain-model` declared the nouns —
  see [the authored native model](#the-authored-native-model) above — and declared **no
  ESS relation at all**. Its `domains/state.yaml` closes the epic's relation census as
  one stated edge, one cited refusal and eight `UNMAPPED:` markers, two of them also
  filed against an open decision-blocker. None may be answered by inference from the pinned
  documents; each needs its own modelled and reviewed record.
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
