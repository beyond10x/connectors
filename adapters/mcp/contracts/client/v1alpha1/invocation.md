# Outbound MCP invocation observations

**Status: authored mapping contract. No runtime, wire interoperability or MCP
conformance is established by this document or its JSON cases.** The owner is
`story:mcp-outbound-invocation-results`. The [selection matrix](../../protocol/v1alpha1/selection.md)
and [connection lifecycle](semantics.md) retain selection and transport ownership.
The [native model](../../../spec/ess/system.yaml) is unchanged.

## Selected operations and revisions

For an explicitly selected, admitted live HTTP interaction, this contract covers
`tools/call`, `resources/read` and `prompts/get`. It creates no implicit selection
from discovery, Connection assignment, credential acquisition, approval or new
invocation record. Selected revisions are 2026-07-28 and 2025-11-25.
Modern request metadata and legacy initialization remain lifecycle obligations.
Other capabilities retain their matrix disposition.

Citations name uncompressed lines of the immutable
[archived specification](../../protocol/v1alpha1/evidence/20260912/specification-sources.md),
not current network documents. Each case cites its revision's schema and chapter.
The checker resolves the pin's file/line; semantic interpretation remains review.

| Observation | 2026-07-28 authority | 2025-11-25 authority |
|---|---|---|
| Tool content and business error | `mcp-2026-07-28-schema.ts:1809`, `mcp-2026-07-28-server-tools.mdx:498`, `mcp-2026-07-28-server-tools.mdx:738` | `mcp-2025-11-25-schema.ts:1102`, `mcp-2025-11-25-server-tools.mdx:449` |
| Resource contents and errors | `mcp-2026-07-28-schema.ts:1229`, `mcp-2026-07-28-server-resources.mdx:402` | `mcp-2025-11-25-schema.ts:721`, `mcp-2025-11-25-server-resources.mdx:385` |
| Prompt messages and errors | `mcp-2026-07-28-schema.ts:1634`, `mcp-2026-07-28-server-prompts.mdx:319` | `mcp-2025-11-25-schema.ts:969`, `mcp-2025-11-25-server-prompts.mdx:270` |

These observation rules use existing protocol values and shared error/limit
vocabulary. The JSON is document-test metadata, not an operation payload, public
response envelope or new ESS domain. A runtime binding needs its native typed
home and executable conformance before implementation.

## Results preserve their meaning

A complete valid tool result preserves ordered `content`, each selected content
block's fields and metadata, `structuredContent` presence and value, and `isError`.
Absent structured output differs from explicit null. Modern structured output can
be any JSON value; legacy structured output is an object. Do not stringify an
array into a legacy object or synthesize structured output from text.
For a declared output schema, validate using its supported dialect; a mismatch is
malformed peer output and an unsupported dialect is `unsupported`. The document
checker does not execute output-schema validation.

Resource reads retain every entry in order, URI, MIME type, text or base64 blob,
annotations and metadata. Prompt retrieval retains description, message order,
role and content. Text, images, audio, embedded resources and resource links keep
their distinct representations. No link is automatically fetched, prompt
instruction executed or binary decoded as text. Unknown optional fields remain
opaque data where the selected schema allows them.
Modern resource results also require and preserve `ttlMs` and `cacheScope`
through `CacheableResult` (`mcp-2026-07-28-schema.ts:1081`). Missing or invalid
cache metadata is malformed peer output. The example uses zero/private; no cache
is created and no cache entry is reused across authorization contexts. Legacy
results acquire no modern cache fields merely by passing through this binding.

An unknown content discriminator yields `unknown-content` with `unsupported`.
Keep the bounded opaque observation, never silently drop that block and report
remaining blocks as complete. A future result discriminator yields
`unselected-result` with `unsupported`. Modern `resultType: input_required`
belongs to the unselected reverse-input flow: do not answer, execute instructions
or submit the request again. Modern missing/non-string `resultType` is malformed
peer output. Legacy absence means complete (`mcp-2026-07-28-schema.ts:232`);
that compatibility rule does not accept an unfamiliar modern result as complete.
`resultType: complete` alone proves neither transport completion nor business success.

## Byte bounds and completeness

Use exactly the existing `OperationLimits` axes: `request_bytes`, `result_bytes`,
`execution_ms`, `provider_ms`, `connect_ms`, declared in
[service_wire.yaml](../../../../../ess/domains/service_wire.yaml).
No numeric production default or sixth limit axis is selected here.

Request bytes are the exact UTF-8 JSON-RPC request body sent after encoding,
including envelope, parameters, escaping and metadata. Result bytes are the exact
UTF-8 JSON-RPC response message before parsing or reserialization, including
its envelope, result or error, metadata, whitespace and base64 characters.
HTTP headers, transfer framing and SSE field delimiters are transport framing.
For SSE, count assembled data-message bytes including newlines inserted between
data lines. Future transport code must independently bound framing, progress and
other events within admitted transport/deadline limits; this rule does not permit
unlimited streaming or buffering before a message.

Exactly the admitted byte ceiling is within bounds. Oversize request encoding is
`request-bound` with `invalid_input` before dispatch. On result overflow, retain
at most the admitted prefix, stop accumulation and report `result-bound` with
`capacity` and incomplete observation. A byte prefix may end inside a UTF-8 code
point. Do not decode it lossily, substitute characters or present it as partial
JSON. Encoded binary and multibyte text count in their transmitted representation.
Cases express the exact prefix as hex; that is not a selected public codec.
Case `response_bytes` counts the supplied observed fixture bytes, never a claim
about the unseen remainder after the reader stops. Implementations must not infer
the full remote output length from an overflow prefix.

Completion requires an observed transport end for a single-body response, or a
complete correlated final JSON-RPC message boundary for SSE, plus valid decoding
and selected result semantics. Content-Length matching, a valid JSON prefix,
progress, cancellation, timeout or closure without a final message does not prove
completion. Case `eof` denotes that terminal boundary, not waiting for a long-lived
SSE connection to close. This preserves the complete-versus-prefix rule in
[transport.yaml](../../../../../ess/domains/transport.yaml).
Malformed complete bytes yield `upstream_protocol`; missing terminal observation
is `outcome_unknown`. Overflow plus terminal loss preserves the `capacity` cause
and unknown effect knowledge, never a successful result.

Connect, provider and execution deadlines keep their existing phase meaning.
After possible dispatch, a deadline does not establish absence of effects.
Lifecycle cancellation, timeout and session loss remain independently observable.
This document introduces no timer or automatic continuation.

## Distinct errors and uncertain effects

| Class | Observable mapping | Preserved distinction |
|---|---|---|
| Local malformed input | `caller-input`, `invalid_input`, preflight | No dispatch; caller-owned malformed parameters cannot be blamed on the peer. |
| Valid tool execution failure | `provider-business-error`, no fabricated protocol code | Preserve result and `isError: true`, content and structured output. |
| Peer JSON-RPC error | `peer-protocol-error` | Preserve integer code, message and optional data as peer data, source peer and response stage. |
| Malformed peer response | `malformed-peer`, `upstream_protocol` | Preserve bounded diagnostics; never relabel as caller input. |
| No complete response | `unobserved`, `outcome_unknown` | Prefix remains incomplete; no result or business success is inferred. |

Generic peer JSON-RPC failures map coarsely to existing `upstream_protocol`
while retaining the integer code/message/data. That shared code alone does not
mean malformed framing: the named observation class preserves the distinction.
Modern errors already owned by lifecycle retain their mapping: `-32020` is
`internal` (client/header disagreement), `-32021` and `-32022` are `unsupported`.
Neither the native three-code enum nor this table closes upstream integer errors.
An unfamiliar integer stays a peer protocol error. Resource absence illustrates
why identity matters: legacy recommends `-32002`, modern `-32602`; the latter
also covers other invalid parameters and cannot universally become `not_found`.
These native observations do not prescribe a new shared wire payload or smuggle
opaque peer data into a shared safe error message. That closed binding requires
its own native ESS home before runtime implementation.

A valid result establishes what the peer reported, not whether an arbitrary
business effect occurred. `isError` proves neither rollback nor non-dispatch.
After sending without a definitive valid observation, effects stay unknown even
when the diagnostic is `capacity`, `timeout`, `upstream_protocol` or a session
error. Case `effect` distinguishes only local non-dispatch from unknown business
effects; it does not replace shared `EffectKnowledge` or `AttemptRecord`.
The [lifecycle contract](semantics.md), under `mcp.outbound.framing-refused`, and
its [unreadable-answer scenario](scenarios/unreadable-answer-is-not-the-callers-input.yaml)
preserve this distinction: an unreadable answer after dispatch establishes neither
non-execution nor rollback.

No path automatically redispatches: version suggestions, idempotent/read-only
annotations, adjusted tool arguments, partial output, credential repair or links.
A caller can explicitly choose a new operation under ordinary admission; no safe
retry token or duplicate-suppression promise is invented. Shared terminal
`Indeterminate` stays terminal. No invocation/result/snapshot persistence follows.
Tool annotations, descriptions, prompts and self-reported identity are untrusted
data, never local write authority. Existing local admission and approval owners
remain authoritative; no credential crosses domains here.

## Named document cases

[invocation-cases.json](invocation-cases.json) uses
`mcp-invocation-document-cases/1`. Each row supplies literal bytes, revision,
family, terminal-boundary and dispatch observations, byte ceilings, citations and
literal outcomes. These are authored examples, not a peer or MCP library run.

The Rust guard derives selected revisions/capabilities from the matrix and
error/limit vocabulary from shared ESS. It checks document/case correspondence,
required obligation coverage, byte prefixes, error distinctions and prohibited
authority/redispatch claims using a small decision table. It is not a full
JSON-RPC/schema decoder, output-schema validator, transport parser, deadline
implementation, authentication test or runtime conformance target. Archive
integrity has existing owning checks. A resolvable citation is not semantic proof.

| Case | Revision | Family | Obligation | Outcome |
|---|---|---|---|---|
| `inv.modern.tools` | 2026-07-28 | tools | tools | `result` |
| `inv.modern.resources` | 2026-07-28 | resources | resources | `result` |
| `inv.modern.prompts` | 2026-07-28 | prompts | prompts | `result` |
| `inv.modern.structured` | 2026-07-28 | tools | structured | `result` |
| `inv.modern.structured-null` | 2026-07-28 | tools | structured | `result` |
| `inv.modern.business` | 2026-07-28 | tools | business | `provider-business-error` |
| `inv.modern.protocol` | 2026-07-28 | tools | protocol | `peer-protocol-error` |
| `inv.modern.resource-missing` | 2026-07-28 | resources | protocol | `peer-protocol-error` |
| `inv.modern.prompt-missing` | 2026-07-28 | prompts | protocol | `peer-protocol-error` |
| `inv.modern.caller-input` | 2026-07-28 | tools | caller-input | `caller-input` |
| `inv.modern.request-bound` | 2026-07-28 | tools | request-bound | `request-bound` |
| `inv.modern.malformed-peer` | 2026-07-28 | tools | malformed-peer | `malformed-peer` |
| `inv.modern.partial-valid-prefix` | 2026-07-28 | tools | partial | `unobserved` |
| `inv.modern.lost` | 2026-07-28 | tools | partial | `unobserved` |
| `inv.modern.mid-utf8-bound` | 2026-07-28 | tools | bound | `result-bound` |
| `inv.modern.annotations` | 2026-07-28 | tools | annotations | `result` |
| `inv.modern.unknown-content` | 2026-07-28 | tools | unknown-content | `unknown-content` |
| `inv.modern.input-required` | 2026-07-28 | tools | unselected-result | `unselected-result` |
| `inv.modern.future-result` | 2026-07-28 | tools | unselected-result | `unselected-result` |
| `inv.modern.missing-result-type` | 2026-07-28 | tools | malformed-peer | `malformed-peer` |
| `inv.modern.protocol-32020` | 2026-07-28 | tools | protocol | `peer-protocol-error` |
| `inv.modern.protocol-32021` | 2026-07-28 | tools | protocol | `peer-protocol-error` |
| `inv.modern.protocol-32022` | 2026-07-28 | tools | protocol | `peer-protocol-error` |
| `inv.legacy.tools` | 2025-11-25 | tools | tools | `result` |
| `inv.legacy.resources` | 2025-11-25 | resources | resources | `result` |
| `inv.legacy.prompts` | 2025-11-25 | prompts | prompts | `result` |
| `inv.legacy.structured` | 2025-11-25 | tools | structured | `result` |
| `inv.legacy.business` | 2025-11-25 | tools | business | `provider-business-error` |
| `inv.legacy.protocol` | 2025-11-25 | tools | protocol | `peer-protocol-error` |
| `inv.legacy.resource-missing` | 2025-11-25 | resources | protocol | `peer-protocol-error` |
| `inv.legacy.prompt-missing` | 2025-11-25 | prompts | protocol | `peer-protocol-error` |
| `inv.legacy.caller-input` | 2025-11-25 | tools | caller-input | `caller-input` |
| `inv.legacy.request-bound` | 2025-11-25 | tools | request-bound | `request-bound` |
| `inv.legacy.malformed-peer` | 2025-11-25 | tools | malformed-peer | `malformed-peer` |
| `inv.legacy.partial-valid-prefix` | 2025-11-25 | tools | partial | `unobserved` |
| `inv.legacy.lost` | 2025-11-25 | tools | partial | `unobserved` |
| `inv.legacy.mid-utf8-bound` | 2025-11-25 | tools | bound | `result-bound` |
| `inv.legacy.annotations` | 2025-11-25 | tools | annotations | `result` |
| `inv.legacy.unknown-content` | 2025-11-25 | tools | unknown-content | `unknown-content` |
| `inv.legacy.invalid-structured-array` | 2025-11-25 | tools | malformed-peer | `malformed-peer` |

## Preserved open work

Seven relation markers in the native state model remain `UNMAPPED:`, including
binding/snapshot ownership and lifetime. The operator resolved
`decision-blocker:mcp-outbound-stdio-process-ownership` on 2026-10-03;
`story:mcp-outbound-stdio-runtime` owns its separate delivery. Caller assignment,
auth lifecycle, composition/provenance, mutation replay and runtime/typed result
binding keep their respective owners. Document checks close none of them.
