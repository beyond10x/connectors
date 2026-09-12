# MCP protocol selection — revisions, transports and capabilities

**Status:** an authoring selection, not an implementation. This document says which
protocol revisions, transport bindings and optional capabilities this repository authors
contracts for, which it refuses, and which nobody can yet decide. No runtime, no fixture
server and no executable is added by it, and nothing here has been spoken to an MCP peer.

Owner: `story:mcp-profile-selection-matrix`, decomposing `epic:mcp-contracts`. The
pinned authority is
[the specification pin](evidence/20260912/specification-sources.md) and its 54 archived
files; the nouns are the authored native model at
[`adapters/mcp/spec/ess/`](../../../spec/ess/system.yaml), whose
`connectors_mcp.protocol` domain says in its own header that it selects nothing and that
this document is what reads it.

## What a row of this matrix is, and what it is not

`epic:mcp-contracts` states the rule this document exists to keep:
**Do not equate a protocol capability listing with implemented support.** The matrix
therefore keeps two things in two columns and never lets the first imply the second:

| Column | What it holds |
|---|---|
| Defined by | Where the pinned revision *defines* the feature, as `<archived file>:<line>` of the uncompressed bytes the pin's digests fix. |
| Disposition | What **this repository selects**: `supported`, `explicitly refused` or `deferred`. |

- `supported` — selected for contract authoring in `epic:mcp-contracts`. It means a later
  story in this epic owns its behaviour and the product is intended to speak it. It is
  not a claim that any code exists.
- `explicitly refused` — this repository will not speak it, and the reason column states
  the refusal a caller observes. `epic:mcp-contracts` acceptance criterion 4 requires
  that unsupported functionality be "explicitly refused, not silently approximated", so a
  refusal that a caller cannot observe is not a refusal.
- `deferred` — the disposition rests on something nobody has decided: an open
  `decision-blocker:`, or an `UNMAPPED:` marker that `story:mcp-domain-model` carries
  because no source states the relation. A deferred row names which one. Guessing a
  disposition to avoid an empty cell is the failure this value exists to prevent.

**Direction** is `outbound`, `inbound` or `both`, because `epic:mcp-contracts` says the
two directions "are separate directions with separate trust boundaries":

- `outbound` — Connectors is the MCP **client**, connecting to a selected remote server.
  For a server capability the question is whether Connectors consumes it; for a client
  capability, whether Connectors advertises it.
- `inbound` — Connectors is the MCP **server**, reached through a local binding or
  `$BIN server`. For a server capability the question is whether Connectors advertises
  it; for a client capability, whether Connectors asks a caller for it.
- `both` — the disposition and its reason are the same in either direction.

A feature is covered when its rows give each direction exactly one disposition: one
`both` row, or one `outbound` row and one `inbound` row. That is what "appears exactly
once" means here, and it is what the check counts.

## How this list was derived

The enumeration comes from the archived bytes, not from what anyone remembers about MCP,
because the property worth having is that a capability absent from the matrix is a defect
in the matrix. `crates/connectors-build/tests/mcp_profile_selection_matrix.rs` implements
exactly these three rules and fails if the matrix and the archives disagree.

1. **Revisions** — every protocol version string the archives state in a
   version-declaring position: the `supported` list of the primary revision's
   `UnsupportedProtocolVersionError` example, plus every `LATEST_PROTOCOL_VERSION`
   constant, every `"protocolVersion"` message field and every `MCP-Protocol-Version`
   header value in any of the 54 archived files. Those four positions **disagree with one
   another**, and the disagreement is deliberate here: it is the trap
   [the pin records](evidence/20260912/specification-sources.md), and a derivation that
   read only the obvious constant would state the wrong version and disposition four
   rows instead of five.
2. **Transports** — the `##` sections of the interoperability revision's transports
   document, which is organised one top-level section per transport family, plus every
   row of the primary revision's deprecation registry whose feature links into a
   `/basic/transports` page. The second rule exists because the fourth family is a
   section nowhere: HTTP+SSE survives as a subsection of Backward Compatibility and as a
   registry row. Four families, which is what
   [the adapter design](../../../design.md#what-is-deliberately-not-decided-here) already
   tabulates.
3. **Capabilities** — the fields of `ServerCapabilities` and `ClientCapabilities` in each
   pinned revision's own normative `schema.ts`, unioned across the two revisions. A
   top-level field is a capability a peer advertises and takes a row. Its nested fields
   are settings of an advertised capability rather than separately advertisable
   capabilities, so they are named inside the row that carries them — all eighteen of
   them, which the same check holds to the same standard as the rows.

Neither capability set is closed. `mcp-2026-07-28-schema.ts:789` and
`mcp-2026-07-28-schema.ts:712` say so in as many words, of the server and the client
side: "any server can define its own, additional capabilities." So this enumeration is
complete over what the pinned revisions **name**, and is not a bound on what a peer may
send. What Connectors does with an advertised key that appears in no row below is stated
under [Limits](#limits); it is not silently accepted.

## Protocol revisions

| Key | Direction | Defined by | Disposition | Reason, and what a caller observes |
|---|---|---|---|---|
| `revision:2026-07-28` | both | `mcp-2026-07-28-schema.ts:30` `mcp-2026-07-28-basic-versioning.mdx:62` | supported | The primary pinned revision, whose archived bytes are the authority every contract in this epic is authored against. Every outbound request declares it and the inbound binding accepts it. |
| `revision:2025-11-25` | outbound | `mcp-2026-07-28-basic-versioning.mdx:62` `mcp-2026-07-28-basic-versioning.mdx:126-152` | supported | `initiative:complete-local-connectors` milestone 5 names explicit 2025-11-25 interoperability, and the primary revision specifies how a client detects a legacy server and falls back to the initialize handshake. |
| `revision:2025-11-25` | inbound | `mcp-2025-11-25-basic-lifecycle.mdx:167` `mcp-2025-11-25-basic-lifecycle.mdx:172-174` | supported | The inbound binding answers a legacy initialize with a version it supports, as that revision requires. Which version it names when it cannot honour the requested one is the lifecycle story's, not settled here. |
| `revision:2024-11-05` | outbound | `mcp-2026-07-28-deprecated.mdx:31` `mcp-2025-11-25-basic-transports.mdx:284-311` | explicitly refused | No bytes of this revision are archived here, and without them every statement about it is a hypothesis. Connectors never offers it in an initialize and never follows a fallback that would land on it; it surfaces an unsupported-revision error naming the endpoint it declined. |
| `revision:2024-11-05` | inbound | `mcp-2025-11-25-basic-lifecycle.mdx:61` `mcp-2026-07-28-schema.ts:450` | explicitly refused | A modern request declaring it is answered with UNSUPPORTED_PROTOCOL_VERSION -32022 whose supported list names only the two pinned strings; a legacy initialize declaring it is answered with a pinned version instead, which a client that cannot use disconnects on. |
| `revision:2025-06-18` | both | `mcp-2025-11-25-basic-transports.mdx:271` `mcp-2026-07-28-basic-transports-streamable-http.mdx:278` | explicitly refused | It reaches this matrix only as a header example inside a pinned document and as a named earlier revision; no bytes of it are archived. It is never sent, and a request declaring it is refused exactly as any other unpinned revision is. |
| `revision:DRAFT-2025-v3` | both | `mcp-2025-11-25-schema.ts:14` `mcp-2026-07-28-basic-versioning.mdx:62` | explicitly refused | It is the interoperability revision's own LATEST_PROTOCOL_VERSION left at a draft value, not a negotiable wire version; the negotiable string for that revision is 2025-11-25, stated by the primary revision. A contract reading this constant states the wrong version. It is never sent and never accepted. |

Three of these seven rows exist because a reader who went to the obvious machine-readable
place would get a different answer from the one the revision's own banner gives. That is
the version-string trap
[the pin makes checkable](evidence/20260912/specification-sources.md), and dispositioning
the three trap strings explicitly is the only way a later reader can tell that they were
seen and rejected rather than missed.

## Transports

| Key | Direction | Defined by | Disposition | Reason, and what a caller observes |
|---|---|---|---|---|
| `transport:stdio` | outbound | `mcp-2026-07-28-basic-transports-index.mdx:18-19` `mcp-2026-07-28-basic-transports-stdio.mdx:87-107` | deferred | The binding is a client-launched subprocess, so selecting it decides whether Connectors spawns and supervises an unpinned third-party binary. `decision-blocker:mcp-outbound-stdio-process-ownership` is open, and the model carries the same edge unread as `UNMAPPED: McpServerBinding -> supervised OS process`. |
| `transport:stdio` | inbound | `mcp-2026-07-28-basic-transports-stdio.mdx:33-63` `mcp-2025-11-25-basic-transports.mdx:22-52` | supported | Here the client launches `$BIN server` and Connectors is the child, so no process-ownership question arises on this side. This is the explicitly selected local-client binding `epic:mcp-contracts` requires. |
| `transport:streamable-http` | outbound | `mcp-2026-07-28-basic-transports-streamable-http.mdx:70-105` `mcp-2026-07-28-basic-transports-streamable-http.mdx:107-165` | supported | Named as an implementation target by `initiative:complete-local-connectors` milestone 5 and required to be evaluated by the epic. Framing, streaming, cancellation and session loss are the outbound lifecycle story's, not settled here. |
| `transport:streamable-http` | inbound | `mcp-2026-07-28-basic-transports-streamable-http.mdx:54-68` `mcp-2025-11-25-basic-transports.mdx:54-75` | deferred | This is the cloud-capable server binding, and that placement has more than one principal. Which connection's provider credential carries out a caller's invocation is `decision-blocker:mcp-caller-connection-assignment`, open, and the epic's caller-isolation criterion cannot be authored without it. |
| `transport:custom-transports` | outbound | `mcp-2026-07-28-basic-transports-index.mdx:60-72` `mcp-2025-11-25-basic-transports.mdx:313-322` | explicitly refused | The specification permits any bidirectional channel under MUST-level constraints, so the option space is unbounded and no archived byte names a specific one to author against. A server binding naming a transport other than the two selected ones is refused at configuration time and never attempted. |
| `transport:custom-transports` | inbound | `mcp-2026-07-28-basic-transports-index.mdx:74-79` `mcp-2025-11-25-basic-transports.mdx:313-322` | explicitly refused | Connectors defines no transport of its own and offers none. In particular the existing protected Unix-domain-socket owner transport of the local CLI is not offered as an MCP custom transport, even though the specification recommends reusing stdio framing over exactly such a stream. |
| `transport:http-sse-transport` | outbound | `mcp-2026-07-28-basic-transports-streamable-http.mdx:710-737` `mcp-2025-11-25-basic-transports.mdx:284-311` | explicitly refused | The interoperability revision's HTTP fallback terminates in this transport and says the client should then use it for all subsequent communication. Connectors stops at that point instead: it surfaces an unsupported-transport error naming the SSE endpoint it declined, rather than following the fallback. |
| `transport:http-sse-transport` | inbound | `mcp-2026-07-28-deprecated.mdx:31` `mcp-2026-07-28-basic-transports-streamable-http.mdx:695-709` | explicitly refused | Deprecated and not removed, with new implementations told not to adopt it. The inbound binding exposes no 2024-11-05 SSE endpoint, so a client probing for one receives the binding's ordinary not-found answer rather than a degraded stream. |

Two things about this table are load-bearing and were settled before it was written. It
has **four** rows and not two: `initiative:complete-local-connectors` names stdio and
Streamable HTTP as implementation *targets*, and a target list is not the option space.
And the fourth family is refused rather than absent precisely because `2025-11-25` is
pinned for the legacy seam — its own documented fallback ends in HTTP+SSE, so an outbound
client that simply follows the pinned text lands on a binding no target list mentions.
The refusal above is the thing that stops that happening silently.

Neither open blocker reaches the two rows beside the deferred ones, and the reason is in
each blocker's own text rather than inferred here.
`decision-blocker:mcp-caller-connection-assignment` says that in the single-owner local
placement the gap "is harmless, because `contracts/cli/v1alpha1/semantics.md` section 1
admits exactly one principal" — so inbound stdio is dispositioned on its merits.
`decision-blocker:mcp-outbound-stdio-process-ownership` is about executing a server
binary, which the inbound direction never does. Those two sentences are deliberately here
and not in a reason cell: the check reads a blocker named in a reason cell as a row that
must defer, and a row explaining why a blocker does *not* reach it would be caught by
that rule.

## Server capabilities

What a server advertises. Outbound, the question is whether Connectors consumes it;
inbound, whether the Connectors binding advertises it.

| Key | Direction | Defined by | Disposition | Reason, and what a caller observes |
|---|---|---|---|---|
| `capability:server/tools` | outbound | `mcp-2026-07-28-schema.ts:865-870` `mcp-2026-07-28-server-tools.mdx:45-74` | supported | Invoking selected tools on a remote server is the first outbound deliverable of `epic:mcp-contracts`. Which tools a binding may invoke, and how provider errors stay distinct from protocol errors, belong to the invocation stories. |
| `capability:server/tools` | inbound | `mcp-2026-07-28-server-tools.mdx:76-278` `mcp-2025-11-25-schema.ts:422-430` | supported | The inbound binding advertises tools; this is how a Connectors capability is reached at all. Which operations appear is not decided here: the model carries `UNMAPPED: which Connectors operations are advertised, and under which admission` and the projection story owns it. |
| `capability:server/resources` | outbound | `mcp-2026-07-28-schema.ts:846-855` `mcp-2026-07-28-server-resources.mdx:39-81` | supported | Named by `initiative:complete-local-connectors` milestone 5 alongside tools and prompts. Whether a discovered set is cached is not decided here; the model carries `UNMAPPED: McpServerBinding -> McpCapabilitySnapshot` for exactly that. |
| `capability:server/resources` | inbound | `mcp-2026-07-28-server-resources.mdx:83-200` `mcp-2025-11-25-schema.ts:409-421` | supported | The inbound binding advertises resources. As with tools, which resources it carries is the projection story's and rests on the same unread admission marker. |
| `capability:server/prompts` | outbound | `mcp-2026-07-28-schema.ts:825-830` `mcp-2026-07-28-server-prompts.mdx:39-64` | supported | Named by the same milestone sentence. A remote prompt is content, never authority: a prompt or a tool annotation from a server cannot establish local write authority, which the epic states as a deliverable constraint. |
| `capability:server/prompts` | inbound | `mcp-2026-07-28-server-prompts.mdx:66-160` `mcp-2025-11-25-schema.ts:400-408` | supported | The inbound binding advertises prompts, on the same footing and with the same unread admission question as tools and resources. |
| `capability:server/logging` | both | `mcp-2026-07-28-schema.ts:808` `mcp-2026-07-28-deprecated.mdx:28` | explicitly refused | Deprecated in the primary revision with a stated migration path away from it, and Connectors already owns its own observability. Inbound the capability is absent from the advertised set and a setLevel request is refused as an unknown method; outbound Connectors never subscribes to a server's log stream. |
| `capability:server/completions` | both | `mcp-2026-07-28-schema.ts:815` `mcp-2026-07-28-server-utilities-completion.mdx:33-43` | explicitly refused | Argument autocompletion is named by no deliverable of `epic:mcp-contracts` and no Connectors contract produces or consumes it. Inbound it is absent from the advertised set and a complete request is refused as an unknown method; outbound Connectors never asks a server for completions. |
| `capability:server/experimental` | both | `mcp-2026-07-28-schema.ts:797` `mcp-2026-07-28-schema.ts:789` | explicitly refused | The field is a map of non-standard keys that no archived byte defines, so there is nothing to author against. Inbound the map is never advertised; outbound an advertised experimental key is not projected into any Connectors capability and a call that would depend on one is refused rather than attempted. |
| `capability:server/extensions` | both | `mcp-2026-07-28-schema.ts:882` `mcp-2026-07-28-basic-versioning.mdx:80-119` | deferred | The negotiation mechanism is specified, but no extension identifier is selected, and the only one either pinned revision names for this model is the tasks extension, whose shape is unread — see the tasks row. Refusing the mechanism while its only named instance is undecided would be a decision made backwards. |
| `capability:server/tasks` | both | `mcp-2025-11-25-schema.ts:431-454` `mcp-2026-07-28-basic-versioning.mdx:105-116` | deferred | The two pinned revisions disagree about what tasks is: a named capability field in the interoperability schema, an extension identifier in the primary revision. `adapters/mcp/spec/ess/domains/protocol.yaml` carries this as `UNMAPPED: whether tasks is modelled as a client capability of this root or as an extension identifier`, and no reading of one revision settles the other. |

Sub-capability settings of the rows above, named here rather than given rows of their
own, because each is a setting of a capability that is already dispositioned:
`tools.listChanged`, `resources.subscribe`, `resources.listChanged` and
`prompts.listChanged` are change-notification and subscription switches inside a
`supported` capability, and whether Connectors sets or honours them is a lifecycle
question the subscription and streaming stories own. `tasks.list`, `tasks.cancel`,
`tasks.requests`, `tasks.requests.tools` and `tasks.requests.tools.call` are settings
inside the `deferred` tasks capability and inherit its deferral; nothing about them can
be selected while the capability's shape is unread.

## Client capabilities

What a client advertises. Outbound, the question is whether Connectors advertises it;
inbound, whether the Connectors binding asks a caller for it. The primary revision
removed server-initiated JSON-RPC requests entirely and replaced them with the
multi-round-trip pattern, so an inbound disposition here is about whether Connectors ever
asks a caller for more information mid-request, not about a reverse request channel.

| Key | Direction | Defined by | Disposition | Reason, and what a caller observes |
|---|---|---|---|---|
| `capability:client/roots` | both | `mcp-2026-07-28-schema.ts:732` `mcp-2026-07-28-deprecated.mdx:26` | explicitly refused | Deprecated in the primary revision, with the migration path being to pass directories through tool parameters or server configuration instead. Outbound Connectors declares no roots capability, so a server that requires one fails the request with MISSING_REQUIRED_CLIENT_CAPABILITY -32021, which Connectors surfaces as a protocol error rather than approximating a filesystem root. Inbound it is never asked for. |
| `capability:client/sampling` | both | `mcp-2026-07-28-schema.ts:749-759` `mcp-2026-07-28-deprecated.mdx:27` | explicitly refused | Deprecated in the primary revision, and Connectors owns no model provider to sample from. Outbound the capability is not declared and a server requiring it fails with -32021; inbound the binding never asks a caller to run inference on its behalf. |
| `capability:client/elicitation` | outbound | `mcp-2026-07-28-schema.ts:769-772` `mcp-2026-07-28-client-elicitation.mdx:49-81` | explicitly refused | Connectors declares no elicitation capability, so a server that needs mid-request input from a user fails with -32021 and the failure reaches the caller as a protocol error. Nothing is auto-answered and no default is supplied on the user's behalf. |
| `capability:client/elicitation` | inbound | `mcp-2026-07-28-basic-patterns-mrtr.mdx:10-13` `mcp-2026-07-28-client-elicitation.mdx:83-103` | explicitly refused | The inbound binding never answers a caller with a request for more information. An invocation that lacks what it needs is refused with its own error, so a caller never has to implement the multi-round-trip pattern to reach a Connectors capability. |
| `capability:client/experimental` | both | `mcp-2026-07-28-schema.ts:720` `mcp-2026-07-28-schema.ts:712` | explicitly refused | The client-side twin of the server field: a map of non-standard keys no archived byte defines. Outbound the map is never declared; inbound a caller's experimental key changes nothing about what is advertised or admitted, and no behaviour is unlocked by sending one. |
| `capability:client/extensions` | both | `mcp-2026-07-28-schema.ts:785` `mcp-2026-07-28-basic-versioning.mdx:89-103` | deferred | Same position as the server side: the mechanism is specified, no extension identifier is selected, and the only named instance is the undecided tasks extension. When one party supports an extension and the other does not, the specification allows reverting to core behaviour or rejecting; which of those Connectors does is part of what is deferred. |
| `capability:client/tasks` | both | `mcp-2025-11-25-schema.ts:344-376` `mcp-2025-11-25-basic-lifecycle.mdx:198` | deferred | The interoperability revision lists a client tasks capability that the primary revision's schema does not carry as a named field, showing it as an extension identifier instead. The same `UNMAPPED:` marker in `connectors_mcp.protocol` covers both sides, and it is a modelling question rather than a selection anyone has declined to make. |

Sub-capability settings of the rows above: `roots.listChanged`, `sampling.context` and
`sampling.tools` are settings inside `explicitly refused` capabilities and are refused
with them — a server cannot reach them by asking for a setting of a capability that was
never declared. `elicitation.form` and `elicitation.url` are the two elicitation modes,
both refused; the URL mode is the one that would have navigated a user to a
server-supplied domain, and refusing the capability refuses that outright.
`tasks.requests`, `tasks.requests.sampling`, `tasks.requests.sampling.createMessage`,
`tasks.requests.elicitation` and `tasks.requests.elicitation.create` are settings inside
the deferred tasks capability and inherit its deferral.

## What this document does not cover

- **How a selected transport behaves.** Framing, streaming, cancellation, progress,
  session loss and version mismatch for the selected profiles are
  `story:mcp-outbound-connection-lifecycle` and `story:mcp-inbound-local-binding`, one per
  direction. A transport row here says the binding is selected, not how it behaves.
- **Which Connectors capabilities an inbound caller may invoke.** That is the projection
  story, and it rests on `UNMAPPED: which Connectors operations are advertised, and under
  which admission` in the model's `domains/state.yaml`.
- **Any ownership relation.** This document declares no cardinality and answers no
  `UNMAPPED:` marker. Where a disposition would have needed one, the row is `deferred`
  and names it. `story:mcp-domain-model` closed the relation census as one stated edge,
  one cited refusal and eight markers; none of them is resolved here.
- **Authorization.** Which credential a selected transport acquires, and how, is the auth
  stories'. The pinned revisions make credential acquisition a property of the transport
  rather than of the peer, so those stories read the transport rows above, but no
  disposition here selects an authorization mode.

## Limits

**No row of this matrix is evidence that any MCP server, including a future Connectors
one, implements anything.** A `supported` row is an authoring selection inside
`epic:mcp-contracts` and nothing more. No MCP connection has been made from this
repository, no executable has been added, and a schema that compiles would not establish
network interoperability, a provider effect, a persistent credential or cloud
authentication in any case.

Neither capability set is closed upstream, so this enumeration is complete over what the
pinned revisions name and is not a bound on what a peer may send. An advertised key that
matches no row above is treated as an unknown capability: it is recorded as advertised
and is never projected into a Connectors capability, an admission decision or a tool
call. It is not an error, and it is not support.

The archived bytes, not the network, are the authority for every citation above. Line
numbers are lines of the uncompressed archived file, which is the only kind of line
number that cannot drift, because a digest fixes it.
