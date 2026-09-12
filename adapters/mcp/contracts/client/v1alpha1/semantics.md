# Outbound MCP connection lifecycle — Streamable HTTP

**Status:** an authored contract, not an implementation. It specifies what the outbound
side of this repository does over the **Streamable HTTP** transport, which
[the protocol selection](../../protocol/v1alpha1/selection.md) dispositions `supported`
for this direction. No runtime, client, fixture server or executable is added by it, no
MCP connection has been made from this repository, and nothing here is evidence that any
MCP server implements anything.

Owner: `story:mcp-outbound-connection-lifecycle`, decomposing `epic:mcp-contracts`. It
reads three landed units and selects nothing of its own:

| Input | What this document takes from it |
|---|---|
| [The specification pin](../../protocol/v1alpha1/evidence/20260912/specification-sources.md) | The 54 archived files every citation below resolves into. A citation is written `<archived file>:<line>` of the **uncompressed** archived bytes, the only line number a digest pins. |
| [The native model](../../../spec/ess/system.yaml) | The nouns. `connectors_mcp.state.McpServerBinding`, `McpOutboundSession`, `McpCapabilitySnapshot` and the `connectors_mcp.protocol` values are read here, never redeclared, and no relation the model marks `UNMAPPED:` is settled by a sentence of this document. |
| [The selection matrix](../../protocol/v1alpha1/selection.md) | Which revisions, transports and capabilities exist for this direction at all. **This document specifies what the selected things do; it does not select.** A row dispositioned `deferred` there is not promoted here. |

Refusals are named in this repository's own public vocabulary,
`connectors.service_wire.ErrorCode`, declared in `ess/domains/service_wire.yaml` and
owned by `contracts/service/compatibility.md`. An outcome that is not a refusal carries
`n/a` in that column: it is an observation, not an error. Every refusal also names
**whose act it reports**, because a code that blames the wrong party is a truthful-looking
lie: `peer` for something the server did, `client` for something this repository or an
intermediary on its side did, `operator` for something only configuration decides,
`unobserved` where nothing was seen at all. Each refusal below says the same thing in its
own section, in one sentence beginning *The act reported is* — a column no prose repeats
is a label nobody has checked.

## Which revision a passage holds for

The selection matrix dispositions **two** revisions `supported` in this direction, and
they disagree — about sessions, about server-initiated requests, about what cancels a
request and about whether a stream can be continued. A passage that names neither states
one of them as if it were the transport's, which is how a rule can be right, cited, and
false for a binding this repository lets an operator configure.

So every numbered section below says what it holds for each, and every scenario names the
binding it is about: `revision:2026-07-28` **the primary revision**, and
`revision:2025-11-25` **the interoperability revision**. A binding speaks one of them and
never both; where they agree the section says so in one sentence, and where they disagree
each gets its own row of the register.

## The outcome register

Every lifecycle state `story:mcp-outbound-connection-lifecycle` names appears here with a
named observable outcome and the scenario that decides it. `crates/connectors-build/tests/mcp_outbound_connection_lifecycle.rs`
derives the required states from the story's own text, so a state missing from this table
is a defect in this table rather than a silent gap.

| State | Observable outcome | `ErrorCode` | Whose act | Scenario |
|---|---|---|---|---|
| `state:discovery` | `mcp.outbound.server-described` | `n/a` | n/a | [`discovery-records-without-selecting.yaml`](scenarios/discovery-records-without-selecting.yaml) |
| `state:explicit-selection` | `mcp.outbound.server-selected` | `n/a` | n/a | [`explicit-selection-of-a-configured-binding.yaml`](scenarios/explicit-selection-of-a-configured-binding.yaml) |
| `state:explicit-selection` | `mcp.outbound.selection-refused-unconfigured-endpoint` | `route_refused` | operator | [`discovered-endpoint-does-not-become-a-selection.yaml`](scenarios/discovered-endpoint-does-not-become-a-selection.yaml) |
| `state:initialize-negotiation` | `mcp.outbound.legacy-initialize-negotiated` | `n/a` | n/a | [`legacy-initialize-negotiates-version-and-capabilities.yaml`](scenarios/legacy-initialize-negotiates-version-and-capabilities.yaml) |
| `state:initialize-negotiation` | `mcp.outbound.legacy-initialize-answered-an-unconfigured-revision` | `unsupported` | peer | [`legacy-initialize-answered-an-unconfigured-revision.yaml`](scenarios/legacy-initialize-answered-an-unconfigured-revision.yaml) |
| `state:version-mismatch` | `mcp.outbound.version-refused` | `unsupported` | peer | [`version-mismatch-names-what-the-server-reported.yaml`](scenarios/version-mismatch-names-what-the-server-reported.yaml) |
| `state:version-mismatch` | `mcp.outbound.era-ambiguous-answer-refused` | `upstream_protocol` | peer | [`era-ambiguous-400-does-not-fall-back.yaml`](scenarios/era-ambiguous-400-does-not-fall-back.yaml) |
| `state:capability-mismatch` | `mcp.outbound.required-client-capability-refused` | `unsupported` | peer | [`required-client-capability-is-refused-not-approximated.yaml`](scenarios/required-client-capability-is-refused-not-approximated.yaml) |
| `state:capability-mismatch` | `mcp.outbound.advertised-capability-recorded-not-projected` | `n/a` | n/a | [`unknown-advertised-capability-is-not-support.yaml`](scenarios/unknown-advertised-capability-is-not-support.yaml) |
| `state:framing` | `mcp.outbound.request-framed` | `n/a` | n/a | [`one-request-is-one-post-with-agreeing-headers.yaml`](scenarios/one-request-is-one-post-with-agreeing-headers.yaml) |
| `state:framing` | `mcp.outbound.framing-refused` | `upstream_protocol` | peer | [`unreadable-answer-is-not-the-callers-input.yaml`](scenarios/unreadable-answer-is-not-the-callers-input.yaml) |
| `state:framing` | `mcp.outbound.header-mismatch-reported-by-the-server` | `internal` | client | [`header-body-mismatch-is-not-the-callers-input.yaml`](scenarios/header-body-mismatch-is-not-the-callers-input.yaml) |
| `state:streaming` | `mcp.outbound.stream-opened` | `n/a` | n/a | [`streamed-answer-carries-only-its-own-request.yaml`](scenarios/streamed-answer-carries-only-its-own-request.yaml) |
| `state:streaming` | `mcp.outbound.legacy-ping-answered` | `n/a` | n/a | [`legacy-ping-on-the-stream-is-answered.yaml`](scenarios/legacy-ping-on-the-stream-is-answered.yaml) |
| `state:streaming` | `mcp.outbound.legacy-server-request-refused` | `unsupported` | peer | [`legacy-server-request-for-an-undeclared-capability.yaml`](scenarios/legacy-server-request-for-an-undeclared-capability.yaml) |
| `state:streaming` | `mcp.outbound.stream-ended-without-an-answer` | `outcome_unknown` | unobserved | [`stream-ends-without-a-final-response.yaml`](scenarios/stream-ends-without-a-final-response.yaml) |
| `state:progress` | `mcp.outbound.progress-observed` | `n/a` | n/a | [`progress-is-display-only-and-bounded.yaml`](scenarios/progress-is-display-only-and-bounded.yaml) |
| `state:cancellation` | `mcp.outbound.cancelled-by-closing-the-stream` | `n/a` | n/a | [`cancellation-closes-the-response-stream.yaml`](scenarios/cancellation-closes-the-response-stream.yaml) |
| `state:cancellation` | `mcp.outbound.legacy-cancellation-notified` | `n/a` | n/a | [`legacy-cancellation-is-sent-not-implied.yaml`](scenarios/legacy-cancellation-is-sent-not-implied.yaml) |
| `state:session-loss` | `mcp.outbound.session-lost` | `session_lost` | peer | [`legacy-session-loss-is-not-a-new-session.yaml`](scenarios/legacy-session-loss-is-not-a-new-session.yaml) |
| `state:transport-unreachable` | `mcp.outbound.transport-unavailable` | `unavailable` | unobserved | [`unreachable-endpoint-is-unavailable-not-loss.yaml`](scenarios/unreachable-endpoint-is-unavailable-not-loss.yaml) |
| `state:connection-lost-in-flight` | `mcp.outbound.answer-never-arrived` | `outcome_unknown` | unobserved | [`single-object-answer-lost-in-flight.yaml`](scenarios/single-object-answer-lost-in-flight.yaml) |
| `state:shutdown` | `mcp.outbound.shutdown-completed` | `n/a` | n/a | [`shutdown-closes-every-stream-it-opened.yaml`](scenarios/shutdown-closes-every-stream-it-opened.yaml) |

The scenario files are not `ess-scenario/1` traces. `connectors_mcp` declares no command
and every entity in it carries a single-state lifecycle, because `story:mcp-domain-model`
models nouns and declares no behaviour; an `ess-scenario/1` file names commands and
outcomes of a domain that has them. These carry `type: mcp-outbound-lifecycle/1` instead,
and the case above is what holds them to this register.

## 1. Discovery — what a server reports, and what that changes

`server/discover` is the primary revision's description request, and exists in no other.
Servers **MUST** implement it and clients **MAY** call it before anything else
(`mcp-2026-07-28-basic-versioning.mdx:73-78`); its result carries the server's supported
protocol versions, the keys of its advertised capabilities, an optional self-reported
identity and optional caching hints (`mcp-2026-07-28-server-discover.mdx:33-60`).

That result is recorded as the model's `McpCapabilitySnapshot` value, whose
`supported_versions` is a `List<String>` and whose `server_capabilities` is a
`List<String>` — because a server may name versions and capability keys neither pinned
revision covers (`mcp-2026-07-28-server-discover.mdx:94-95`,
`mcp-2026-07-28-schema.ts:789`). A reported string that matches no row of the selection
matrix is still recorded exactly as reported.

The interoperability revision has no description request at all. What a server reports
there is the `InitializeResult` of the handshake in section 3, recorded the same way and
under the same rule; a binding configured for it calls `server/discover` on nobody, and
the absence of a description is not a reason to select anything.

**Discovery describes; it never selects.** The rule is this repository's, not MCP's:
`contracts/service/compatibility.md` section 2 — "a discovery observation cannot switch
the endpoint, credential, version or profile". `epic:mcp-contracts` states the same line
for composition: "prohibit implicit credential forwarding, account fallback or authority
escalation from discovered metadata". The server's self-reported identity is display data
by its own specification, which says clients **SHOULD NOT** use it to change their
behavior and **SHOULD NOT** rely on it for security decisions
(`mcp-2026-07-28-server-discover.mdx:103-107`).

### `mcp.outbound.server-described`

An observer sees the reported versions, the advertised capability keys, the self-reported
name and version and any `ttlMs`/`cacheScope` hint, each labelled as *reported by the
server*. Every field of the `McpServerBinding` is byte-identical before and after:
`canonical_uri`, `transport_binding`, `determined_era` and `selected_revision` are
unchanged, no credential is acquired, and no capability is projected into a Connectors
capability. Whether the snapshot is retained past the process that took it is not stated
here: `McpServerBinding → McpCapabilitySnapshot` is `UNMAPPED:` in
`story:mcp-domain-model`, so nothing in this document implies a durable record of it.

## 2. Explicit selection of a server

An outbound server is selected by operator configuration and by nothing else. A selection
names the endpoint, the transport binding and **one** revision the binding speaks: the
primary revision or the interoperability revision, never both. `selected_revision` is the
model's only carrier and holds one string, and the outcome below is one revision declared
on every request, so a binding configured for a pair would be observable by nobody. What
era a *server* turns out to speak is a different question, and section 4 is where a probe
of it stops. `epic:mcp-contracts` requires "discovery and explicit
selection of a server", and the two words are kept apart here exactly as
`contracts/service/compatibility.md` section 2 keeps them apart for the service families:
"A configured endpoint plus this fixed path identifies the binding".

`selected_revision` is a `String` on `McpServerBinding`, not a two-variant enum, and this
document does not narrow it. A server may report `2025-06-18` or `2024-11-05` — both
appear as version strings inside the pinned archives, and the selection matrix refuses
both — and a closed carrier would make a reportable version unrepresentable rather than
refusable. Refusing a version requires being able to hold it first.

### `mcp.outbound.server-selected`

An observer sees the binding the operator configured: one endpoint, one transport
binding, one revision the binding will declare on every request. Nothing about that set
came from the wire. The same configuration selected before the first request is the one
in force after any number of discovery observations, and the CLI can print it without
having spoken to the server at all. Whether that record is one
`connectors.declarations.ServiceConfiguration`, one `connectors.auth_bindings.Connection`
or neither is `UNMAPPED:` in `story:mcp-domain-model` and is not decided by this sentence.

### `mcp.outbound.selection-refused-unconfigured-endpoint`

An observer sees `route_refused`, naming the endpoint that was declined and the
configured binding that was in force. This is what happens when anything other than
operator configuration proposes an endpoint: a URL inside a discovery result, a redirect
to a second origin, a server-supplied instruction, an `endpoint` event of the deprecated
HTTP+SSE transport. No request is sent to the declined endpoint, no credential is
offered to it, and the configured binding is left exactly as it was. The act reported is
the operator's: an endpoint is in the binding or it is not, and only a configuration an
operator wrote puts it there. An operator reconfiguration is a new interaction, not a
continuation of this one.

## 3. What `initialize` negotiates

There is no `initialize` in the primary revision. `mcp-2026-07-28-basic-versioning.mdx:12-13`
says there is no negotiation handshake: every request carries its own protocol version
and the server accepts or rejects each request independently, and
`mcp-2026-07-28-basic-index.mdx:184-187` declares MCP "a **stateless protocol**" whose
server "processes each request independently". So for a binding whose selected revision is
`2026-07-28`, the answer to "what does `initialize` negotiate" is: nothing, because none
is sent.

`initialize` is the **interoperability** revision's handshake, and the selection matrix
dispositions `revision:2025-11-25` outbound as `supported`, so this document specifies it
for a binding whose configuration names that revision. It establishes protocol version
compatibility, exchanges capabilities and shares implementation details
(`mcp-2025-11-25-basic-lifecycle.mdx:42-53`); the client follows a successful handshake
with an `initialized` notification before ordinary requests
(`mcp-2025-11-25-basic-lifecycle.mdx:149-157`), and both parties may then use only the
capabilities that were negotiated (`mcp-2025-11-25-basic-lifecycle.mdx:186-212`).

Connectors declares the client capabilities the selection matrix gives it, which is none
of `roots`, `sampling` or `elicitation` — every client capability there is `explicitly
refused` or `deferred`. The handshake therefore negotiates: one protocol version, the
server's advertised capability set, and an empty client capability set.

### `mcp.outbound.legacy-initialize-negotiated`

An observer sees one `initialize` exchange whose result names a protocol version the
binding's configuration already names, the server's advertised capabilities recorded as
reported, and a client capability object that declares nothing. The `McpOutboundSession`
moves from `initialization` to `operation` in the protocol's own phase vocabulary
(`connectors_mcp.protocol.LegacySessionPhase`), and the wire session id, if the server
assigned one, is recorded as `wire_session_id` and echoed on every subsequent request
(`mcp-2025-11-25-basic-transports.mdx:194-222`). Nothing else about the server changes
what the binding speaks.

### `mcp.outbound.legacy-initialize-answered-an-unconfigured-revision`

An observer sees `unsupported`, naming the revision the server answered with and the
revision the binding is configured for, and the connection closed. The interoperability
revision permits the server to answer with a different version of its own choosing and
tells a client that cannot use it to disconnect (`mcp-2025-11-25-basic-lifecycle.mdx:167-177`);
The act reported is the peer's: the server chose a version of its own. Connectors
disconnects for a stricter reason as well — the answered revision was not selected by an
operator, and a server's answer is not a selection. No second handshake is
opened, no second revision is offered, and no request is carried out under the revision
the server named.

## 4. Protocol version mismatch

**Under the primary revision** a server that does not implement the requested version
**MUST** answer `UnsupportedProtocolVersionError` (`-32022`,
`mcp-2026-07-28-schema.ts:450`) listing the versions it does support
(`mcp-2026-07-28-basic-versioning.mdx:48-52`), and over HTTP that answer is a
`400 Bad Request` (`mcp-2026-07-28-basic-transports-streamable-http.mdx:263-269`). The
error body carries both halves: `supported`, a list of strings, and `requested`
(`mcp-2026-07-28-basic-versioning.mdx:54-67`).

**Under the interoperability revision there is no such error.** The version is settled
once, by the handshake in section 3; afterwards the client **MUST** carry
`MCP-Protocol-Version` on every request, and a server that finds it invalid or unsupported
**MUST** answer `400 Bad Request` (`mcp-2025-11-25-basic-transports.mdx:267-269` and
`:281-282`) with no structured body to read. That is `mcp.outbound.version-refused` as
well, reported with the `supported` list absent rather than invented: that revision
defines no field to carry one, and a list this client made up would be the fallback it
refuses, wearing a server's name.

**A mismatch refuses; it does not degrade.** The rule is
`contracts/service/compatibility.md` section 2: "Unknown/missing routes or incompatible
descriptors stop selection", and "A raw HTTP 404 or malformed response is not renamed into
proof that a particular version is unsupported." The version the binding declares is the
one an operator configured; what a server reports is recorded and shown, never selected.
The rest of that sentence, and why this document takes it, is
[below](#why-no-outcome-above-sends-the-request-a-second-time).

### `mcp.outbound.version-refused`

An observer sees `unsupported`, carrying the endpoint, the version the binding declared
and — where the revision defines one, which is the primary revision alone — the exact
`supported` list the server reported, shown as empty where the interoperability revision's
bare `400` carried none. A reported list may name strings no pinned revision covers, such
as `2025-06-18` or `2024-11-05`, which are shown as reported and are not offered as
choices the client will take on its own. The act reported is the peer's: a server that
does not implement the version it was asked for said so. The request that met the mismatch
is not carried out and no other version is declared on its behalf. An operator who wants one
of the reported versions changes the binding's configuration, and that is a new
interaction with its own record.

### `mcp.outbound.era-ambiguous-answer-refused`

An observer sees `upstream_protocol`, naming the HTTP status and stating that the body
carried no recognised modern MCP error. This is the `400` the primary revision tells a
dual-era client to inspect before deciding what the server is
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:650-668`,
`mcp-2026-07-28-basic-versioning.mdx:126-146`). Connectors reads the body for the same
reason and stops at the same point: a recognised modern error is the refusal above, and
anything else is this outcome. The act reported is the peer's: it answered, and what it
answered was not readable as this protocol. A binding configured for one revision does not
begin speaking another because an answer was unreadable, and the deprecated HTTP+SSE
transport is never reached — the selection matrix refuses `transport:http-sse-transport`
outbound, and the fallback that would have landed there
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:710-737`) is declined at its first
step. Era remains a property of the server rather than of a request
(`mcp-2026-07-28-basic-versioning.mdx:148-152`), and this outcome is where a probe of it
stops. Whether what a probe settles outlives the process that ran it is not stated here:
`determined_era` is a field of the `McpServerBinding` entity, `McpOutboundSession →
McpServerBinding` is `UNMAPPED:` in `story:mcp-domain-model`, and an operator's
configuration decides what is spoken either way.

## 5. Capability mismatch

Two different things are called a capability mismatch, and they have different outcomes.

The first is a server needing a **client** capability that Connectors does not declare. A
server **MUST NOT** rely on capabilities the client has not declared and **MUST** answer
`MissingRequiredClientCapabilityError` (`-32021`, `mcp-2026-07-28-schema.ts:442`) naming
the missing ones (`mcp-2026-07-28-basic-index.mdx:387-392`). Connectors declares none of
`roots`, `sampling` or `elicitation`: the selection matrix refuses all three, so this
answer is expected rather than exceptional, and it is surfaced rather than worked around.

The second is a server advertising a capability Connectors does not consume — a key the
matrix refuses, like `logging` or `completions`, or a key no archived byte names at all,
which the schema explicitly permits (`mcp-2026-07-28-schema.ts:789`). That is not an
error. It is recorded as advertised and projected into nothing.

Neither error code exists in the interoperability revision. There capabilities are agreed
once, in the handshake, and both parties **MUST** use only what was negotiated
(`mcp-2025-11-25-basic-lifecycle.mdx:186-212`) — so a server that needs a client capability
Connectors did not declare has left the negotiated set, which is the same refusal named
below, and a server advertising one Connectors does not consume is the same observation.
The two outcomes hold for both revisions; only the codes that announce them are the
primary revision's.

### `mcp.outbound.required-client-capability-refused`

An observer sees `unsupported`, carrying the endpoint, the method that was refused and
the `requiredCapabilities` list the server named. Nothing is auto-answered on a user's
behalf: no elicitation form is filled in, no default is supplied, no model is sampled,
and no filesystem root is invented. The capability is not declared on a second attempt in
order to get past the refusal, because whether Connectors declares it at all is the
selection matrix's decision and not this document's. The act reported is the peer's: the
server required something of a client that never offered it. The caller learns which
capability the server wanted, which is the actionable half.

### `mcp.outbound.advertised-capability-recorded-not-projected`

An observer sees the advertised key listed exactly as the server sent it, marked as
advertised-and-not-consumed, and no Connectors capability appears because of it. This
holds for a refused key and for an unknown key alike: the snapshot's
`server_capabilities` is a `List<String>` precisely so an unknown key has somewhere to go
(`mcp-2026-07-28-server-discover.mdx:94-95`). It is not an error, it is not support, and
it never becomes an admission decision, an authority, or a reason to send a request that
would depend on it.

## 6. Framing

**Under the primary revision** every JSON-RPC message the client sends is its own HTTP
POST to the MCP endpoint; the client **MUST** offer both `application/json` and
`text/event-stream` in `Accept`, **MUST** carry the request-metadata headers, **MUST** put
exactly one JSON-RPC request or notification in the body and **MUST NOT** send JSON-RPC
responses (`mcp-2026-07-28-basic-transports-streamable-http.mdx:70-91`). The server answers
a request with either a single JSON object or an SSE stream and the client supports both.

Three headers are required and are mirrors of body fields: `MCP-Protocol-Version`, whose
value **MUST** equal the `io.modelcontextprotocol/protocolVersion` in the body's `_meta`,
`Mcp-Method` and `Mcp-Name` (`mcp-2026-07-28-basic-transports-streamable-http.mdx:250-261`
and `:286-293`). The body's `_meta` carries the protocol version and the client
capabilities as required fields on every request
(`mcp-2026-07-28-basic-index.mdx:373-382`). A server that finds header and body
disagreeing **MUST** answer `400 Bad Request` with `HeaderMismatch` (`-32020`,
`mcp-2026-07-28-schema.ts:434`, `mcp-2026-07-28-basic-transports-streamable-http.mdx:580-607`).

**Under the interoperability revision the shape is the same and the required headers are
not.** Each message is its own POST, the client **MUST** offer both media types, and the
server **MUST** answer a request with either one JSON object or an SSE stream, both of
which the client **MUST** support (`mcp-2025-11-25-basic-transports.mdx:90-106`). There is
no `Mcp-Method`, no `Mcp-Name`, no `_meta` mirror and no `HeaderMismatch` code. What is
required instead is `MCP-Protocol-Version` on every request after the handshake
(`mcp-2025-11-25-basic-transports.mdx:267-269`) and the `MCP-Session-Id` header echoed
wherever the server assigned one (`mcp-2025-11-25-basic-transports.mdx:208-210`). And the
body of a POST **may** be a JSON-RPC response there, which is how this client answers the
one request a server is allowed to send it — section 7.

### `mcp.outbound.request-framed`

An observer sees one POST per JSON-RPC message, carrying one request or one notification,
with `Accept` offering both media types; under the primary revision the three mirrored
headers agree byte-for-byte with the body they mirror, and under the interoperability
revision `MCP-Protocol-Version` and any assigned `MCP-Session-Id` are on every request
after the handshake. A notification is answered `202 Accepted` with no body. The protocol
version the header declares is the binding's configured revision, identical on every
request of that binding. Under the primary revision no POST of this repository ever
carries a JSON-RPC response as its body, which that revision forbids; under the
interoperability revision exactly one kind does — the answer to a server request, below —
and it carries nothing else.

### `mcp.outbound.framing-refused`

An observer sees `upstream_protocol`, carrying the endpoint, the HTTP status and what was
unreadable: a body that is not readable as JSON-RPC, or an answer whose media type is
neither of the two the transport defines. The act reported is the peer's, on either
revision — the client offered both media types and the answer was neither one of them nor
readable —
and it is deliberately **not** `invalid_input`: the caller supplied no framing, so blaming
the caller's input for a framing disagreement would be a false attribution. The caller's
request is not carried out. What a truthful result looks like once an answer *is* readable
belongs to `story:mcp-outbound-invocation-results`.

### `mcp.outbound.header-mismatch-reported-by-the-server`

Primary revision only. An observer sees `internal`, carrying the endpoint, the header and
the body value that disagreed, and the `-32020` the server returned. A server answering
that has spoken the protocol correctly: it is reporting that *this client's* header and
its own body did not agree — a condition `mcp.outbound.request-framed` says this client
never produces, so it arises only from this repository or from an intermediary that
rewrote a header in flight. The act reported is this client's. `upstream_protocol` would
blame a peer for its own correct report, and `invalid_input` would blame a caller who
supplied no header; `internal` is the code that names the side that can fix it. The
interoperability revision defines no header mirror and no code for a disagreement between
one and a body, so nothing reaches this outcome there.

## 7. Streaming

**Under the primary revision**, when the server answers with
`Content-Type: text/event-stream`, the stream is scoped to the one request that opened
it. The server **MAY** send notifications related to that request before the final
response and **MUST NOT** send independent JSON-RPC requests on
it — server-initiated interactions are embedded in an `InputRequiredResult` instead — and
the final response **SHOULD** terminate the stream
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:107-125`). Long-lived notification
streams are a separate request, `subscriptions/listen`, whose filter names the
notification types the client opted into and whose first message is an acknowledgment
(`mcp-2026-07-28-basic-patterns-subscriptions.mdx:12-16` and `:52-56`).

**Under the primary revision there is no resumption.**
`mcp-2026-07-28-basic-transports-streamable-http.mdx:157` states that resumable SSE
streams via `Last-Event-ID` are not supported, and `:670-678` records that the earlier
shape — server-assigned session ids, a standalone GET stream, server-sent requests and
resumable streams — is not part of it. A stream that ends has ended; nothing about it is
continued from a cursor.

**The interoperability revision has all four of those, and this document takes one.**
There the server **MAY** send JSON-RPC requests and notifications on the response stream
before the response (`mcp-2025-11-25-basic-transports.mdx:121-123`), and `ping` is a
request whose receiver **MUST** answer promptly with an empty result
(`mcp-2025-11-25-basic-utilities-ping.mdx:31-39`). So this client answers it — and `ping`
is the only server request it can legitimately meet, because it declares no client
capability for a server to use and a negotiated set is what a server may rely on
(`mcp-2025-11-25-basic-lifecycle.mdx:186-212`). A stream there may also be resumable
(`mcp-2025-11-25-basic-transports.mdx:107-118`, `:166-189`): a client that wishes to
continue a broken stream **SHOULD** reopen it with a `Last-Event-ID` GET and **MUST**
honour the reconnection delay the server sent before closing.

**This document does not continue a broken stream, and the cost is stated rather than
hidden.** A server that closes connections between events — which that revision permits
precisely so it need not hold one open — produces
`mcp.outbound.stream-ended-without-an-answer` here where a resuming client would have read
the answer. That is a lost answer, never a repeated effect: reopening a stream reads what
was already computed, so this cost is weighed against adding a mechanism the primary
revision has removed, not against the rule in the last section of this document. This
client also opens no standalone GET stream, so a server request reaches it only on the
stream its own request opened.

### `mcp.outbound.stream-opened`

An observer sees a stream bound to exactly one request id, carrying only notifications
that relate to it and terminating with that request's response, on either revision.

Under the primary revision that stream carries no server-initiated request at all.
A JSON-RPC *request* arriving on the stream is refused as `upstream_protocol` under
`mcp.outbound.framing-refused` rather than answered, because that revision defines no
channel for it — server-initiated interactions are embedded in an `InputRequiredResult`
instead. Under the interoperability revision the same arrival is answered, and that is the
outcome below. A `subscriptions/listen` stream carries only the notification types its
filter named, each tagged with its subscription id, and its acknowledgment arrives before
any notification of that subscription.

### `mcp.outbound.legacy-ping-answered`

Interoperability revision only. An observer sees a `ping` arrive on the response stream of
a request this client sent, and an empty result go back promptly on its own POST as a
JSON-RPC response body, carrying nothing else
(`mcp-2025-11-25-basic-utilities-ping.mdx:31-39`). That is the whole of this outcome: an
answer this revision requires, given. The client's own request is untouched by it — its
outcome is still whatever its own answer produces, and having answered a `ping` is not an
answer to it. A server request that is *not* a `ping` is a different event with a
different answer: it carries a code and names whose act it reports, and it is the refusal
below. One row cannot hold both, because a row carrying `n/a` says that no refusal
happened.

### `mcp.outbound.legacy-server-request-refused`

Interoperability revision only. An observer sees `unsupported`, naming the method the
server asked for and the empty set of client capabilities this binding declared at the
handshake, and a JSON-RPC error going back in answer — never silence, because that
revision lets a server wait for one. The act reported is the peer's: a server may rely
only on the capabilities the handshake negotiated
(`mcp-2025-11-25-basic-lifecycle.mdx:186-212`), and this repository negotiates none, so
any server request other than `ping` is one it was told not to send. Nothing is answered
on a caller's behalf: no model is sampled, no elicitation form is shown to a user and no
filesystem root is disclosed. The client's own request keeps its own outcome.

### `mcp.outbound.stream-ended-without-an-answer`

An observer sees `outcome_unknown`, naming the request whose stream closed before its
final response arrived, and the last progress value observed if there was one. The
specification calls this an unexpected disconnect and distinguishes it from the graceful
closure that carries a response (`mcp-2026-07-28-basic-patterns-subscriptions.mdx:128-135`
and `:155-157`). The act reported is nobody's: the answer that would have said what
happened is the thing that did not arrive. Connectors reports the uncertainty as
uncertainty: it does not report success, does not report failure, and does not send the
request again, because a request
whose answer was never seen may already have had its effect.
`story:mcp-outbound-invocation-results` owns what a caller does with an unknown outcome;
this document owns the observation that produced it.

## 8. Progress

**Under the primary revision** a client opts into progress by putting a `progressToken`
in a request's `_meta`; tokens
must be unique across active requests, and the server **MAY** then send
`notifications/progress` carrying that token, a monotonically increasing `progress`, an
optional `total` and an optional human-readable `message`
(`mcp-2026-07-28-basic-patterns-progress.mdx:13-18` and `:53-56`). A server may also send
none at all (`mcp-2026-07-28-basic-patterns-progress.mdx:60-62`), so absence of progress
is not evidence of absence of work.

Progress may reset a per-request timeout clock, but a maximum timeout is enforced
regardless (`mcp-2026-07-28-basic-patterns-cancellation.mdx:45-64`). Both parties are
expected to rate-limit, and notifications stop after completion
(`mcp-2026-07-28-basic-patterns-progress.mdx:86-90`).

**The interoperability revision's progress is the same mechanism with a different end.**
A `progressToken` in `_meta`, unique across active requests; a `progress` that **MUST**
increase; an optional `total` and an optional human-readable `message`
(`mcp-2025-11-25-basic-utilities-progress.mdx:15-20` and `:55-58`); a receiver that **MAY**
send none at all (`:66-69`). Its timeout rule is where it parts company with the primary
revision: an implementation **MAY** reset the clock on a progress notification and
**SHOULD** always enforce a maximum, and when that bound expires the sender **SHOULD**
issue a cancellation notification and stop waiting
(`mcp-2025-11-25-basic-lifecycle.mdx:248-262`). So a timeout ends in a closed stream on one
revision and in a sent notification on the other, which is why section 9 has two outcomes
and not one.

### `mcp.outbound.progress-observed`

An observer sees progress lines for the request that asked for them, in non-decreasing
order, each showing the server's `message` as server-supplied text. A notification
carrying a token that belongs to no active request of this client is ignored and
recorded, never attributed to another request. Progress is display: it is never a result,
never a partial result the caller may act on, and never authority. When progress stops
arriving the request's own timeout still expires, and the bounded outcome is the one
`state:cancellation` carries for the revision the binding speaks, not an indefinite wait.

## 9. Cancellation

The two revisions disagree about what cancels a request — one of them says the act the
other calls cancellation means nothing — so this section states both and the register
carries an outcome for each.

**Under the primary revision, cancellation is the transport act.** Closing the SSE
response stream **MUST** be treated by the server as cancellation of that request, the
disconnect is unambiguous because each request has its own stream,
and no `notifications/cancelled` message is expected
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:233-240`,
`mcp-2026-07-28-basic-patterns-cancellation.mdx:39-41`). A timeout is the same act: when
the answer has not arrived within the request's timeout the client closes the stream,
which constitutes cancellation (`mcp-2026-07-28-basic-patterns-cancellation.mdx:45-64`).

**Under the interoperability revision it is a message, and closing proves nothing.**
Disconnection **SHOULD NOT** be interpreted as the client cancelling its request, and to
cancel, the client **SHOULD** explicitly send a `CancelledNotification`
(`mcp-2025-11-25-basic-transports.mdx:128-133`). That notification names the request id and
an optional reason, **MUST NOT** be sent for `initialize`, and its sender **SHOULD** ignore
any response that arrives afterwards (`mcp-2025-11-25-basic-utilities-cancellation.mdx:15-19`,
`:34-48`). A timeout ends the same way there, by sending it
(`mcp-2025-11-25-basic-lifecycle.mdx:248-262`). A connection that drops on that revision is
therefore not a cancellation at all: where it leaves the request is section 10's question,
and the answer is one of that section's three.

This document states the cancellation signal for the Streamable HTTP transport and for no
other. Outbound stdio is held by
`decision-blocker:mcp-outbound-stdio-process-ownership`, whose section below says what
that means; the two remaining outbound families the selection matrix names are refused
there under no blocker at all, and this document states nothing about any of the three.

A late answer may still arrive for a cancelled request; the client **SHOULD** ignore it on
either revision (`mcp-2026-07-28-basic-patterns-cancellation.mdx:81-82`,
`mcp-2025-11-25-basic-utilities-cancellation.mdx:47-48`).

### `mcp.outbound.cancelled-by-closing-the-stream`

Primary revision only. An observer sees the request reported as cancelled by this client,
with the reason — caller cancellation or timeout expiry — and with the fact that the
effect at the server was not observed. A cancelled mutation leaves its caller exactly
where `mcp.outbound.stream-ended-without-an-answer` does, holding `outcome_unknown`:
cancellation stops the client's waiting, not the server's work. An answer that arrives
after cancellation is recorded as late and does not become the request's result, and a
subscription is ended the same way, by closing its stream
(`mcp-2026-07-28-basic-patterns-subscriptions.mdx:118-126`).

### `mcp.outbound.legacy-cancellation-notified`

Interoperability revision only. An observer sees a `notifications/cancelled` sent for the
request being cancelled, naming its id and the reason — caller cancellation or the expiry
of the maximum bound — followed by the client no longer waiting for that request. The
stream is not the signal there and closing it is not reported as cancellation; the effect
at the server is reported as unobserved for the same reason it is on the other revision,
because a receiver **MAY** ignore the notification if the work has already finished
(`mcp-2025-11-25-basic-utilities-cancellation.mdx:43-46`). `initialize` is never
cancelled this way, which its own revision forbids, and a response that arrives after the
notification is recorded as late and does not become the request's result.

## 10. Session loss, and the failure it is not

The story requires these two to stay apart, and the shared model already has two values
for them: `connectors.service_wire.ErrorCode` declares `unavailable` and `session_lost`
as separate variants.

A **session** exists only where the binding's selected revision is the interoperability
one. There the server **MAY** assign an `MCP-Session-Id` at initialization, the client
**MUST** echo it, the server **MAY** terminate it at any time and then answers `404` to
requests carrying it (`mcp-2025-11-25-basic-transports.mdx:194-222`). Under the primary
revision there is no session to lose: an open connection "is not a conversation or
session" (`mcp-2026-07-28-basic-index.mdx:204-209`).

Which leaves the ordinary case, on both revisions: a dropped connection is three outcomes,
not one, and which it is depends on how far the request got rather than on what the
connection was carrying. Nothing framed is `mcp.outbound.transport-unavailable`. An answer
being streamed that stops is `mcp.outbound.stream-ended-without-an-answer`. A request
framed and sent whose single-object answer — the other shape section 6 says a server may
choose — never arrived is `mcp.outbound.answer-never-arrived`, under
`state:connection-lost-in-flight`; without that row half the answer shapes this document
accepts would reach no row at all. It is deliberately not filed under
`state:transport-unreachable` beside the row above: that state's name asserts the
endpoint could not be reached, and this outcome exists only because it was. A `404`
repudiating a session is a fourth thing, `mcp.outbound.session-lost`, and exists only on
the revision that has sessions.

### `mcp.outbound.session-lost`

An observer sees `session_lost`, naming the endpoint and the session the server
repudiated, and — separately — `outcome_unknown` for any request that was in flight when
the `404` arrived. The two facts are reported as two facts: the session is gone, and what
happened to the in-flight request was not observed. The act reported is the peer's: a
server terminated a session it had assigned, which its own revision permits it to do at
any time. The lost session's requests are not sent again on a fresh session. A new session
is established when a caller makes a new request that needs one, which is a new
interaction with its own record, and the interoperability revision's instruction to open a
new session is satisfied by that — it asks for a new `InitializeRequest`, not for the lost
request to be sent a second time.

### `mcp.outbound.transport-unavailable`

An observer sees `unavailable`, naming the configured endpoint and the transport-level
reason: the name did not resolve, the connection was refused, the TLS handshake failed,
or nothing answered within the connect bound. The act reported is nobody's — and that is
the whole of what `unavailable` claims. Not one of those four is something an MCP server
did: a name resolves in a resolver, a refusal and an alert come from whatever holds the
address, and a bound that expires is the absence of an answer rather than an answer. The
endpoint is a server's address, which is what makes `peer` the tempting answer and the
wrong one — the address is not the server, and nothing of the server was heard. No session
was involved, so none is reported lost, and no request was ever framed, so no
outcome is uncertain — this is the one connection failure that is *not* ambiguous about a
provider effect. It is also not a version fact and not a capability fact: an endpoint
that cannot be reached says nothing about what it would have supported.

### `mcp.outbound.answer-never-arrived`

An observer sees `outcome_unknown`, naming the request that was framed and sent and the
connection that died before any answer to it was seen. This is the half of loss that never
involved a stream: section 6 says the server may answer with one JSON object rather than
opening one, and that this client supports both, so an answer can be lost with nothing
ever having been opened to lose. The act reported is nobody's: the endpoint did take the
request, which is why this is not `mcp.outbound.transport-unavailable`, but what it did
with it was never seen. At this client it is indistinguishable from an answer that was
computed, applied at the server and delivered into a broken socket, which is exactly why
it is uncertainty rather than failure. It is not `unavailable`, because the endpoint was
reachable enough to take the request, and it is not `session_lost`, because nothing
repudiated a session. It holds on either revision.

## 11. Shutdown

The interoperability revision defines no shutdown messages: shutdown is signalled by the
transport, and for HTTP that is closing the associated connections
(`mcp-2025-11-25-basic-lifecycle.mdx:224-246`). A client that no longer needs a session
**SHOULD** send an HTTP DELETE carrying the session id, and the server **MAY** answer
`405 Method Not Allowed` to say it does not allow that
(`mcp-2025-11-25-basic-transports.mdx:194-222`). Under the primary revision there is
nothing to terminate beyond the streams themselves, and a server that only speaks it
answers `405` to a DELETE
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:670-678`).

### `mcp.outbound.shutdown-completed`

An observer sees every stream this client opened closed, each in-flight request reported
under its own outcome — `mcp.outbound.stream-ended-without-an-answer` or
`mcp.outbound.answer-never-arrived`, both carrying `outcome_unknown`, where an answer was
never seen — and, for a session-bearing binding, one DELETE attempt whose `405` is a
permitted answer rather than an error. Shutdown invents no results: it does not report a
request as successful because the process is ending, and it does not report one as failed
when its effect was not observed. After it, the binding's configuration is unchanged and
reusable; `story:mcp-outbound-auth-lifecycle` owns what persists of the credential.

## Why no outcome above sends the request a second time

The pinned specification does permit it, in two places, and both are declined here rather
than left unmentioned. `mcp-2026-07-28-basic-versioning.mdx:69-71`: "The client
**SHOULD** select a mutually supported version from the `supported` list and retry the
request, or surface an error to the user if no compatible version exists." And
`mcp-2026-07-28-basic-transports-streamable-http.mdx:650-668` tells a client that reads a
recognised modern error to "retry using the advertised `supported` versions or correct the
request, rather than falling back."

Both are `SHOULD`, both offer the alternative in their own words, and this repository
already took that alternative for every other family it speaks:
`contracts/service/compatibility.md` section 2 — "the client does not try a second
route/version/credential or resend the request" — and, for an uncertain mutation, "A new
mutation client that has sent an invocation and lacks a valid definitive response reports
outcome uncertainty and never retries automatically, even if an HTTP failure resembles a
version problem."

So the second half of the upstream `SHOULD` is what this document selects: surface the
error to the user, naming what the server reported. An operator who wants a different
version changes the binding. The cost is a round trip a user has to authorise; the thing
bought is that no unobserved effect is ever repeated by this client on its own, which is
`epic:mcp-contracts` acceptance 5 — "A repeated MCP request cannot automatically
duplicate an uncertain business effect."

This section is the only place in this document where those words appear, and the case
`no_state_is_answered_by_sending_the_request_again` enforces that.

## Outbound stdio is held, not specified here

`decision-blocker:mcp-outbound-stdio-process-ownership` is open: nobody has decided
whether Connectors spawns an MCP server as a child process, and the model carries the
same edge unread as `UNMAPPED: McpServerBinding -> supervised OS process`. The selection
matrix dispositions `transport:stdio` outbound as `deferred` for exactly that reason.

This document therefore specifies the Streamable HTTP profile only. Nothing here states a
framing, a cancellation signal, a shutdown sequence or a process lifecycle for outbound
stdio, and no outcome above may be read as covering it. A specification of that transport
answers the blocker's question by existing, so it waits for the answer.

The inbound direction is not held by that blocker and is not this document's:
`story:mcp-inbound-local-binding` owns it.

## What this document does not cover

- **Authentication of the connection.** Which credential the transport acquires, how it
  persists across a CLI restart, and what expiry, refresh, repair and revocation do is
  `story:mcp-outbound-auth-lifecycle`, in `auth.md` beside this file. The pinned
  revisions make credential acquisition a property of the transport, so that document
  reads this one; no outcome here names a credential state.
- **What an invocation returns.** Result content, structured output, output bounds, the
  preservation of a provider error as distinct from a protocol error, and what a caller
  does with `outcome_unknown` are `story:mcp-outbound-invocation-results`.
- **Anything inbound.** No `$BIN server` behaviour is stated here.
- **Any relation the model could not read.** `McpOutboundSession → McpServerBinding`,
  `McpServerBinding → McpCapabilitySnapshot`, `McpServerBinding →
  connectors.auth_bindings.Connection`, `McpServerBinding →
  connectors.declarations.ServiceConfiguration` and `McpServerBinding → credential
  custody` are all `UNMAPPED:` in `story:mcp-domain-model`. This document specifies what a
  *live* connection does; no sentence of it implies a durable binding record, and none
  resolves a marker.

## Limits

These are authored contract statements. They are not evidence that any MCP server, or any
future Connectors client, implements any of them. No MCP connection has been made from
this repository, no executable has been added, and no fixture server exists; a document
that validates proves a document validates, not network interoperability, a provider
effect, a persistent credential or cloud authentication.

The archived bytes, not the network, are the authority for every citation above, and line
numbers are lines of the uncompressed archived file — the only kind of line number that
cannot drift, because a digest fixes it. Citations into files this repository can edit
name a section or quote a phrase instead, which is the rule
[the adapter design](../../../design.md#citing-this-document-use-a-heading-not-a-line-number)
records after this epic spent three findings on drifted line numbers.
