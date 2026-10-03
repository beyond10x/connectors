# Local MCP composition and provenance

**Authored contract with document cases. No composed capability is configured,
implemented, advertised or exercised by this change.** Owner:
`story:mcp-composition-provenance`. This selects requirements for a single local
configured principal when an inbound MCP capability is backed by a separately
admitted outbound HTTP MCP interaction. Multi-caller placement and outbound stdio
remain with their open decision blockers.

## Explicit owner and independent hops

The local composition owner selects and wires the concrete inbound projection,
native operation mapping and outbound binding. Neither generic host libraries nor
one adapter discovering the other creates this pairing. A package, endpoint,
capability listing or two configured adapters supplies no wiring or permission.
This is a requirement on the existing composition boundary, not a new owner entity,
configuration format, process placement or persistent composition record.

[Inbound projection](../../server/v1alpha1/projection.md) owns qualified names,
family eligibility, descriptor revisions, admission order and complete result/error
codecs. [Outbound invocation](../../client/v1alpha1/invocation.md) owns remote
observations, byte accounting and protocol errors. Their transport/session owners
remain independent. [Outbound auth](../../client/v1alpha1/auth.md) owns credential
lifecycle; [inbound mutation](../../server/v1alpha1/mutations.md) owns keyed replay,
protected approval and effect knowledge. This contract adds only cross-boundary
requirements and cannot weaken an owner's guarantees.

Inbound and outbound select their own supported protocol revisions independently.
All four pairs of `2026-07-28` and `2025-11-25` must be considered for each of tools,
resources and prompts; matching revisions is not a prerequisite or a negotiated
shortcut. A selected pair does not prove an operation mapping exists. The inbound
projection's revision is its own descriptor/configuration meaning. It is not an
outbound protocol version, credential generation, or persisted capability-snapshot
revision: the native snapshot declares no such persistent revision identity.
An unsupported revision on one hop cannot silently select another on either hop.

The [mediated HTTP contract](../../../../../contracts/discovery/mediated_route/v1alpha1/semantics.md)
is a precedent for explicit ownership, independent admission and no fallback.
Its transport-only, credential-free child, parent connection, GET-only and private
same-process constraints do not describe this MCP operation composition. No
`mediated_route` or `parent_connection_ref` relation is inferred here.

## Fixed selection and authority

The admitted local configuration fixes the operation, provider connection,
remote MCP resource/authority and account. Inbound names, resource URIs, prompt
arguments and ordinary tool input cannot retarget that selection. A peer's tool
annotations, description, resource link, prompt or claimed account cannot raise
local permission, create an approval, change an audience or select a new operation.
A returned URL is data; it is not followed by composition.

Apply current inbound lookup/target/revision/input policy in the order its owner
requires. The selected outbound interaction separately needs current admission,
its selected revision, credential validity and fixed target. Current dispatch
fences apply at dispatch; current result-disclosure admission applies again at
return. An inbound grant cannot stand in for an outbound grant and vice versa.
Failure on either hop is reported through the admitted inbound error carrier,
without leaking private lookup, target or credential details. Safe outer
correlation survives; a denied result is not disclosed from a cache.

Three credential domains remain separate: inbound caller material, the credential
for the fixed remote MCP server, and whatever credentials that server owns for its
providers. Composition never forwards one domain's material into another, including
arguments, prompts, URIs, headers, error messages, metadata, provenance or logs.
Outbound credentials are used only at their admitted outbound auth boundary.
Credential failure preserves the auth owner's distinct missing, expired, revoked
or unavailable observation. There is no anonymous retry, alternate account,
credential substitution, repaired-credential replay or version fallback.

## Complete mappings and safe provenance

A family is eligible only when its complete native mapping satisfies both owning
contracts. Tools preserve the full safe service Response in the inbound object
wrapper and matching JSON text. Thus a modern outbound structured array or explicit
null can enter legacy inbound **only through that object wrapper**, after the native
operation has supplied an eligible complete safe result mapping. No raw array/null
is passed as legacy `structuredContent`, coerced to an object, or derived from text.
Absent and explicit null remain distinct. A hypothetical document fixture is not
that missing native typed binding.

Resources still require the projection's read-only, empty-input complete profile;
no write or guessed argument is introduced. Prompts still require the declared
ordered-message schema and exact string argument mapping, including their complete
safe Response metadata. A random remote prompt or dataset does not establish that
profile. The three families do not become interchangeable merely because their
payloads fit JSON. Protocol result discriminants and cache fields stay with each
selected revision's owner. Unknown optional content, partialness, cursor and errors
retain their declared meaning; no truncation or field-dropping makes an ineligible
mapping eligible. Without the actual executable replay/protected-approval binding,
mutation tools remain omitted even if an upstream tool claims idempotence or safety.

Caller and target provenance consists of admitted local identity plus preserved
safe observations of the selected source. The existing
[SourceAudit model](../../../../../ess/domains/service_wire.yaml) and
[service compatibility §5](../../../../../contracts/service/compatibility.md#5-extended-responses-audit-and-mutation-observation)
provide an instance-qualified opaque audit correlation, not a record, lookup
capability or provenance-chain schema. A source reference requires an acknowledged
trustworthy observation from the selected instance. A peer's self-reported
`audit_ref`, description or JSON metadata cannot manufacture this trust. An
unacknowledged plausible string is not a reference. Where no trustworthy downstream
audit observation exists, omit `source_audit`; preserve the local audit status and
admitted source identity without inventing acknowledgement.

Complete/incomplete source audit statuses require acknowledged non-null references;
unavailable/not_required use null, under the existing owner's exact rules. Local
delivery audit and the source's audit remain distinct. A new public provenance
chain or native source payload needs its own ESS typed home before implementation;
this document introduces neither. The cases' complete-source example is a
hypothetical already-verified service observation, never trust in arbitrary MCP
peer data. Additional typed native result mapping remains required.

## Bounds, budgets and truthful observations

Honor each hop's independently admitted `OperationLimits`: request_bytes,
result_bytes, execution_ms, provider_ms and connect_ms. The downstream request must
fit its actual encoded JSON-RPC request limit; the inbound request independently
fits its own ingress limit before admitted mapping. Likewise count the actual
outbound response bytes and the separately encoded inbound envelope. Taking only
the minimum of two raw payload ceilings is insufficient: the complete service
wrapper, JSON escaping, provenance, metadata and duplicate tool text/structured
representations add bytes. Enforce exact UTF-8 octets, including multibyte content;
exactly the bound is accepted, one octet over is not. No provenance or required
representation may be dropped to fit. Existing outbound prefix/terminal rules,
including SSE assembled message accounting, are unchanged.

The document cases use literal synthetic downstream bytes and a complete safe
service fixture encoded into each inbound family. They measure representation
cost, not a production codec or native mapping. For the wrapper-overflow case the
raw downstream observation fits while the safe inbound representation does not.
The multibyte case fits a character count but exceeds the octet ceiling. A bound
refusal after possible dispatch establishes no non-execution or retry authority.

The original admitted execution deadline is conserved across authentication,
admission, dispatch and wrapping. Each phase receives at most the remaining
budget and its own applicable phase limit; neither hop nor credential maintenance
resets the caller's budget. Zero remaining time permits no new dispatch. Time
spent processing or encoding is charged too. A timeout after possible dispatch
cannot claim non-execution; final delivery can fail after a host has a known
observation. The cases use explicit hypothetical elapsed costs, not a real clock
or proof of deadline enforcement.

A complete peer response establishes the peer's report, not arbitrary effect
certainty. A missing terminal boundary or partial observation remains incomplete;
modern `resultType:complete` does not prove business completeness. No automatic
redispatch follows a timeout, cancellation, malformed/oversized result, session
loss or caller disconnect. A live host with independent definitive applied
knowledge preserves it even if wrapping exceeds a limit. A caller that lost its
reply remains uncertain even when the host knows success. Recovery without that
definitive evidence remains unknown; current admitted keyed observation follows
the mutation owner and never creates a second attempt. The synthetic knowledge
facts are supplied premises, not an inference of applied from `isError:false`.

## Named document cases and proof boundary

[composition-cases.json](composition-cases.json) is document-test metadata only,
format `mcp-composition-document-cases/1`. Its hypothetical facts select a deciding
branch; they are neither public protocol fields nor assertions about today's
runtime. The guard requires all twelve successful revision/family combinations
and distinct refusal/provenance/bound/deadline/knowledge premises. It compares the
complete unique rendered table inventory below, refuses raw HTML, and does not
count fenced or quoted copies as the authoritative table. Negative controls keep
late cases from being silently replaced by earlier policy refusals. It also holds
the complete object-wrapper representations and literal outcomes to their owners.

These tests do not run MCP, credentials, a provider, admission/audit storage,
transport framing, protected approval, a native codec, a clock or a restart. They
prove document consistency and the named perturbations only. Before advertising
composition, executable conformance must cover actual ingress and egress codecs,
all selected revision/family pairs, exact target/credential isolation, independent
policy and revocation races, safe audit acknowledgement, wrapping/escaping byte
bounds, real deadline exhaustion, partial/lost replies and non-redispatch.

Seven native relations in [state.yaml](../../../spec/ess/domains/state.yaml) remain
unresolved: server binding to ServiceConfiguration, Connection, capability snapshot
and credential custody; general outbound HTTP/session binding lifetime; inbound
session to caller; and caller to Connection. This document selects no identity,
cardinality, persistence or deletion ownership for those mappings. The operator's
2026-10-03 stdio decision separately resolves the supervised-process subset: one
pinned process per outbound session, with its selected-server reference. The
caller-assignment blocker remains open. Contract guard success is no runtime
conformance claim and does not implement the newly selected stdio binding.

| Case | Inbound | Outbound | Family | Obligation | Outcome |
|---|---|---|---|---|---|
| `compose.paired.2026-07-28.2026-07-28.tools` | 2026-07-28 | 2026-07-28 | tools | paired | `preserved` |
| `compose.paired.2026-07-28.2026-07-28.resources` | 2026-07-28 | 2026-07-28 | resources | paired | `preserved` |
| `compose.paired.2026-07-28.2026-07-28.prompts` | 2026-07-28 | 2026-07-28 | prompts | paired | `preserved` |
| `compose.paired.2026-07-28.2025-11-25.tools` | 2026-07-28 | 2025-11-25 | tools | paired | `preserved` |
| `compose.paired.2026-07-28.2025-11-25.resources` | 2026-07-28 | 2025-11-25 | resources | paired | `preserved` |
| `compose.paired.2026-07-28.2025-11-25.prompts` | 2026-07-28 | 2025-11-25 | prompts | paired | `preserved` |
| `compose.paired.2025-11-25.2026-07-28.tools` | 2025-11-25 | 2026-07-28 | tools | paired | `preserved` |
| `compose.paired.2025-11-25.2026-07-28.resources` | 2025-11-25 | 2026-07-28 | resources | paired | `preserved` |
| `compose.paired.2025-11-25.2026-07-28.prompts` | 2025-11-25 | 2026-07-28 | prompts | paired | `preserved` |
| `compose.paired.2025-11-25.2025-11-25.tools` | 2025-11-25 | 2025-11-25 | tools | paired | `preserved` |
| `compose.paired.2025-11-25.2025-11-25.resources` | 2025-11-25 | 2025-11-25 | resources | paired | `preserved` |
| `compose.paired.2025-11-25.2025-11-25.prompts` | 2025-11-25 | 2025-11-25 | prompts | paired | `preserved` |
| `compose.caller-retarget` | 2026-07-28 | 2026-07-28 | tools | caller-retarget | `retarget-refused` |
| `compose.credential-forward` | 2026-07-28 | 2026-07-28 | tools | credential-forward | `credential-forward-refused` |
| `compose.account-fallback` | 2026-07-28 | 2026-07-28 | tools | account-fallback | `auth-unavailable` |
| `compose.metadata-authority` | 2026-07-28 | 2026-07-28 | tools | metadata-authority | `outbound-denied` |
| `compose.inbound-admission` | 2026-07-28 | 2026-07-28 | tools | inbound-admission | `inbound-denied` |
| `compose.outbound-admission` | 2026-07-28 | 2026-07-28 | tools | outbound-admission | `outbound-denied` |
| `compose.inbound-stale` | 2026-07-28 | 2026-07-28 | tools | inbound-stale | `stale-inbound` |
| `compose.outbound-unselected` | 2026-07-28 | 2026-07-28 | tools | outbound-unselected | `unselected-outbound` |
| `compose.delivery-admission` | 2026-07-28 | 2026-07-28 | tools | delivery-admission | `delivery-denied` |
| `compose.mapping-unavailable` | 2026-07-28 | 2026-07-28 | tools | mapping-unavailable | `unbound` |
| `compose.auth-unavailable` | 2026-07-28 | 2026-07-28 | tools | auth-unavailable | `auth-unavailable` |
| `compose.mutation-unbound` | 2026-07-28 | 2026-07-28 | tools | mutation-unbound | `mutation-omitted` |
| `compose.resource-write` | 2026-07-28 | 2026-07-28 | resources | resource-write | `resource-ineligible` |
| `compose.prompt-untyped` | 2026-07-28 | 2026-07-28 | prompts | prompt-untyped | `prompt-ineligible` |
| `compose.audit-peer` | 2026-07-28 | 2026-07-28 | tools | audit-peer | `preserved` |
| `compose.audit-unack` | 2026-07-28 | 2026-07-28 | tools | audit-unack | `preserved` |
| `compose.audit-trusted` | 2026-07-28 | 2026-07-28 | tools | audit-trusted | `preserved` |
| `compose.request-overflow` | 2026-07-28 | 2026-07-28 | tools | request-overflow | `request-bound` |
| `compose.result-overflow` | 2026-07-28 | 2026-07-28 | tools | result-overflow | `result-bound` |
| `compose.exact-bound` | 2026-07-28 | 2026-07-28 | tools | exact-bound | `preserved` |
| `compose.multibyte` | 2026-07-28 | 2026-07-28 | tools | multibyte | `result-bound` |
| `compose.remaining-deadline` | 2026-07-28 | 2026-07-28 | tools | remaining-deadline | `preserved` |
| `compose.deadline-exhausted` | 2026-07-28 | 2026-07-28 | tools | deadline-exhausted | `deadline-exhausted` |
| `compose.partial` | 2026-07-28 | 2026-07-28 | tools | partial | `incomplete` |
| `compose.lost-reply` | 2026-07-28 | 2026-07-28 | tools | lost-reply | `caller-unobserved` |
| `compose.host-known` | 2026-07-28 | 2026-07-28 | tools | host-known | `result-bound` |
| `compose.modern-array` | 2025-11-25 | 2026-07-28 | tools | modern-array | `preserved` |
| `compose.modern-null` | 2025-11-25 | 2026-07-28 | tools | modern-null | `preserved` |
