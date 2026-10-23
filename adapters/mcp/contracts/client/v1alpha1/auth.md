# Outbound MCP credential lifecycle

**Authored requirements and document cases; not an implemented OAuth client,
credential store, restart test or MCP runtime conformance result.** Owner:
`story:mcp-outbound-auth-lifecycle`. [Transport](semantics.md),
[invocation](invocation.md) and the [selection matrix](../../protocol/v1alpha1/selection.md)
retain their existing ownership.

## Binding prerequisite and credential boundaries

This contract specifies what a future selected outbound HTTP auth binding must
establish. It does not select the unresolved `McpServerBinding → Connection` or
`McpServerBinding → credential custody` relations. All eight unresolved native
relation markers remain. Before executable persistence work, give the chosen
binding its native ESS design, including owner, identity, publication/revocation
fences, replaceable custody and crash semantics. Do not claim that a
`material_handle` declares a foreign key, durability or a cardinality.

The existing `McpOutboundServerCredential` carries the remote resource URI,
expected issuer, granted scopes and opaque material handles. Shared
`CredentialGeneration`, `Connection`, `Acquisition` and `RefreshAttempt` describe
one candidate established owner, not an already-selected MCP mapping.
The [native state model](../../../spec/ess/domains/state.yaml) states this boundary.
Cases assume a hypothetical binding satisfying those prerequisites; their
`binding_available` fact is not evidence such a binding exists today.

Three credential domains stay distinct: inbound caller credentials, credentials
for the selected remote MCP server, and credentials used by that server for its
underlying provider. Never substitute or forward one for another. A discovered
endpoint, annotation, prompt, capability advertisement or previous connection
cannot authorize a new audience, account, issuer or credential selection.
Outbound stdio ownership was resolved by
`decision-blocker:mcp-outbound-stdio-process-ownership` on 2026-10-03. Its environment
and credential binding remain part of `story:mcp-outbound-stdio-runtime`; these HTTP
OAuth requirements do not supply them. The `stdio-held` cases below continue to refuse
stdio through this HTTP-only authorization profile until its own binding is authored.

## Selected HTTP authorization requirements

Both selected revisions, 2026-07-28 and 2025-11-25, are covered. Citations below
refer to uncompressed lines of the [pinned archives](../../protocol/v1alpha1/evidence/20260912/specification-sources.md).
The pin and matrix, not current online documents or a library default, are authority.

| Concern | Primary archive | Interoperability archive |
|---|---|---|
| Roles, acquisition and resource audience | `mcp-2026-07-28-basic-authorization-index.mdx:136`, `:215`, `:254` | `mcp-2025-11-25-basic-authorization.mdx:352`, `:402`, `:441` |
| Discovery | `mcp-2026-07-28-basic-authorization-authorization-server-discovery.mdx:11`, `:37` | `mcp-2025-11-25-basic-authorization.mdx:74`, `:131` |
| Registration choices | `mcp-2026-07-28-basic-authorization-client-registration.mdx:7` | `mcp-2025-11-25-basic-authorization.mdx:198` |
| Response issuer validation | `mcp-2026-07-28-basic-authorization-index.mdx:190` | No identical four-row modern response table is claimed for this revision. |
| Refresh confidentiality/rotation | `mcp-2026-07-28-basic-authorization-index.mdx:299` | `mcp-2025-11-25-basic-authorization.mdx:575` |

Protected Resource Metadata discovery must support the challenge-provided
`resource_metadata` location and the specified well-known fallback order, with
validated metadata and bounded admitted egress. Authorization-server discovery
uses the selected revision's required endpoint ordering. A discovery request is
metadata work; it neither retries the failed business call nor lets an arbitrary
redirect or fetched URL receive the credential. Validate destinations and metadata
before trusting the issuer or authorization/token endpoints.

Registration declares which mechanism it implements; no implicit registration
fallback is permission. The archives name pre-registration, Client ID Metadata
Documents and dynamic registration. For clients supporting all three, the stated
priority is existing pre-registration, advertised metadata-document support, then
advertised dynamic registration, then protected user entry. This document does
not assert all three are implemented or create a public metadata-hosting service.
Missing selected registration is a safe acquisition refusal, not anonymous use.

Browser acquisition uses validated endpoints, exact allowed redirect binding,
PKCE S256 with verified support, fresh single-use correlation and a protected
completion channel. Include the canonical MCP resource in authorization and token
requests. A credential may be sent only to its bound MCP resource and authority;
never put access tokens in query strings or forward them to provider endpoints.
A pasted credential is admitted only by a separately declared protected-entry
profile with issuer, audience, material and validation requirements; pasted bytes
are not presumed valid OAuth, and static entry does not silently acquire refresh.

For modern responses, record the authentic issuer before redirect and apply the
archive's complete response table: advertised `iss` support plus absent `iss`
refuses; present `iss` must exactly match the recorded issuer regardless of
advertisement; absent `iss` with no advertised support may proceed to the other
checks. Compare decoded strings without case, port, trailing-slash or percent
normalization. A mismatched issuer also prevents using/displaying OAuth error
fields. Legacy still requires validated authority/resource binding; the modern
table is not retroactively presented as a legacy protocol requirement.

The document facts `issuer_matches` and `audience_matches` denote completion of
these revision-specific checks, not arbitrary claims supplied by a caller. The
small guard does not fetch metadata, validate a JWT, compare real URLs or execute
PKCE. Those remain explicit future native implementation/conformance obligations.

## Protected entry, publication and restart

[Acquisition](../../../../../contracts/auth/acquisition/v1alpha1/semantics.md),
[custody](../../../../../contracts/auth/custody/v1alpha1/semantics.md) and
[management](../../../../../contracts/auth/management.md) own generic guarantees.
Business permission never grants management permission. Begin, completion,
repair and revoke require their current admitted owner and authority. An opaque
reference is correlation, not a grant.

Ordinary begin/status results expose safe acquisition reference, expiry and action
kind only. A separately admitted confidential UI/ingress receives any actionable
browser/protected-entry continuation. No token, refresh token, code, PKCE verifier,
state, client secret, completion URL or private custody scope/version/generation
reference enters ordinary results, discovery, logs or audit. Without the protected
channel, interactive acquisition refuses before making a usable flow.

Consume one-use completion correlation under current authority and known expiry
before exchange. An unknown consumption acknowledgement grants no exchange.
Completion requires valid authority, identity and baseline grants, a complete
validated candidate, acknowledged durable custody, and acknowledged publication
under the current fence. Only then report completed and the safe connection
reference. Failures do not publish half a credential set. Exact UI/ingress codecs
and the MCP binding remain unselected implementation work.

The local baseline requires replaceable custody, normally the admitted OS Secret
Service collection. CLI exit must not erase the acknowledged active binding; a
fresh invocation resolves the same committed association and rechecks authority,
revocation, credential validity and custody availability. Only confirmed durable
custody plus publication permits reuse. In-memory capture, pointer guesses,
unknown writes, stale backups or process survival are not restart evidence.
No storage backend or new binding relation is created by these requirements.

## Four credential failures

| Observation | Named outcome | Shared error vocabulary | Safe consequence |
|---|---|---|---|
| Required material/binding absent | `missing` | `connection_not_ready` | Protected acquisition/repair may be offered; no anonymous dispatch. |
| Credential expiry established | `expired` | `connection_not_ready` | Separately admitted refresh if selected and usable, otherwise reauthorization; no anonymous dispatch. |
| Local revocation committed | `revoked` | `revoked` | Terminal cutoff persists across restart; no repair/re-enable of that record. |
| Required custody cannot be read | `custody-unavailable` | `unavailable` | Availability failure, not evidence of missing/revoked material; no anonymous dispatch. |

These are distinct named observations, not four invented `ErrorCode` variants or
new product states. The [admission model](../../../../../ess/domains/connection_admission.yaml)
keeps reauthorization, custody unavailability and revocation distinct. A locked,
denied or failed custody read is not permission to try another account or treat
configured authenticated access as credential-free. Management status/revoke can
remain admitted despite provider unavailability; that exception grants no business
or token-endpoint work. Explicit credential-free profiles elsewhere are not a
fallback from this authenticated binding.

## Refresh and recovery

Refresh is separately admitted private maintenance, not an ordinary caller verb.
Refresh tokens are optional: absence does not imply permission to acquire one or
send an expired access token. Keep rotating material confidential and do not reuse
the SaaS static-entry exception that requires non-rotating refresh tokens.
A provider 401/403 may justify reporting an auth problem or a separately selected
maintenance action; neither a challenge nor a repaired credential authorizes an
automatic business request replay.

The [shared refresh model](../../../../../ess/domains/refresh.yaml) and acquisition
§4.1–4.2 require one fenced coordinator authority for the source generation.
Custody CAS, a local mutex or lease expiry alone does not authorize an exchange.
Only the original live invocation observing the definite atomic
`Reserved → Authorized` success can send once; rereading Authorized cannot send.
Provider clients must not implicitly repeat the exchange.

After possible exchange, loss of the result or unknown commit/store acknowledgement
retains consumed-source uncertainty. Resolve through the guarded owner-loss
operation to terminal `Uncertain`, requiring repair/reauthorization; never exchange
again merely because the active pointer looks unchanged. A committed
`ResponseStored` candidate may instead recover publication only: fence its old
owner, recheck current authority, revocation, identity, grants, expiry and custody,
then publish exactly that candidate. No new token exchange. A stale fence, invalid
candidate, wrong state or lost recovery owner refuses; do not recover an orphan
blob or memory-only response as committed evidence.

The examples' refresh-next field describes the required observation in these
specific owner-loss/publication cases. It is not a new transition implementation,
an instruction for a stale caller to change metadata, or a claim that a rotating
refresh flow ran. Actual atomic joins and native binding are future ESS work.

## Repair and revocation

Repair is separately admitted for the existing non-revoked identity. Reject an
identity/issuer/audience mismatch or insufficient baseline scope; never silently
switch accounts. Failed independent repair preserves a still-valid prior binding,
subject to its own expiry, revocation and authority. Successful repair publishes
only after both custody and fence acknowledgements. It does not run the previously
failed business operation.

Local revocation is acknowledged only when the cutoff definitely commits. It
blocks future dispatch and survives restart even if provider revocation is absent,
unsupported, pending or failed. Provider revocation remains a separately admitted
bounded action and its result is reported separately. Unknown local commitment
is outcome uncertainty, not confirmed revocation or renewed authority. Repeated
observation does not repeat a potentially consumed provider revocation. Already
in-flight provider effects are not undone by local cutoff or secret deletion.

## Named document cases and evidence limits

[auth-cases.json](auth-cases.json) is `mcp-auth-document-cases/1`, containing
hypothetical observations and literal outcomes for each selected revision. Boolean
facts summarize trusted checks; they are not a product input schema or tokens.
No secrets occur in the examples.
The examples' `token_exchanges` counts additional sends permitted by that
observation. Completion cases already have a validated candidate and permit no
new exchange. `refresh-once` describes the original definite live authorization
from Reserved, not a restart reading an Authorized record.
The guard derives revisions, shared error names
and refresh-state vocabulary from their real owners. It requires unique complete
case/document correspondence and contradicting-copy controls. It does not prove
the facts, native persistence, timing, OAuth or actual MCP conformance. Citation
existence checks do not replace source interpretation or pin-integrity checks.

The checked authoring subset uses Markdown tables with text and inline code.
Raw HTML is unsupported throughout this document, both inline and block forms;
the guard refuses it rather than guessing its rendered inventory. Fenced code
examples remain non-authoritative text and cannot supply required case rows.

| Case | Revision | Obligation | Outcome |
|---|---|---|---|
| `auth.2026-07-28.missing` | 2026-07-28 | missing | `missing` |
| `auth.2026-07-28.expired` | 2026-07-28 | expired | `expired` |
| `auth.2026-07-28.revoked` | 2026-07-28 | revoked | `revoked` |
| `auth.2026-07-28.unavailable` | 2026-07-28 | unavailable | `custody-unavailable` |
| `auth.2026-07-28.restart` | 2026-07-28 | restart | `credential-ready` |
| `auth.2026-07-28.unacknowledged` | 2026-07-28 | unacknowledged | `persistence-unacknowledged` |
| `auth.2026-07-28.protected-entry` | 2026-07-28 | protected-entry | `protected-action` |
| `auth.2026-07-28.no-protected-channel` | 2026-07-28 | no-protected-channel | `protected-channel-unavailable` |
| `auth.2026-07-28.completion` | 2026-07-28 | completion | `acquisition-published` |
| `auth.2026-07-28.issuer` | 2026-07-28 | issuer | `authority-mismatch` |
| `auth.2026-07-28.audience` | 2026-07-28 | audience | `authority-mismatch` |
| `auth.2026-07-28.consumption` | 2026-07-28 | consumption | `completion-refused` |
| `auth.2026-07-28.refresh-once` | 2026-07-28 | refresh-once | `refresh-send-once` |
| `auth.2026-07-28.refresh-uncertain` | 2026-07-28 | refresh-uncertain | `refresh-uncertain` |
| `auth.2026-07-28.refresh-authorized` | 2026-07-28 | refresh-authorized | `refresh-uncertain` |
| `auth.2026-07-28.refresh-recovery` | 2026-07-28 | refresh-recovery | `publication-only-recovery` |
| `auth.2026-07-28.refresh-wrong-state` | 2026-07-28 | refresh-wrong-state | `recovery-refused` |
| `auth.2026-07-28.repair` | 2026-07-28 | repair | `repair-published` |
| `auth.2026-07-28.repair-identity` | 2026-07-28 | repair-identity | `identity-mismatch` |
| `auth.2026-07-28.revoke` | 2026-07-28 | revoke | `locally-revoked` |
| `auth.2026-07-28.revoke-unknown` | 2026-07-28 | revoke-unknown | `revocation-unacknowledged` |
| `auth.2026-07-28.stdio` | 2026-07-28 | stdio | `stdio-held` |
| `auth.2026-07-28.unselected-binding` | 2026-07-28 | unselected-binding | `binding-unselected` |
| `auth.2026-07-28.management-admission` | 2026-07-28 | management-admission | `management-refused` |
| `auth.2025-11-25.missing` | 2025-11-25 | missing | `missing` |
| `auth.2025-11-25.expired` | 2025-11-25 | expired | `expired` |
| `auth.2025-11-25.revoked` | 2025-11-25 | revoked | `revoked` |
| `auth.2025-11-25.unavailable` | 2025-11-25 | unavailable | `custody-unavailable` |
| `auth.2025-11-25.restart` | 2025-11-25 | restart | `credential-ready` |
| `auth.2025-11-25.unacknowledged` | 2025-11-25 | unacknowledged | `persistence-unacknowledged` |
| `auth.2025-11-25.protected-entry` | 2025-11-25 | protected-entry | `protected-action` |
| `auth.2025-11-25.no-protected-channel` | 2025-11-25 | no-protected-channel | `protected-channel-unavailable` |
| `auth.2025-11-25.completion` | 2025-11-25 | completion | `acquisition-published` |
| `auth.2025-11-25.issuer` | 2025-11-25 | issuer | `authority-mismatch` |
| `auth.2025-11-25.audience` | 2025-11-25 | audience | `authority-mismatch` |
| `auth.2025-11-25.consumption` | 2025-11-25 | consumption | `completion-refused` |
| `auth.2025-11-25.refresh-once` | 2025-11-25 | refresh-once | `refresh-send-once` |
| `auth.2025-11-25.refresh-uncertain` | 2025-11-25 | refresh-uncertain | `refresh-uncertain` |
| `auth.2025-11-25.refresh-authorized` | 2025-11-25 | refresh-authorized | `refresh-uncertain` |
| `auth.2025-11-25.refresh-recovery` | 2025-11-25 | refresh-recovery | `publication-only-recovery` |
| `auth.2025-11-25.refresh-wrong-state` | 2025-11-25 | refresh-wrong-state | `recovery-refused` |
| `auth.2025-11-25.repair` | 2025-11-25 | repair | `repair-published` |
| `auth.2025-11-25.repair-identity` | 2025-11-25 | repair-identity | `identity-mismatch` |
| `auth.2025-11-25.revoke` | 2025-11-25 | revoke | `locally-revoked` |
| `auth.2025-11-25.revoke-unknown` | 2025-11-25 | revoke-unknown | `revocation-unacknowledged` |
| `auth.2025-11-25.stdio` | 2025-11-25 | stdio | `stdio-held` |
| `auth.2025-11-25.unselected-binding` | 2025-11-25 | unselected-binding | `binding-unselected` |
| `auth.2025-11-25.management-admission` | 2025-11-25 | management-admission | `management-refused` |
