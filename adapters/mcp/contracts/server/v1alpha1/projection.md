# MCP inbound capability projection — server/v1alpha1

Status: authored full projection and document-only cases; the production
[read-only tools subset](../../../../../docs/local-mcp-stdio.md) has process tests.
Resources, prompts and mutations remain unfinished. Owner: `story:mcp-inbound-capability-projection`.
[Local binding](semantics.md) owns the single configured owner, stdio admission,
framing, cancellation, supervision and selected revisions. The
[selection matrix](../../protocol/v1alpha1/selection.md) selects inbound tools,
resources and prompts for primary `2026-07-28` and interoperability `2025-11-25`.
This contract maps those selections without activating any currently unbound
operation. The [cases](projection-cases.json) are document metadata, not ESS
commands or wire messages.

Use the selected revision's exact outer codec. Primary `server/discover` reports
protocol support; legacy `initialize` reports its capabilities. Actual operation
discovery uses `tools/list`, `resources/list` and `prompts/list` in either revision,
ordered by canonical advertised name. Each primary result includes
`resultType:"complete"`; this means the MCP request finished, never that the
business dataset is complete. Its service payload retains partialness and cursors.
Primary cacheable results (discovery, lists and resource reads) use `ttlMs:0` and
`cacheScope:"private"`; no authorization context may borrow another's result.
Legacy results omit these primary-only fields. Required per-request primary
protocol metadata and legacy initialization remain the local binding's duties.

## Admission and discovery

Each advertised capability projects exactly one declared operation, identified by
(instance, adapter, operation), using the existing `McpAdvertisedCapability`
carrier in [native state](../../../spec/ess/domains/state.yaml). The declaration
remains owned by `AdapterSpecification`; no second operation owner is created.

An operation is discoverable in a family exactly when it is implemented, fully
bound to that family's complete codec/profile, enabled, selected in the local
owner's configuration, and admitted by current metadata policy. A package name,
reachable connection, raw provider capability, prompt, annotation or stale cached
list establishes none of these facts. No selected compatible declarations means
an empty family list, not synthetic example operations. Unsupported/reserved or
unbound operations are omitted. Explicit family selection remains configuration
owned by the existing local binding; this contract invents no persisted config
entity or adapter declaration field.

Metadata eligibility and execution viability are separate. An admitted operation
remains visible when dependencies, provider permission, credentials or fresh
execution approval are unavailable. Advertise its declared approval requirement
and limits, never a promise that listing grants execution. Describe/list uses safe
host metadata only: **no provider credential probe, refresh, approval spend or
business dispatch**. Required current metadata policy unavailable means a safe
`unavailable` error, not a successful empty or stale list. A metadata-denied
operation is absent; separately admitted private lookup can still identify it.

Re-evaluate admission on every page and request. List cursors bind the selected
family, owner/configuration scope and descriptor projection revision, use bounded
pages, and refuse stale cursors; they cannot grant scope. No provider cursor or
credential locator is repurposed as a list cursor. Declaration removal,
deselection, disablement or metadata admission loss withdraws the capability on
the next admitted list and invalidates affected projection revisions. Withdrawal
is discovery behavior, not persistent record deletion or a cascade. In-flight
behavior and change notifications follow the selected local/protocol bindings;
notification delivery never substitutes for admission checks.

For valid framing/authentication/version and bounded syntactically valid
identifiers, use [governed service §3.2.1](../../../../../contracts/service/v1alpha2/semantics.md#321-visibility-lookup-and-refusal-precedence-e12):
current lookup/operation/result-scope policy and verified executor binding first;
then exact current projection revision; then private implemented/bound lookup and
enablement; then operation input shape and admitted scope; then existing key
observation; only a candidate new attempt reaches approval and dependency/provider
preflight. Dispatch repeats/fences current admission. The existing mutation owner
retains its audit, spend, replay and uncertainty rules.

Thus denied lookup yields `not_granted` before stale/existence disclosure;
unavailable policy yields `unavailable`; denied target observation yields
`forbidden`. Admitted stale revision yields `stale_description` before hidden,
absent, invalid-input or key decisions. With current admitted lookup/revision an
unknown/unbound operation yields `not_found`, while a disabled one yields
`forbidden`. A guessed name grants neither scope nor an alternate connection.
The legacy binding's universal hidden-name `not_found` is not this governed rule.
Refusals before admitted key/result observation omit mutation metadata; absence
cannot assert that an earlier invocation was not attempted.

The following table checks document decisions, not a policy engine. `policy`
in the case file is current invocation lookup policy; `metadata_admitted` is its
separate metadata decision. `candidate_or_retained_key` only passes lookup/input
checks: it is neither a dispatch instruction nor a claim of a successful result.
| Case | Discovery | First lookup/input outcome |
|---|---|---|
| ready | visible | candidate_or_retained_key |
| dependency-unready | visible | candidate_or_retained_key |
| approval-missing | visible | candidate_or_retained_key |
| unknown | omitted | not_found |
| unbound | omitted | not_found |
| disabled | omitted | forbidden |
| metadata-hidden | omitted | candidate_or_retained_key |
| policy-beats-stale | omitted | not_granted |
| policy-unavailable | omitted | unavailable |
| scope-beats-stale | visible | forbidden |
| revision-beats-hidden | omitted | stale_description |
| revision-beats-absent | omitted | stale_description |
| revision-beats-input | visible | stale_description |
| bad-input | visible | invalid_input |

## Qualified names and inverse selection

Names are `c1_` followed by three nonempty lowercase hexadecimal encodings of the
exact UTF-8 bytes of instance, adapter and operation, separated by `_`. No Unicode
normalization, case folding, percent decoding or concatenation without boundaries
is permitted. Decode exactly three even-length fields, reject invalid UTF-8 and
noncanonical spelling, then require byte-for-byte re-encoding equality. The name
is at most 128 ASCII bytes; an identity exceeding that ceiling is omitted rather
than truncated, hashed or aliased. Empty identity components are ineligible.
This deliberately bounded injective encoding follows the pinned tool-name
recommendations (`mcp-2026-07-28-server-tools.mdx:311-329`); it also gives prompts
an unambiguous name. Namespace separation means the same qualified operation may
be selected in multiple families without changing its identity. Its full
(instance, adapter, operation) remains in safe admitted metadata. No caller may
supply a different instance, adapter or connection inside operation input to
change that selection; existing native input schemas retain their own business
targets under current scope checks.

Resources use `connectors-mcp:///<name>?revision=<hex>`: exactly the canonical
name above and lowercase hex of the nonempty UTF-8 descriptor projection
revision. No authority/host, fragment, extra or repeated query member, or URI
normalization fallback is accepted. The resource name field remains `<name>`.
The URI addresses that operation's configured read with `{}` input; it is not a
provider URL or a credential reference. Tools and prompts carry the exact
revision string in request `_meta["io.beyond10x.connectors/revision"]`. Missing
revision is malformed input; a syntactically valid old revision is checked at the
position defined above. List results and each advertised definition carry the
same revision in safe `_meta`; it is never an authorization token. Operation
names, resource URIs and projection revisions cannot switch owner or Connection.

| Case | Advertised name |
|---|---|
| qualified-first | c1_61_6263_64 |
| qualified-separator | c1_6162_63_64 |
| qualified-other-instance | c1_7a_6263_64 |
| qualified-unicode | c1_c3a9_615f62_612e62 |
| qualified-empty-refused | omitted |
| qualified-long-refused | omitted |

## Family compatibility and results

All three families require a complete selected binding. “Complete” means the
operation's exact input/output schema, target and permission constraints, effect,
approval, revision, correlation, audit/provenance, cursor, partial/completeness,
byte/time bounds and declared errors survive this projection. Metadata includes
those applicable safe declared requirements; a human description or untrusted
annotation cannot replace them. A receiver/reader unable to honor any requirement
must omit that projection. This rule includes unsupported streaming, media,
protected credential acquisition, callbacks and live-session control: a unary
JSON wrapper alone cannot provide them. No new CLI command or product type is
introduced by this document.

- **Tools:** eligible declared operations use the complete selected service
  invocation/profile codec with operation identity fixed by the name. Tool input
  schema wraps the operation's input under `input`; the selected complete binding
  must also preserve any declared approval/key/executor members, without admitting
  unsupported combinations. Provider Connection selection stays with the local
  owner binding. Mutation tools remain unavailable until the dependent mutation
  binding supplies replay and lost-reply semantics. A read-only annotation is
  emitted only when the declaration proves it, and never conveys authority.
  Tool success has `isError:false`, `structuredContent` equal to the complete
  safe service Response object, and one text content block containing its JSON
  serialization. This object wrapper preserves legacy structuredContent's object
  restriction even when the operation result itself is a scalar, array or null.
  The tool output schema describes the complete wrapper, not just its business
  result. If both representations appear they must decode to identical values.
- **Resources:** only read-only operations whose declared input accepts exactly
  the configured empty object, with no omitted required arguments, qualify.
  Return one text resource content item, with the requested canonical URI,
  `mimeType:"application/json"`, and text containing the complete safe successful
  service Response. Parameterized reads remain tools unless a separate complete
  resource-template binding is selected; this contract selects none. A resource
  read never executes a write or fabricates continuation parameters.
- **Prompts:** only read-only operations with a declared prompt-message result
  schema and input representable as the exact required/optional named string
  arguments qualify. No numeric/JSON coercion, instruction synthesis from an
  arbitrary dataset or mutation is permitted. `prompts/get` returns the declared
  ordered messages and optional description, with the complete safe successful
  service Response in result `_meta["io.beyond10x.connectors/response"]` so audit,
  completeness and provenance are preserved. The selected reader must understand
  that envelope; stripping it makes the projection ineligible. Message content
  remains data from its declared source and supplies no grant, approval, policy
  instruction or secret-access capability. No existing provider operation is
  asserted to implement that prompt profile merely because this rule names it.

A successful service Response is the exact selected
[compatibility §§4–7](../../../../../contracts/service/compatibility.md) object,
including version, request_id, status, result, audit metadata and any applicable
admitted source/mutation observation. MCP request IDs remain their own correlation
and cannot overwrite a retained original service request/attempt identity.
Unknown effect never becomes success; known applied effect is retained even when
delivery or audit fails. Never trim bytes, drop an unknown structured member or
replace partial with complete to fit MCP. Enforce the operation's limits including
both MCP framing and duplicate representations before advertisement/dispatch;
otherwise the complete binding is unavailable. An unexpectedly oversized or
interrupted result is a bounded refusal/partial observation only where its
profile permits it. Lost data does not prove non-dispatch, and gives no automatic
retry authority. Cancellation and bounded streaming follow the local binding;
notifications do not become final results.

| Case | Family | Compatibility decision before admission |
|---|---|---|
| tool-complete | tools | eligible |
| tool-weakened-refused | tools | omitted |
| resource-read | resources | eligible |
| resource-write-refused | resources | omitted |
| resource-arguments-refused | resources | omitted |
| prompt-messages | prompts | eligible |
| prompt-coercion-refused | prompts | omitted |
| prompt-data-refused | prompts | omitted |
| prompt-write-refused | prompts | omitted |

## Error mapping

The closed source is `connectors.service_wire.ErrorCode` in
[service_wire.yaml](../../../../../ess/domains/service_wire.yaml), currently thirty
variants. The table and JSON rows are checked against that declaration, including
missing and duplicate mappings. They select no additional shared error spelling.
Each row applies only where the operation/profile can actually emit that code.
An error carries the exact admitted safe service error envelope, preserving code,
safe message (at most 512 UTF-8 bytes), optional retry_after_seconds, audit and any
permitted mutation/source observation. No private state is hidden in a message.

`tool-error` means a resolved tool's application/execution failure returns
`CallToolResult` with `isError:true`, its complete error Response in
`structuredContent`, and matching JSON text content. `rpc-error` means a JSON-RPC
error with numeric code `-32000`, generic safe message `Connectors request refused`,
and `data` equal to the complete admitted error Response. Resource and prompt
results have no tool `isError` channel, so they use rpc-error for every service
failure. The integer is a binding-local carrier, not a thirty-first shared code.

For tools, errors in admission, locating the tool or its selected binding, revision
checks and pre-resolution request decoding use rpc-error regardless of the table's
execution column. Operation input validation after admitted lookup is an
application tool-error. The policy/revision order still applies before choosing
a channel: hiding an operation never discloses whether a tool handler exists.
Protocol-only parse/invalid-request/unknown-method/version/capability errors stay
with the pinned protocol/local binding and never masquerade as a service code.
Unavailable audit or unauthenticated coordinates are omitted/null only as the
existing envelope permits; no invented audit acknowledgement or attempt ID.

| Shared code | Resolved tool execution | Resource | Prompt | Observable service fact |
|---|---|---|---|---|
| invalid_input | tool-error | rpc-error | rpc-error | Preserve invalid_input and the admitted envelope; no retry authority. |
| unsupported | tool-error | rpc-error | rpc-error | Preserve unsupported and the admitted envelope; no retry authority. |
| unauthorized | tool-error | rpc-error | rpc-error | Preserve unauthorized and the admitted envelope; no retry authority. |
| forbidden | tool-error | rpc-error | rpc-error | Preserve forbidden and the admitted envelope; no retry authority. |
| not_found | tool-error | rpc-error | rpc-error | Preserve not_found and the admitted envelope; no retry authority. |
| stale_description | tool-error | rpc-error | rpc-error | Preserve stale_description and the admitted envelope; no retry authority. |
| stale_cursor | tool-error | rpc-error | rpc-error | Preserve stale_cursor and the admitted envelope; no retry authority. |
| rate_limited | tool-error | rpc-error | rpc-error | Preserve rate_limited and the admitted envelope; no retry authority. |
| unavailable | tool-error | rpc-error | rpc-error | Preserve unavailable and the admitted envelope; no retry authority. |
| capacity | tool-error | rpc-error | rpc-error | Preserve capacity and the admitted envelope; no retry authority. |
| timeout | tool-error | rpc-error | rpc-error | Preserve timeout and the admitted envelope; no retry authority. |
| upstream_protocol | tool-error | rpc-error | rpc-error | Preserve upstream_protocol and the admitted envelope; no retry authority. |
| internal | tool-error | rpc-error | rpc-error | Preserve internal and the admitted envelope; no retry authority. |
| approval_required | tool-error | rpc-error | rpc-error | Preserve approval_required and the admitted envelope; no retry authority. |
| approval_refused | tool-error | rpc-error | rpc-error | Preserve approval_refused and the admitted envelope; no retry authority. |
| approval_replayed | tool-error | rpc-error | rpc-error | Preserve approval_replayed and the admitted envelope; no retry authority. |
| idempotency_conflict | tool-error | rpc-error | rpc-error | Preserve idempotency_conflict and the admitted envelope; no retry authority. |
| outcome_unknown | tool-error | rpc-error | rpc-error | Preserve outcome_unknown and the admitted envelope; no retry authority. |
| not_granted | tool-error | rpc-error | rpc-error | Preserve not_granted and the admitted envelope; no retry authority. |
| stale_authority | tool-error | rpc-error | rpc-error | Preserve stale_authority and the admitted envelope; no retry authority. |
| connection_not_ready | tool-error | rpc-error | rpc-error | Preserve connection_not_ready and the admitted envelope; no retry authority. |
| route_unavailable | tool-error | rpc-error | rpc-error | Preserve route_unavailable and the admitted envelope; no retry authority. |
| route_refused | tool-error | rpc-error | rpc-error | Preserve route_refused and the admitted envelope; no retry authority. |
| insufficient_scope | tool-error | rpc-error | rpc-error | Preserve insufficient_scope and the admitted envelope; no retry authority. |
| session_not_ready | tool-error | rpc-error | rpc-error | Preserve session_not_ready and the admitted envelope; no retry authority. |
| session_lost | tool-error | rpc-error | rpc-error | Preserve session_lost and the admitted envelope; no retry authority. |
| offer_expired | tool-error | rpc-error | rpc-error | Preserve offer_expired and the admitted envelope; no retry authority. |
| offer_rejected | tool-error | rpc-error | rpc-error | Preserve offer_rejected and the admitted envelope; no retry authority. |
| lease_expired | tool-error | rpc-error | rpc-error | Preserve lease_expired and the admitted envelope; no retry authority. |
| revoked | tool-error | rpc-error | rpc-error | Preserve revoked and the admitted envelope; no retry authority. |

`outcome_unknown` preserves unknown effect; timeout/unavailable alone cannot prove
not_attempted. An applied effect remains applied even on an error. Replay is an
observation flag, never a new effect classification. The dependent mutation story
owns exact key binding, candidate approval and lost-reply behavior; this mapping
neither selects a replay store nor authorizes a second dispatch.

## Credential locality and unresolved ownership

Only already admitted public metadata and operation results may cross MCP. No
provider credential, secret locator, custody/credential generation handle,
protected entry value, token-derived private claim or credential-derived private
fact is copied into names, descriptions, arguments, results, errors, progress,
cursors or logs. Safe explicitly admitted connection metadata remains distinct
from secret evidence. Local OS Secret Service custody and provider-side
permission checks keep their existing owners. MCP descriptions and credentials
are not a substitute for them. No credential is acquired over this stdio binding.

The configured single owner remains the only principal. This document does not
introduce `McpCaller`, a caller-selected Connection, a session-to-Connection edge,
a deletion cascade, or a resolution of `decision-blocker:mcp-caller-connection-assignment`.
All eight native relation markers remain unresolved:

- UNMAPPED: McpServerBinding → ServiceConfiguration.
- UNMAPPED: McpServerBinding → Connection.
- UNMAPPED: McpServerBinding → McpCapabilitySnapshot.
- UNMAPPED: McpOutboundSession → McpServerBinding.
- UNMAPPED: McpServerBinding → credential custody.
- UNMAPPED: McpInboundSession → McpCaller.
- UNMAPPED: McpCaller → Connection.
- UNMAPPED: McpServerBinding → supervised OS process.

## Pinned evidence and remaining proof

Citations below refer to uncompressed archived bytes in the
[pinned source record](../../protocol/v1alpha1/evidence/20260912/specification-sources.md),
not a fetched newer specification:

- `mcp-2026-07-28-schema.ts:1809-1843` and
  `mcp-2025-11-25-schema.ts:1102-1129`: tool content, structuredContent and
  application error versus protocol error distinction.
- `mcp-2026-07-28-schema.ts:209-235,1081-1109` and
  `mcp-2026-07-28-server-discover.mdx:7-9`: primary result discriminant,
  mandatory cache fields and discovery; request completion is distinct from
  provider completeness.
- `mcp-2026-07-28-schema.ts:1229-1231` and
  `mcp-2025-11-25-schema.ts:721-723`: resource contents.
- `mcp-2026-07-28-schema.ts:1634-1708` and
  `mcp-2025-11-25-schema.ts:969-983`: prompt results and arguments.
- `mcp-2026-07-28-server-tools.mdx:311-329`: bounded names and server-local
  uniqueness. This contract's exact codec is Connectors-owned, not an upstream
  naming mandate.

The Rust guard executes a document checker against the actual ESS error set,
qualified-name cases, family cases and visibility/refusal rows. Mutated copies
with missing/duplicate errors, ambiguous names, weakened family compatibility or
contradicted policy/revision rows must fail. This proves only those authored
correspondences. It does not execute MCP, a provider, a policy engine, credential
custody, a result codec, revision/cursor invalidation, or live discovery withdrawal.
Existing ESS `PermitData` session traces cannot establish those projection facts.
Future runtime conformance must exercise all three real protocol families and
both selected revisions, every error/channel boundary, exact round-tripping,
negative authority and credential leakage cases, partial/oversize/lost replies,
withdrawal and dispatch fencing, and the dependent mutation observation binding.
