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
`n/a` in that column: it is an observation, not an error.

## The outcome register

Every lifecycle state `story:mcp-outbound-connection-lifecycle` names appears here with a
named observable outcome and the scenario that decides it. `crates/connectors-build/tests/mcp_outbound_connection_lifecycle.rs`
derives the required states from the story's own text, so a state missing from this table
is a defect in this table rather than a silent gap.

| State | Observable outcome | `ErrorCode` | Scenario |
|---|---|---|---|
| `state:discovery` | `mcp.outbound.server-described` | `n/a` | [`discovery-records-without-selecting.yaml`](scenarios/discovery-records-without-selecting.yaml) |
| `state:explicit-selection` | `mcp.outbound.server-selected` | `n/a` | [`explicit-selection-of-a-configured-binding.yaml`](scenarios/explicit-selection-of-a-configured-binding.yaml) |
| `state:explicit-selection` | `mcp.outbound.selection-refused-unconfigured-endpoint` | `route_refused` | [`discovered-endpoint-does-not-become-a-selection.yaml`](scenarios/discovered-endpoint-does-not-become-a-selection.yaml) |
| `state:initialize-negotiation` | `mcp.outbound.legacy-initialize-negotiated` | `n/a` | [`legacy-initialize-negotiates-version-and-capabilities.yaml`](scenarios/legacy-initialize-negotiates-version-and-capabilities.yaml) |
| `state:initialize-negotiation` | `mcp.outbound.legacy-initialize-answered-an-unconfigured-revision` | `unsupported` | [`legacy-initialize-answered-an-unconfigured-revision.yaml`](scenarios/legacy-initialize-answered-an-unconfigured-revision.yaml) |
| `state:version-mismatch` | `mcp.outbound.version-refused` | `unsupported` | [`version-mismatch-names-what-the-server-reported.yaml`](scenarios/version-mismatch-names-what-the-server-reported.yaml) |
| `state:version-mismatch` | `mcp.outbound.era-ambiguous-answer-refused` | `upstream_protocol` | [`era-ambiguous-400-does-not-fall-back.yaml`](scenarios/era-ambiguous-400-does-not-fall-back.yaml) |
| `state:capability-mismatch` | `mcp.outbound.required-client-capability-refused` | `unsupported` | [`required-client-capability-is-refused-not-approximated.yaml`](scenarios/required-client-capability-is-refused-not-approximated.yaml) |
| `state:capability-mismatch` | `mcp.outbound.advertised-capability-recorded-not-projected` | `n/a` | [`unknown-advertised-capability-is-not-support.yaml`](scenarios/unknown-advertised-capability-is-not-support.yaml) |
| `state:framing` | `mcp.outbound.request-framed` | `n/a` | [`one-request-is-one-post-with-agreeing-headers.yaml`](scenarios/one-request-is-one-post-with-agreeing-headers.yaml) |
| `state:framing` | `mcp.outbound.framing-refused` | `upstream_protocol` | [`header-body-mismatch-is-not-the-callers-input.yaml`](scenarios/header-body-mismatch-is-not-the-callers-input.yaml) |
| `state:streaming` | `mcp.outbound.stream-opened` | `n/a` | [`streamed-answer-carries-only-its-own-request.yaml`](scenarios/streamed-answer-carries-only-its-own-request.yaml) |
| `state:streaming` | `mcp.outbound.stream-ended-without-an-answer` | `outcome_unknown` | [`stream-ends-without-a-final-response.yaml`](scenarios/stream-ends-without-a-final-response.yaml) |
| `state:progress` | `mcp.outbound.progress-observed` | `n/a` | [`progress-is-display-only-and-bounded.yaml`](scenarios/progress-is-display-only-and-bounded.yaml) |
| `state:cancellation` | `mcp.outbound.cancelled-by-closing-the-stream` | `n/a` | [`cancellation-closes-the-response-stream.yaml`](scenarios/cancellation-closes-the-response-stream.yaml) |
| `state:session-loss` | `mcp.outbound.session-lost` | `session_lost` | [`legacy-session-loss-is-not-a-new-session.yaml`](scenarios/legacy-session-loss-is-not-a-new-session.yaml) |
| `state:transport-unreachable` | `mcp.outbound.transport-unavailable` | `unavailable` | [`unreachable-endpoint-is-unavailable-not-loss.yaml`](scenarios/unreachable-endpoint-is-unavailable-not-loss.yaml) |
| `state:shutdown` | `mcp.outbound.shutdown-completed` | `n/a` | [`shutdown-closes-every-stream-it-opened.yaml`](scenarios/shutdown-closes-every-stream-it-opened.yaml) |

The scenario files are not `ess-scenario/1` traces. `connectors_mcp` declares no command
and every entity in it carries a single-state lifecycle, because `story:mcp-domain-model`
models nouns and declares no behaviour; an `ess-scenario/1` file names commands and
outcomes of a domain that has them. These carry `type: mcp-outbound-lifecycle/1` instead,
and the case above is what holds them to this register.

## 1. Discovery — what a server reports, and what that changes

`server/discover` is the modern revision's description request. Servers **MUST**
implement it and clients **MAY** call it before anything else
(`mcp-2026-07-28-basic-versioning.mdx:73-78`); its result carries the server's supported
protocol versions, the keys of its advertised capabilities, an optional self-reported
identity and optional caching hints (`mcp-2026-07-28-server-discover.mdx:33-60`).

That result is recorded as the model's `McpCapabilitySnapshot` value, whose
`supported_versions` is a `List<String>` and whose `server_capabilities` is a
`List<String>` — because a server may name versions and capability keys neither pinned
revision covers (`mcp-2026-07-28-server-discover.mdx:94-95`,
`mcp-2026-07-28-schema.ts:789`). A reported string that matches no row of the selection
matrix is still recorded exactly as reported.

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
names the endpoint, the transport binding and the revision — or the ordered pair of
revisions — the binding may speak. `epic:mcp-contracts` requires "discovery and explicit
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
offered to it, and the configured binding is left exactly as it was. An operator
reconfiguration is a new interaction, not a continuation of this one.

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
Connectors disconnects for a stricter reason as well — the answered revision was not
selected by an operator, and a server's answer is not a selection. No second handshake is
opened, no second revision is offered, and no request is carried out under the revision
the server named.

## 4. Protocol version mismatch

Under the primary revision, a server that does not implement the requested version
**MUST** answer `UnsupportedProtocolVersionError` (`-32022`,
`mcp-2026-07-28-schema.ts:450`) listing the versions it does support
(`mcp-2026-07-28-basic-versioning.mdx:48-52`), and over HTTP that answer is a
`400 Bad Request` (`mcp-2026-07-28-basic-transports-streamable-http.mdx:263-269`). The
error body carries both halves: `supported`, a list of strings, and `requested`
(`mcp-2026-07-28-basic-versioning.mdx:54-67`).

**A mismatch refuses; it does not degrade.** The rule is
`contracts/service/compatibility.md` section 2: "Unknown/missing routes or incompatible
descriptors stop selection", and "A raw HTTP 404 or malformed response is not renamed into
proof that a particular version is unsupported." The version the binding declares is the
one an operator configured; what a server reports is recorded and shown, never selected.
The rest of that sentence, and why this document takes it, is
[below](#why-no-outcome-above-sends-the-request-a-second-time).

### `mcp.outbound.version-refused`

An observer sees `unsupported`, carrying the endpoint, the version the binding declared
and the exact `supported` list the server reported — including strings no pinned revision
covers, such as `2025-06-18` or `2024-11-05`, which are shown as reported and are not
offered as choices the client will take on its own. The request that met the mismatch is
not carried out and no other version is declared on its behalf. An operator who wants one
of the reported versions changes the binding's configuration, and that is a new
interaction with its own record.

### `mcp.outbound.era-ambiguous-answer-refused`

An observer sees `upstream_protocol`, naming the HTTP status and stating that the body
carried no recognised modern MCP error. This is the `400` the primary revision tells a
dual-era client to inspect before deciding what the server is
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:650-668`,
`mcp-2026-07-28-basic-versioning.mdx:126-146`). Connectors reads the body for the same
reason and stops at the same point: a recognised modern error is the refusal above, and
anything else is this outcome. A binding configured for one revision does not begin
speaking another because an answer was unreadable, and the deprecated HTTP+SSE transport
is never reached — the selection matrix refuses `transport:http-sse-transport` outbound,
and the fallback that would have landed there
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:710-737`) is declined at its first
step. Era remains a property of the server rather than of a request
(`mcp-2026-07-28-basic-versioning.mdx:148-152`); `determined_era` is recorded when a probe
settles it, and an operator's configuration still decides what is spoken.

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

### `mcp.outbound.required-client-capability-refused`

An observer sees `unsupported`, carrying the endpoint, the method that was refused and
the `requiredCapabilities` list the server named. Nothing is auto-answered on a user's
behalf: no elicitation form is filled in, no default is supplied, no model is sampled,
and no filesystem root is invented. The capability is not declared on a second attempt in
order to get past the refusal, because whether Connectors declares it at all is the
selection matrix's decision and not this document's. The caller learns which capability
the server wanted, which is the actionable half.

### `mcp.outbound.advertised-capability-recorded-not-projected`

An observer sees the advertised key listed exactly as the server sent it, marked as
advertised-and-not-consumed, and no Connectors capability appears because of it. This
holds for a refused key and for an unknown key alike: the snapshot's
`server_capabilities` is a `List<String>` precisely so an unknown key has somewhere to go
(`mcp-2026-07-28-server-discover.mdx:94-95`). It is not an error, it is not support, and
it never becomes an admission decision, an authority, or a reason to send a request that
would depend on it.

## 6. Framing

Every JSON-RPC message the client sends is its own HTTP POST to the MCP endpoint; the
client **MUST** offer both `application/json` and `text/event-stream` in `Accept`, **MUST**
carry the request-metadata headers, **MUST** put exactly one JSON-RPC request or
notification in the body and **MUST NOT** send JSON-RPC responses
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:70-91`). The server answers a
request with either a single JSON object or an SSE stream and the client supports both.

Three headers are required and are mirrors of body fields: `MCP-Protocol-Version`, whose
value **MUST** equal the `io.modelcontextprotocol/protocolVersion` in the body's `_meta`,
`Mcp-Method` and `Mcp-Name` (`mcp-2026-07-28-basic-transports-streamable-http.mdx:250-261`
and `:286-293`). The body's `_meta` carries the protocol version and the client
capabilities as required fields on every request
(`mcp-2026-07-28-basic-index.mdx:373-382`). A server that finds header and body
disagreeing **MUST** answer `400 Bad Request` with `HeaderMismatch` (`-32020`,
`mcp-2026-07-28-schema.ts:434`, `mcp-2026-07-28-basic-transports-streamable-http.mdx:580-607`).

### `mcp.outbound.request-framed`

An observer sees one POST per JSON-RPC message, carrying one request or one notification,
with `Accept` offering both media types and with the three mirrored headers agreeing
byte-for-byte with the body they mirror. A notification is answered `202 Accepted` with
no body. The protocol version in the header and in `_meta` is the binding's configured
revision, identical on every request of that binding, and no request of this repository
ever carries a JSON-RPC response as its body.

### `mcp.outbound.framing-refused`

An observer sees `upstream_protocol`, carrying the endpoint, the HTTP status and the
JSON-RPC code the peer returned. This is the outcome for `-32020`, for a body that is not
readable as JSON-RPC, and for an answer whose media type is neither of the two the
transport defines. It is deliberately **not** `invalid_input`: the caller supplied no
header, so blaming the caller's input for a framing disagreement would be a false
attribution, and the caller's request is not carried out. What a truthful result looks
like once an answer *is* readable belongs to `story:mcp-outbound-invocation-results`.

## 7. Streaming

When the server answers with `Content-Type: text/event-stream`, the stream is scoped to
the one request that opened it. The server **MAY** send notifications related to that
request before the final response and **MUST NOT** send independent JSON-RPC requests on
it — server-initiated interactions are embedded in an `InputRequiredResult` instead — and
the final response **SHOULD** terminate the stream
(`mcp-2026-07-28-basic-transports-streamable-http.mdx:107-125`). Long-lived notification
streams are a separate request, `subscriptions/listen`, whose filter names the
notification types the client opted into and whose first message is an acknowledgment
(`mcp-2026-07-28-basic-patterns-subscriptions.mdx:12-16` and `:52-56`).

**There is no resumption.** `mcp-2026-07-28-basic-transports-streamable-http.mdx:157`
states that resumable SSE streams via `Last-Event-ID` are not supported, and `:670-678`
records that the earlier shape — server-assigned session ids, a standalone GET stream,
server-sent requests and resumable streams — is not part of this revision. A stream that
ends has ended; nothing about it is resumed from a cursor.

### `mcp.outbound.stream-opened`

An observer sees a stream bound to exactly one request id, carrying only notifications
that relate to it and terminating with that request's response. A JSON-RPC *request*
arriving on the stream is refused as `upstream_protocol` under
`mcp.outbound.framing-refused` rather than answered, because this revision defines no
channel for it. A `subscriptions/listen` stream carries only the notification types its
filter named, each tagged with its subscription id, and its acknowledgment arrives before
any notification of that subscription.

### `mcp.outbound.stream-ended-without-an-answer`

An observer sees `outcome_unknown`, naming the request whose stream closed before its
final response arrived, and the last progress value observed if there was one. The
specification calls this an unexpected disconnect and distinguishes it from the graceful
closure that carries a response (`mcp-2026-07-28-basic-patterns-subscriptions.mdx:128-135`
and `:155-157`). Connectors reports the uncertainty as uncertainty: it does not report
success, does not report failure, and does not send the request again, because a request
whose answer was never seen may already have had its effect.
`story:mcp-outbound-invocation-results` owns what a caller does with an unknown outcome;
this document owns the observation that produced it.

## 8. Progress

A client opts into progress by putting a `progressToken` in a request's `_meta`; tokens
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

### `mcp.outbound.progress-observed`

An observer sees progress lines for the request that asked for them, in non-decreasing
order, each showing the server's `message` as server-supplied text. A notification
carrying a token that belongs to no active request of this client is ignored and
recorded, never attributed to another request. Progress is display: it is never a result,
never a partial result the caller may act on, and never authority. When progress stops
arriving the request's own timeout still expires, and the bounded outcome is the one
under `state:cancellation`, not an indefinite wait.

## 9. Cancellation

On Streamable HTTP, cancellation is the transport act: closing the SSE response stream
**MUST** be treated by the server as cancellation of that request, the disconnect is
unambiguous because each request has its own stream, and no `notifications/cancelled`
message is expected (`mcp-2026-07-28-basic-transports-streamable-http.mdx:233-240`,
`mcp-2026-07-28-basic-patterns-cancellation.mdx:39-41`). A timeout is the same act: when
the answer has not arrived within the request's timeout the client closes the stream,
which constitutes cancellation
(`mcp-2026-07-28-basic-patterns-cancellation.mdx:45-64`). This document states the signal
for the selected transport only; the other outbound transport family is held by
`decision-blocker:mcp-outbound-stdio-process-ownership` and is not specified here.

A late answer may still arrive for a cancelled request; the client **SHOULD** ignore it
(`mcp-2026-07-28-basic-patterns-cancellation.mdx:81-82`).

### `mcp.outbound.cancelled-by-closing-the-stream`

An observer sees the request reported as cancelled by this client, with the reason —
caller cancellation or timeout expiry — and with the fact that the effect at the server
was not observed. A cancelled mutation is `outcome_unknown` to its caller for the same
reason a truncated stream is: cancellation stops the client's waiting, not the server's
work. An answer that arrives after cancellation is recorded as late and does not become
the request's result, and a subscription is ended the same way, by closing its stream
(`mcp-2026-07-28-basic-patterns-subscriptions.mdx:118-126`).

## 10. Session loss, and the failure it is not

The story requires these two to stay apart, and the shared model already has two values
for them: `connectors.service_wire.ErrorCode` declares `unavailable` and `session_lost`
as separate variants.

A **session** exists only where the binding's selected revision is the interoperability
one. There the server **MAY** assign an `MCP-Session-Id` at initialization, the client
**MUST** echo it, the server **MAY** terminate it at any time and then answers `404` to
requests carrying it (`mcp-2025-11-25-basic-transports.mdx:194-222`). Under the primary
revision there is no session to lose: an open connection "is not a conversation or
session" (`mcp-2026-07-28-basic-index.mdx:204-209`), so a dropped connection there is
either `mcp.outbound.transport-unavailable` or, for a request already in flight,
`mcp.outbound.stream-ended-without-an-answer`.

### `mcp.outbound.session-lost`

An observer sees `session_lost`, naming the endpoint and the session the server
repudiated, and — separately — `outcome_unknown` for any request that was in flight when
the `404` arrived. The two facts are reported as two facts: the session is gone, and what
happened to the in-flight request was not observed. The lost session's requests are not
sent again on a fresh session. A new session is established when a caller makes a new
request that needs one, which is a new interaction with its own record, and the
interoperability revision's instruction to open a new session is satisfied by that — it
asks for a new `InitializeRequest`, not for the lost request to be sent a second time.

### `mcp.outbound.transport-unavailable`

An observer sees `unavailable`, naming the configured endpoint and the transport-level
reason: the name did not resolve, the connection was refused, the TLS handshake failed,
or nothing answered within the connect bound. No session was involved, so none is
reported lost, and no request was ever framed, so no outcome is uncertain — this is the
one connection failure that is *not* ambiguous about a provider effect. It is also not a
version fact and not a capability fact: an endpoint that cannot be reached says nothing
about what it would have supported.

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
under its own outcome — `outcome_unknown` where an answer was never seen — and, for a
session-bearing binding, one DELETE attempt whose `405` is recorded as a permitted answer
rather than an error. Shutdown invents no results: it does not report a request as
successful because the process is ending, and it does not report one as failed when its
effect was not observed. After it, the binding's configuration is unchanged and
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
