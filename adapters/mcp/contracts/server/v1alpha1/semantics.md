# MCP inbound local binding — `server/v1alpha1`

**Status:** an authored contract, not an implementation. No MCP message has been spoken
by this repository, no executable is added here, and no `server` command exists in the
generated CLI binding yet — `apps/connectors/spec/cli.yaml` carries no such command path
and `apps/connectors/spec/compatibility.json` records `"mcp": "deferred"`. This document
says what the selected inbound binding *does*; the command surface that exposes it is
`story:mcp-cli-journey-discovery-contract`.

Owner: `story:mcp-inbound-local-binding`, decomposing `epic:mcp-contracts`. It reads three
landed inputs and adds nothing to them: the
[pinned specification](../../protocol/v1alpha1/evidence/20260912/specification-sources.md)
and its 54 archived files, the authored native model at
[`adapters/mcp/spec/ess/`](../../../spec/ess/system.yaml), and the
[protocol selection matrix](../../protocol/v1alpha1/selection.md), which dispositions
every revision, transport and capability. **This document specifies what the selected
things do. It selects nothing**, promotes no `deferred` row and answers no `UNMAPPED:`
marker.

Citations into `<file>:<line>` are lines of the *uncompressed* archived bytes under
[`evidence/20260912/vendor/`](../../protocol/v1alpha1/evidence/20260912/), whose line
numbers a digest pins. Citations into this repository's own documents are by heading or
quoted phrase, because a line number into a living file drifts — the rule
[the adapter design states](../../../design.md#citing-this-document-use-a-heading-not-a-line-number).

## 1. What this binding is

| | |
|---|---|
| Direction | inbound — Connectors is the MCP **server**, a local MCP client is the caller |
| Transport | `transport:stdio`, the one inbound transport the selection matrix disposes `supported` |
| Placement | local, single-owner. The cloud placement is not this document's: `story:mcp-inbound-cloud-profile` |
| Revisions | `2026-07-28` (primary) and `2025-11-25` (interoperability), both disposed `supported` inbound by the matrix |
| Era | dual-era, by `mcp-2026-07-28-basic-versioning.mdx:174-180`: a request carrying modern per-request `_meta` is served statelessly, an `initialize` request selects legacy semantics scoped to the stdio process |

**The client launches the process; Connectors is the child.** On this transport "the
client launches the MCP server as a subprocess"
(`mcp-2026-07-28-basic-transports-stdio.mdx:7`), so no provider binary is executed by
this repository and `decision-blocker:mcp-outbound-stdio-process-ownership` — which asks
whether Connectors spawns an MCP server — does not reach this direction. That blocker
says so in its own text: "The **inbound** stdio direction is also not behind it: there
the MCP client spawns `$BIN server`, so Connectors is the child rather than the parent,
and no provider binary is executed by this repository."

**One principal, admitted before any capability is named.** The binding admits exactly
one: the configured owner of the local CLI binding.
[`contracts/cli/v1alpha1/semantics.md` §1](../../../../../contracts/cli/v1alpha1/semantics.md)
states it — "The first local binding admits the current effective Linux UID as the
configured owner, verified again at a protected Unix-domain socket using kernel peer
credentials. Missing/mismatched owner policy refuses before protected entry or provider
work" — and `crates/connectors-host/src/local/owner/transport.rs`, in `connect_socket`,
refuses a path that is not a socket, whose `uid()` is not the process's own, or whose
mode carries any group or other bits. Nothing about that admission is MCP's: the caller
presents no credential over stdio at all, because this transport carries none to present
(§4, `message framing`).

**No credential is acquired over this binding.** The pinned primary revision makes
credential acquisition a property of the transport, and says stdio is not the
authorization framework's business: implementations "using STDIO transport **SHOULD NOT**
follow this specification, and instead retrieve credentials from the environment"
(`mcp-2026-07-28-basic-index.mdx:223-226`). So this binding requires no network service,
no hosted identity and no cloud control plane — it has none available and asks for none.
The model records the same shape: `McpInboundCallerCredential.audience_canonical_uri` is
`Optional` precisely because a subprocess has no canonical URI
([`domains/state.yaml`](../../../spec/ess/domains/state.yaml)).

## 2. The session record this binding supervises

This binding reuses `connectors.sessions.Session` — `Offered`, `Establishing`, `Ready`,
`Closing`, `Closed`, `Lost` — and adds no state, no transition and no vocabulary of its
own. The record is **the host's supervision of one stdio connection**, and it is not an
MCP session:

- The primary revision declares MCP "a **stateless protocol**" whose server "processes
  each request independently" (`mcp-2026-07-28-basic-index.mdx:184-187`), forbids a server
  relying on prior requests over the same connection (`:191-193`), and says in as many
  words that "an open connection, such as a STDIO process, is not a conversation or
  session" (`:204-209`). The shared sessions contract says the converse from its own side:
  a physical socket is never the semantic session identity.
- So no admission, capability or protocol version is inferred from the session record.
  Every modern request carries its own version and capabilities in `_meta`, required on
  every request (`mcp-2026-07-28-basic-index.mdx:365-382`), and the binding reads them
  there or refuses (§3.6).
- A `connectors_mcp.state.McpInboundSession` record carries a `LegacySessionPhase` and is
  meaningful only where the legacy revision is in force, which is what its own model
  comment says.

Establishment, for completeness, is not one of the six behaviours: the process starts,
the owner is admitted, and the supervisor moves `accept` (`Offered` → `Establishing`)
then `ready` (`Establishing` → `Ready`). The session's `connectors.sessions.Binding`
coordinates in this placement are the configured instance (`instance_ref`), the single
configured owner's connection coordinate (`connection_ref`), the admitted configuration
revision (`admitted_revision`) and the owner authority (`authority_ref`).

**Data rides a lease, and the lease is short.** Reusing `connectors.sessions.Session`
inherits its timing obligations, not only its states.
[`contracts/sessions/v1alpha1/semantics.md` §4.1](../../../../../contracts/sessions/v1alpha1/semantics.md)
makes a live data lease "mandatory for every admitted data path" and caps its effective
deadline at **2,000 ms after authoritative issuance**, "including delivery delay, clock
uncertainty, scheduling delay and already-buffered output" — and it requires a binding
that cannot enforce that for every data path to **refuse session admission**. So every
trace under [`scenarios/`](scenarios/) issues a lease of at most that, and an inbound
request that outlives one stays `Ready` only through the supervisor's own
`RenewDataLease` (`renew`, `Ready` → `Ready`), taken "under the same serialized authority
check as revocation" and never past an earlier deadline. No peer-visible MCP message is a
lease input; none of them renews anything (§3.3). If the supervisor does not renew before
the effective deadline the lease expires, and expiry is a local terminal fact: the model
fixes it to `lease_expired`, there is no grace past the deadline, the outcome of whatever
was in flight is unknown, and a renewal arriving afterwards cannot reopen the session. A
terminal fact starts the two other clocks §4.1 owns — data stops within **2,000 ms** of
it and local teardown and accounting complete within **5,000 ms** — and where the live
lease's own deadline is earlier than the first of those, the earlier one dominates.

**`connection_ref` here is the supervisor's own coordinate, not an assignment.** Which
connection's provider credential carries out an inbound invocation is
`decision-blocker:mcp-caller-connection-assignment`, open. In this placement there is
nothing to assign, and the blocker says why: "With one principal there is nothing to
assign." Every trace under [`scenarios/`](scenarios/) therefore names exactly one
`instance_ref`, one `connection_ref` and one `authority_ref`, and a second value of any
of them is the moment this document stops.

Two markers the domain model carries, repeated here rather than resolved:

- **UNMAPPED: `McpInboundSession → McpCaller`** — whether a caller is an entity in this
  repository at all. No caller record is introduced here, and no caller-keyed field is
  added to anything.
- The absence of `McpSession → connectors.auth_bindings.Connection` is cited, not
  invented: `ess/domains/sessions.yaml` declines to join a session binding to its one
  Connection, "whose persistence model remains unselected … no stub edge is implied".
  This binding implies none either.

## 3. The six behaviours

Each row names what a caller can observe, the transition it turns on in the
`connectors.sessions.Session` vocabulary, and the compiled trace that carries it. Rows
repeat transitions, and that is a property of the vocabulary rather than a gap in the
enumeration: `permit_data` is the only transition `connectors.sessions.Session` offers for
"the session carries more data and stays `Ready`", which is what framing, progress,
cancellation and a per-request refusal all are, and `begin_close` is the only one for "the
host records a terminal and stops admitting data", which both a graceful close and the end
of a partial result are. What distinguishes rows is the observable outcome, and no two
rows share one.

**Where a behaviour has more than one observable route, each route is its own row.** A
behaviour whose answer differs by *where* the mismatch happens, by *how* the connection
ends, or by whether the request outlives the lease admitting it does not have one
transition, and collapsing it into one row picks a winner silently: a reader implementing
from the table would deny readiness for a per-request capability refusal, have nothing to
implement for a graceful shutdown, and renew no lease at all. Each route therefore carries
its own observable outcome, its own transition and its own trace, and the prose section
says which is which.

**Each trace records one admitted data decision, and that is arithmetic, not brevity.**
ESS instants are whole seconds and must strictly increase, a data lease may not outlive
2,000 ms from its issuance (§2), and an act admitted *on* the effective expiry is admitted
after the gate has stopped — so exactly one whole second falls strictly inside a lease
window. A trace therefore records the decision its outcome turns on, and a renewal where
the request outlives the lease; the messages either side of it are the same decision
again, and nothing in the enumeration rests on them.

| Behaviour | Observable outcome | Session transition | Scenario |
|---|---|---|---|
| message framing | `framing-refusal-keeps-the-session-ready` | `permit_data` (`Ready` → `Ready`) | [`framing-refusal-keeps-the-session-ready.yaml`](scenarios/framing-refusal-keeps-the-session-ready.yaml) |
| streaming | `partial-result-never-reported-complete` | `begin_close` (`Ready` → `Closing`) | [`partial-result-never-reported-complete.yaml`](scenarios/partial-result-never-reported-complete.yaml) |
| progress | `progress-stops-at-the-result` | `permit_data` (`Ready` → `Ready`) | [`progress-stops-at-the-result.yaml`](scenarios/progress-stops-at-the-result.yaml) |
| progress (lease renewal) | `renewed-lease-keeps-a-long-request-ready` | `renew` (`Ready` → `Ready`) | [`renewed-lease-keeps-a-long-request-ready.yaml`](scenarios/renewed-lease-keeps-a-long-request-ready.yaml) |
| cancellation | `cancelled-request-answers-nothing` | `permit_data` (`Ready` → `Ready`) | [`cancelled-request-answers-nothing.yaml`](scenarios/cancelled-request-answers-nothing.yaml) |
| connection and session loss (graceful close) | `graceful-stdin-close-ends-the-session` | `begin_close` (`Ready` → `Closing`) | [`graceful-stdin-close-ends-the-session.yaml`](scenarios/graceful-stdin-close-ends-the-session.yaml) |
| connection and session loss (transport drop) | `transport-drop-loses-the-session` | `continuity_lost` (`Ready` → `Lost`) | [`transport-drop-loses-the-session.yaml`](scenarios/transport-drop-loses-the-session.yaml) |
| version and capability mismatch (per-request refusal) | `capability-refusal-keeps-the-session-ready` | `permit_data` (`Ready` → `Ready`) | [`capability-refusal-keeps-the-session-ready.yaml`](scenarios/capability-refusal-keeps-the-session-ready.yaml) |
| version and capability mismatch (legacy initialize) | `unsupported-version-denies-readiness` | `deny_ready` (`Establishing` → `Closing`) | [`unsupported-version-denies-readiness.yaml`](scenarios/unsupported-version-denies-readiness.yaml) |

### 3.1 Message framing — `framing-refusal-keeps-the-session-ready`

The frame is one JSON-RPC message per line: messages "are delimited by newlines, and
**MUST NOT** contain embedded newlines" (`mcp-2026-07-28-basic-transports-stdio.mdx:13`,
and identically for the interoperability revision at
`mcp-2025-11-25-basic-transports.mdx:30`). The binding writes nothing to `stdout` that is
not a valid MCP message (`mcp-2026-07-28-basic-transports-stdio.mdx:18-19`); its own logs
go to `stderr`, which a client "**SHOULD NOT** assume … indicates error conditions"
(`mcp-2026-07-28-basic-transports-stdio.mdx:14-17`).

**An unparseable inbound line** — invalid UTF-8, or bytes that are not JSON — is answered
with a JSON-RPC parse error, code `-32700`. The code is the revision's own: it keeps "the
standard JSON-RPC 2.0 error codes (`-32700`, `-32600` to `-32603`) for general protocol
failures" (`mcp-2026-07-28-basic-index.mdx:111-112`), and its schema declares
`PARSE_ERROR = -32700` for exactly this condition — "invalid JSON was received by the
server … the server cannot parse the JSON text of a message"
(`mcp-2026-07-28-schema.ts:312`, `mcp-2026-07-28-schema.ts:319`).

**A line that parses as JSON but is not a request** — no `jsonrpc`, no `method`, an `id`
of a type the envelope does not allow — is a different refusal, and it is `-32600`, not
`-32700`. The schema draws that line itself: `INVALID_REQUEST = -32600` is "returned when
the message structure does not conform to the JSON-RPC 2.0 specification requirements for
a request (e.g., missing required fields like `jsonrpc` or `method`, or using invalid
types for these fields)" (`mcp-2026-07-28-schema.ts:313`, `mcp-2026-07-28-schema.ts:333`).
Parsed bytes are never a parse error.

Either way, **the error response carries no `id` when the `id` could not be read**, which
the revision permits in exactly that case: "Error responses **MUST** include the same ID
as the request they correspond to (except in error cases where the ID could not be read
due a malformed request)" (`mcp-2026-07-28-basic-index.mdx:103`). A line that parses as a
well-formed request but lacks a required `_meta` field is a third refusal, with a readable
`id`: `-32602`, by `mcp-2026-07-28-basic-index.mdx:380-382`.

**A truncated inbound line** — bytes arriving with no terminating newline — is never
executed. The binding holds the partial bytes and executes nothing until a newline
completes the frame; if the stream ends first, the partial buffer is discarded unparsed
and the connection has closed, which is §3.5 and not this behaviour.

**The session stays `Ready`.** The refusal is answered on the same channel and the
connection keeps carrying traffic: the supervisor's decision is `permit_data`
(`Ready` → `Ready`), not a denial. Closing the connection on a bad frame is refused as a
design, with a reason from the revision itself: a caller may interleave unrelated
requests on the same transport (`mcp-2026-07-28-basic-index.mdx:204-209`), so dropping the
connection would destroy in-flight work that has nothing to do with the offending line.

### 3.2 Streaming — `partial-result-never-reported-complete`

There is no per-request stream on this transport: "All messages share this single
channel; there are no per-request streams"
(`mcp-2026-07-28-basic-transports-stdio.mdx:41-42`). A long-lived result is a
`subscriptions/listen` request whose response "is just an open stream of notifications"
(`mcp-2026-07-28-basic-index.mdx:211-214`), correlated by
`io.modelcontextprotocol/subscriptionId`, which on stdio a client **MUST** use because
every subscription shares one channel
(`mcp-2026-07-28-basic-patterns-subscriptions.mdx:83-92`).

**A partially delivered outbound result is never reported complete.** `resultType:
"complete"` means the request completed and the result holds the final content
(`mcp-2026-07-28-basic-index.mdx:81-85`), so the binding emits it only for a result it has
in full. When it ends a partially delivered stream on its own initiative it uses the
revision's graceful-closure form instead — the empty `subscriptions/listen` response,
carrying the subscription id and no content
(`mcp-2026-07-28-basic-patterns-subscriptions.mdx:128-135`) — and the caller can tell the
two apart, because "a transport that closes without it indicates an unexpected
disconnect" (`:150-157`).

**The transition is `begin_close` (`Ready` → `Closing`).** Ending a partially delivered
result is a terminal decision about the connection, recorded once, with reason
`local_close` by `host`; the graceful response is emitted before teardown finishes, and
after the close is recorded no further result content is emitted — a later emission is
refused with `connectors.sessions.StateConflict`. A partial result ended by the *caller*
is §3.4, and one ended by a drop is §3.5; both leave the same guarantee, that nothing
partial is presented as complete.

### 3.3 Progress — `progress-stops-at-the-result` and `renewed-lease-keeps-a-long-request-ready`

Progress is opt-in and caller-keyed: a caller that wants it includes a `progressToken` in
`_meta` (`mcp-2026-07-28-basic-patterns-progress.mdx:13-18`), and a caller that sends none
receives none. The binding emits `notifications/progress` only against a token "provided
in an active request" and "associated with an in-progress operation" (`:60-62`), with a
`progress` value that increases on every notification (`:53-54`), and it is free to send
none at all (`:64-67`). Progress rides the same single channel as everything else, as one
of the three kinds of message the server writes
(`mcp-2026-07-28-basic-transports-stdio.mdx:44-48`).

**Progress stops at the result**: "Progress notifications **MUST** stop after completion"
(`mcp-2026-07-28-basic-patterns-progress.mdx:90`), so the response for a request is the
last message a caller sees for it. Rate limiting is a duty of both parties (`:88-89`).

**The transition is `permit_data` (`Ready` → `Ready`), and the notification itself is not
a `renew`.** A progress notification is outbound data on a session that is already
`Ready`; it is not a lease input and it renews nothing. The revision's licence to reset a
clock on progress belongs to the *sender of a request* — "Implementations **MAY** choose
to reset the timeout clock when receiving a progress notification … however,
implementations **SHOULD** always enforce a maximum timeout"
(`mcp-2026-07-28-basic-patterns-cancellation.mdx:60-64`) — which on this binding is the
caller, not this repository. Lease authority stays where the shared sessions contract puts
it, with the host's own authenticated decision, and a peer-visible notification is not
one.

**What keeps a long request alive is the supervisor's own renewal, and a request that is
not renewed dies rather than continuing.** Progress exists because a request can run
longer than a moment, and §2 says a data lease may not outlive 2,000 ms from issuance, so
a request that runs longer than one lease is the normal case rather than the exotic one.
The supervisor renews it — `RenewDataLease`, `renew` (`Ready` → `Ready`) — before the
effective deadline, under the same serialized authority check as revocation, never past an
earlier deadline, and never on the strength of a progress notification having been sent.
If it does not, the lease expires at its deadline: that expiry is itself the terminal fact,
recorded `lease_expired`, with no grace period after it and no way for a later renewal to
reopen the session. The caller then observes what §3.5 describes for any ending —
notifications stop, no result arrives, and whether the work took effect is unknown.

That renewal is a route of this behaviour and has its own row, its own outcome
`renewed-lease-keeps-a-long-request-ready`, and its own trace,
[`scenarios/renewed-lease-keeps-a-long-request-ready.yaml`](scenarios/renewed-lease-keeps-a-long-request-ready.yaml):
the supervisor renews at `12:00:03`, strictly before the `12:00:04` deadline of the lease
that admitted the request, and the response goes out under the renewed one. A reader
implementing only the `permit_data` row would implement no renewal, and every request
outliving one lease would die at its deadline — which is why the row exists rather than
the prose alone.

### 3.4 Cancellation — `cancelled-request-answers-nothing`

A caller cancels an in-flight request by sending `notifications/cancelled` naming the
request's id. On this transport that notification is the only signal there is: "Because
stdio is a single shared bidirectional channel, there is no per-request stream to close.
Servers **SHOULD** stop work on a cancelled request as soon as practical and **MUST NOT**
send any further messages for it" (`mcp-2026-07-28-basic-transports-stdio.mdx:78-83`), and
the pattern page states the same split between the two transports
(`mcp-2026-07-28-basic-patterns-cancellation.mdx:39-43`).

**The cancelled request is answered with nothing.** The binding stops processing, frees
what the request held, and sends no response for it
(`mcp-2026-07-28-basic-patterns-cancellation.mdx:73-76`). A notification naming an unknown
id, an already-completed request or one that cannot be cancelled is ignored (`:77-80`), as
is a malformed one (`:113-117`) — the race in which a cancellation arrives after the
response was already written is expected and is not an error (`:86-89`). In the other
direction the binding sends `notifications/cancelled` for exactly one purpose, tearing
down a `subscriptions/listen` stream, and for no other (`:11-14`).

**The transition is `permit_data` (`Ready` → `Ready`).** Cancelling a request is scoped to
that request: the session stays `Ready` and its other in-flight work is untouched. The
`cancelled` terminal reason in the `connectors.sessions` vocabulary is a *session*
terminal and is deliberately not what a per-request `notifications/cancelled` turns on;
reading one onto the other would end a caller's whole connection because it withdrew one
request.

### 3.5 Connection and session loss — `graceful-stdin-close-ends-the-session` and `transport-drop-loses-the-session`

Two endings, and the binding distinguishes them. They are two rows of the enumeration
because they end in two different states and a reader implementing one is not implementing
the other.

**Graceful — `graceful-stdin-close-ends-the-session`.** The caller closes the binding's
`stdin`; the revision makes that the shutdown signal — "Servers **SHOULD** exit promptly
when their standard input is closed or reads return end-of-file. This is the primary
graceful-shutdown signal and the only portable one"
(`mcp-2026-07-28-basic-transports-stdio.mdx:102-104`), the client having closed the stream
and waited before any forced termination (`:89-94`); the interoperability revision
specifies the same sequence (`mcp-2025-11-25-basic-lifecycle.mdx:232-241`). The supervisor
records one terminal by `begin_close` (`Ready` → `Closing`) with reason `remote_hangup`,
cuts data off by the earlier of the §4.1 ceiling and the live lease's own deadline, and
finishes at `Closed`.

**The peer's shutdown is recorded `unconfirmed`, and end-of-file is not evidence
otherwise.** Closing that stream is step one of the client's three — "1. Closing the input
stream to the child process (the server). 2. Waiting for the server to exit. 3. If the
server does not exit within a reasonable time, forcibly terminating the process"
(`mcp-2026-07-28-basic-transports-stdio.mdx:89-94`) — so at EOF the peer has not exited;
it is waiting on this process with the escalation in hand. `Closed` therefore records
confirmed *local* release and nothing about the caller: the sessions contract says
`closed` "does not assert remote cooperation", and peer shutdown is recorded confirmed or
unconfirmed independently of the local cutoff. A binding that read EOF as the peer's exit
would be reporting an observation it never made.

**Lost — `transport-drop-loses-the-session`.** The connection drops with no graceful close — the caller's process dies, the
pipe breaks mid-write, the child is force-terminated. The outcome is unknown, which is
what `Lost` exists for: the supervisor moves `continuity_lost` (`Ready` → `Lost`) and the
state is terminal. Nothing afterwards revives it; a later data or renewal decision is
refused with `connectors.sessions.StateConflict` naming `Lost`.

**Nothing is resumed, and in-flight requests are simply lost.** The revision says it of
this transport: "Because the protocol is stateless, any in-flight requests are simply
lost and the client can retry them against the fresh process. Active
`subscriptions/listen` streams must also be re-established after restart"
(`mcp-2026-07-28-basic-transports-stdio.mdx:111-115`), and on stdio the server "holds no
subscription state across reconnections"
(`mcp-2026-07-28-basic-patterns-subscriptions.mdx:159-161`). **Whether a caller may safely
retry a lost mutating invocation is not settled here**: lost replies and mutation
uncertainty are `story:mcp-inbound-mutation-replay`.

### 3.6 Version and capability mismatch — `capability-refusal-keeps-the-session-ready` and `unsupported-version-denies-readiness`

There is no negotiation handshake in the primary revision: "Every request carries its
protocol version, and the server accepts or rejects each request independently"
(`mcp-2026-07-28-basic-versioning.mdx:12-13`).

**Per-request — `capability-refusal-keeps-the-session-ready`.** A modern request declaring
a version this binding does not speak is refused with
`UnsupportedProtocolVersionError`, code `-32022`, whose `data.supported` names the
versions the binding does support and `data.requested` echoes what arrived
(`mcp-2026-07-28-basic-versioning.mdx:48-67`). The supported list is exactly the two
revisions the matrix disposes `supported` inbound, `2026-07-28` and `2025-11-25`; every
other version string, `2024-11-05` and `2025-06-18` among them, is refused this way rather
than approximated. A request that needs a client capability the caller did not declare is
refused with `MissingRequiredClientCapabilityError`, `-32021`, listing the missing
capabilities in `data.requiredCapabilities` (`mcp-2026-07-28-basic-index.mdx:387-392`); a
request missing a required `_meta` field is `-32602` (`:380-382`). None of these closes the
connection: each is one request's refusal, and the session stays `Ready` under
`permit_data`, because version and capabilities are per-request facts on this revision.

**Legacy `initialize` — `unsupported-version-denies-readiness`.** This is the one route on
which a mismatch refuses the whole connection rather than one request, and it exists only
here: a legacy opening, on the interoperability revision. A dual-era server "selects its behavior from how the client
opens", and an `initialize` request "selects legacy semantics, scoped to the stdio
process" (`mcp-2026-07-28-basic-versioning.mdx:174-180`). The interoperability revision
then requires the binding to answer with the requested version if it supports it and
"otherwise … another protocol version it supports", after which "If the client does not
support the version in the server's response, it **SHOULD** disconnect"
(`mcp-2025-11-25-basic-lifecycle.mdx:172-177`). Establishment therefore does not complete:
the supervisor moves `deny_ready` (`Establishing` → `Closing`) with the gate decision
`unverified`, which the shared sessions model fixes to the terminal reason `error`
("unverified fixes error (rejected before readiness)"), and teardown follows. The binding
names the versions it supports in that answer rather than failing silently, which is what
the primary revision asks of any server answering an `initialize` it will not serve:
legacy clients "have no fall-forward mechanism, and this message may be the only
diagnostic they can surface to users" (`mcp-2026-07-28-basic-versioning.mdx:154-157`).

An advertised or requested capability that matches no row of the selection matrix is not
an error and not support: it is recorded as advertised and projected into nothing, which
is the matrix's own rule under its `Limits`. **Which** Connectors capabilities this
binding advertises at all is not this document's: that is
`story:mcp-inbound-capability-projection`, and it rests on the model's `UNMAPPED: which
Connectors operations are advertised, and under which admission`. Error mapping onto
`connectors.service_wire.ErrorCode` belongs to that story too.

## 4. What inbound stdio does not offer

The selected transport is narrower than the protocol, and every narrowing is stated here
rather than left out of §3. A caller that expects an HTTP-shaped mechanism gets the
refusal in the third column, never a silent approximation.

| Behaviour | What inbound stdio does not offer | What a caller observes instead | Source |
|---|---|---|---|
| message framing | No header layer at all — protocol version, per-request capabilities and caller identity travel inline in the JSON-RPC body, so there is no header for a caller to get wrong and `HeaderMismatch` (`-32020`) is unreachable on this binding | A body-level refusal instead of a transport-level one: `-32700` for an unparseable line, `-32602` for a line missing a required `_meta` field, and never a `-32020` | `mcp-2026-07-28-basic-transports-stdio.mdx:65-72`, `mcp-2026-07-28-basic-index.mdx:133-135` |
| streaming | No per-request stream, no SSE response body and no resumable event cursor; every subscription and every response shares one channel, so no stream can be opened, closed or resumed on its own | Correlation by `io.modelcontextprotocol/subscriptionId` on one channel, a graceful empty `subscriptions/listen` response when the binding ends a stream, and no resumption of a stream that dropped | `mcp-2026-07-28-basic-transports-stdio.mdx:41-42`, `mcp-2026-07-28-basic-patterns-subscriptions.mdx:128-135` |
| progress | No out-of-band progress channel and no reverse request channel: a server "**MUST NOT** write JSON-RPC _requests_" to its output stream, and this revision replaced server-initiated requests with the multi round-trip pattern, which the matrix refuses inbound (`capability:client/elicitation`) | Progress only as `notifications/progress` on the same channel, and only when the caller supplied a `progressToken`; an invocation that lacks what it needs is refused with its own error rather than answered with a request for more input | `mcp-2026-07-28-basic-transports-stdio.mdx:55-58`, `mcp-2026-07-28-basic-patterns-mrtr.mdx:10-13` |
| cancellation | No transport-level cancellation signal: closing a stream cancels a request on Streamable HTTP, and there is no per-request stream here to close, so a caller that simply stops reading cancels nothing | An explicit `notifications/cancelled` naming the request id is the only cancellation; a caller that instead drops its end is handled as connection loss, not as per-request cancellation | `mcp-2026-07-28-basic-patterns-cancellation.mdx:39-43`, `mcp-2026-07-28-basic-transports-stdio.mdx:78-83` |
| connection and session loss | No session identifier, no resumption and no replay — `MCP-Session-Id` and its `404` re-initialisation are Streamable HTTP mechanics that this transport has no counterpart for, and it keeps no state across a reconnection | A dropped connection ends every in-flight request; the caller relaunches the process and re-sends, including every `subscriptions/listen`, and no reply is redelivered | `mcp-2025-11-25-basic-transports.mdx:200-217`, `mcp-2026-07-28-basic-transports-stdio.mdx:111-115`, `mcp-2026-07-28-basic-patterns-subscriptions.mdx:159-161` |
| version and capability mismatch | No transport-level version carrier, so no `MCP-Protocol-Version` header to validate and no `400 Bad Request` path; and a legacy caller has no fall-forward once it has been answered | `-32022` with the supported list in the body for a modern request; for a legacy `initialize`, an answer naming a version the binding supports, on which a caller that cannot use it disconnects | `mcp-2026-07-28-basic-versioning.mdx:43-46`, `mcp-2026-07-28-basic-versioning.mdx:154-157` |

Three whole transports are outside this binding, and the matrix disposed each of them
already:

- `transport:streamable-http` inbound is `supported`, and it is **not this binding's**.
  This binding does not offer that placement, names no service it would need, and does
  not own it — the cloud placement belongs to `story:mcp-inbound-cloud-profile` and not
  here. This document specifies the local-client binding only and says nothing about how
  a caller reaches that placement, nor which connection such a caller may use — the
  second question is
  `decision-blocker:mcp-caller-connection-assignment`, which remains open and which this
  document does not answer.

  *(This bullet read `deferred`, held by that blocker, until the selection matrix's own
  correction re-dispositioned the row. The blocker's record never named a transport; a
  deferral resting on it withdrew a story the blocker does not claim. The disposition
  moved, so this statement of it moves with it.)*
- `transport:custom-transports` inbound is explicitly refused; this binding offers no
  transport of its own, and the local owner's protected Unix-domain socket is not
  offered as one either.
- `transport:http-sse-transport` inbound is explicitly refused and exposes no
  `2024-11-05` SSE endpoint to probe for.

This document specifies none of them and promotes none of them.

## 5. What is held, and by whom

| Held thing | Holder | What this document does instead |
|---|---|---|
| Which connection's provider credential runs an inbound invocation | `decision-blocker:mcp-caller-connection-assignment`, open | Specifies the binding for the one principal the local placement admits, and stops at the first second principal |
| Whether a caller is an entity here — `UNMAPPED: McpInboundSession → McpCaller` | `story:mcp-domain-model` | Introduces no caller record and no caller-keyed field |
| Joining a session to its one `Connection` | `ess/domains/sessions.yaml`, which declines it | Repeats the refusal; implies no stub edge |
| Which Connectors capabilities are advertised, and error mapping | `story:mcp-inbound-capability-projection` | Names none |
| Retry safety for a lost mutating invocation | `story:mcp-inbound-mutation-replay` | States that in-flight requests are lost and stops there |
| The `$BIN server` command surface, flags and output | `story:mcp-cli-journey-discovery-contract` | States the binding's semantics only |
| The cloud placement and its caller isolation | `story:mcp-inbound-cloud-profile` | Does not describe it |

## 6. Limits

**Nothing here is implemented, and nothing here has been spoken to an MCP client.** No
runtime, fixture server or executable is added by this document; a compiled scenario is
not an executed one. The pinned toolchain's own note says it: ESS "compiles obligations
but does NOT execute a sequential trace", so the traces under
[`scenarios/`](scenarios/) establish that this binding's decisions are expressible and
legal in the `connectors.sessions` vocabulary — not that any code takes them.

The archived bytes, not the network, are the authority for every protocol citation above.
No claim is made here about network interoperability, a provider effect, a persistent
credential or authentication of any kind beyond the local owner admission §1 cites.
