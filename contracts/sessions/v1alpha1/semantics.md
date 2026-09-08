# sessions/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** sessions (kernel). Consumed by [media](../../media/v1alpha1/semantics.md); establishment authority from [auth.capability](../../auth/capability/v1alpha1/semantics.md) (`session-authority`, `inbound-verifier`).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `sessions/v1alpha1` |
| Profiles | `outbound` (an operation establishes a session toward a configured target), `inbound_offer` (an adapter presents an offer that the host admits to one configured destination), `duplex_transport` (the binding that carries session control) |
| Vocabulary | session ref, offer, participant, request/response, event, stream, lease, terminal reason |

A session is a shared interaction lifetime with at least two participants; either may issue correlated requests and publish events; optional continuous data streams ride on it; lifecycle and control messages are modeled separately (`docs/design.md:389-396`). A physical socket is never the semantic session identity (`docs/design.md:398`). An operation may execute independently or establish a session (`docs/design.md:358`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `sip-dial`: `interaction_shape = session_establishment`, `protocol_driver = sip_v1`; response `call`, `session`, optional `channel`, `state = established`; `target` is an opaque connection-owned trunk alias, never a URI | `../connectors/providers/b10x.toml:741-760` | preserve as the `outbound` profile: a `mutation` operation whose result is a session ref |
| VoiceSession lifecycle `created → ready → closing → closed`, monotonic; transitions serialized by the endpoint loop; first terminal action wins, later ones are idempotent observations | `../connectors/contracts/voice-session/v0alpha1/README.md:16-20` | preserve, renamed to the design's states with an `offered` and `establishing` prefix (`docs/design.md:404`) |
| RTVBP: 60-second proof-bound single-redemption authority; serving endpoint redeems `(iss, jti)` before accepting bytes; authority expiry controls establishment; hangup, cancellation, revocation, lease, bounded drain control an established session; each call belongs to one generation; live sessions never migrate | `../connectors/docs/design/05-native-sip-and-rtvbp.md:297-318` | preserve |
| `dial_establishment_channel()` emits exactly one `SipDialEstablished` receipt after both SIP and application binding are ready; a terminal result before readiness returns a closed refusal rather than a plausible handle | `../connectors/crates/voice-runtime/README.md`, paragraph 3 | preserve as the `outbound` establishment rule |
| `connect-session-transport` crate | `../connectors/crates/` listing | not inspected; the `duplex_transport` profile is specified from the design, not from it |
| Inbound offer to a tenant resolved from assignment | `docs/design.md:656-691` | change for the first profile: `inbound_offer` routes to one configured destination; tenant assignment stays deferred (`resources` family, tenancy UNMAPPED) |

## 3. Types

Session record (control plane):

```json
{
  "session": "sess_…",
  "instance": "…",
  "connection": "conn_…",
  "profile": "outbound",
  "state": "ready",
  "participants": [ { "role": "adapter", "endpoint": "opaque" }, { "role": "application", "endpoint": "opaque" } ],
  "streams": [ { "id": "audio", "contract": "media/v1alpha1", "profile": "pcm-s16le-8k-mono-20ms", "direction": "duplex" } ],
  "lease": { "expires_unix_ms": 0, "renewable": true },
  "admitted": { "revision": "config revision", "authority": "auth_…" },
  "terminal": null
}
```

States: `offered → establishing → ready → closing → closed`, plus `lost` (continuity lost; outcome unknown). Terminal record:

```json
{ "state": "closed", "reason": "remote_hangup | local_close | cancelled | rejected | expired | revoked | lease_expired | media_overload | media_incompatible | transport_lost | error", "by": "adapter | application | host", "at_unix_ms": 0 }
```

Control messages over the `duplex_transport` binding (each with `session` and a direction-qualified `id`):

| Message | Direction | Rule |
|---|---|---|
| `request { id, method, body }` / `response { id, outcome }` | either | ids are unique per (session, sender); a response is matched to its sender and id |
| `event { seq, type, body }` | either | `seq` per sender; ordering per sender only |
| `offer { deadline, streams, participant_context }` | adapter → host | `inbound_offer` only |
| `accept { admitted }` / `reject { reason }` / `cancel { reason }` | host → adapter, adapter → host | one serialized decision |
| `close { reason }` | either | begins `closing` |
| `lease { expires_unix_ms }` / `revoke { reason }` | host → adapter | authenticated, session/revision-bound authority; §4.1 fixes cutoff and late-renewal rules |
| `ping`/`pong` | either | control responsiveness under data load |

Outbound establishment operation result (`operations` mutation profile, `effects` includes `session_establishment`):

```json
{ "session": "sess_…", "state": "ready", "streams": [ … ], "transport": { "binding": "websocket", "endpoint": "opaque one-use locator", "authority": "auth_…" } }
```

Errors: base codes plus `session_not_ready`, `session_lost`, `offer_expired`, `offer_rejected`, `lease_expired`, `revoked`.

## 4. Rules (`docs/design.md:406-418` settled for this profile)

| Concern | Decision |
|---|---|
| Accept versus cancel/expiry race | one serialized decision per offer in the host; `accept` after `cancel` or deadline is refused and the adapter tears down |
| Concurrent requests | bounded in-flight per direction (first-profile default 16); excess → `Capacity` response, session stays up |
| Ordering | per sender per stream; no global order across senders or between events and data |
| Half-close | a data stream may end in one direction (`stream_end`) while control stays open; control cannot half-close |
| Terminal race | the first terminal fact accepted by the session's serialized loop wins; later terminals are recorded as observations, never replace the reason |
| Backpressure | control queue bounded and prioritized over data; data policy per stream is declared by the stream contract (media: drop oldest with loss report) |
| Restart | no reattach in v1alpha1; a restart on either side yields `lost` with `transport_lost`; the outcome of in-flight requests is unknown |
| Lease/revocation | a live data lease is mandatory for every admitted data path; authoritative revocation stops all controlled data within 2 s, including partitions (§4.1) |
| Shutdown | host drain: no new offers; leases cannot extend past the drain deadline; data stops by that deadline, with bounded teardown and explicit task accounting (§4.1) |

Establishment (`outbound`): the operation returns `ready` only after every declared stream and the application binding are ready; a terminal before readiness returns `offer_rejected`/`error` with no session ref (old `SipDialEstablished` rule). The session ref is bound to the descriptor revision and connection it was admitted under; a later configuration change follows explicit continuation or revocation (`docs/design.md:485`).

Inbound (`inbound_offer`): the adapter presents `offer` with a verified ingress ref, interpreted destination, stream offer, and untrusted participant context (`docs/design.md:663-669`). The host resolves the destination from receiver-owned configuration (first profile: one configured application endpoint per ingress), checks capacity and stream compatibility, and answers within the deadline. No data byte flows before `accept`. Caller identity in `participant_context` remains untrusted.

Authority: each session has one `session-authority` issued at accept (host-issued, proof-bound, single redemption, short-lived); the transport binding presents it once; the verifier redeems before accepting bytes. Expiry of an unredeemed authority closes the session as `expired`.

### 4.1 Traffic cutoff and teardown (F06)

This profile selects **2,000 ms maximum from authoritative revocation to data cutoff** and **5,000 ms maximum from an accepted terminal fact to completion of local teardown/accounting**. These are new normative ceilings, not measurements of the old runtime, configurable suggestions, or consequences of a maximum call duration. An implementation may promise tighter bounds. It must refuse session admission if the selected binding cannot enforce these ceilings for every data path.

The session host serializes terminal decisions, establishment, and lease issuance. Let `t0` be its acceptance of the first terminal fact. It immediately refuses new session requests, data admission and renewals under its control, preserves the first reason/actor/time, and enters `closing`. A revoke notification is an acceleration mechanism; delivery or acknowledgement is not the source of cutoff authority. `closing` by itself is not evidence that remote gates have stopped. The host records cutoff and teardown separately and cannot report successful cutoff on the strength of a close message sent.

| Trigger | Data policy and deadline | Terminal / completion |
|---|---|---|
| Host revocation, cancellation, local close, remote hangup, rejection, media failure | stop local admission immediately; all controlled directions stop no later than `t0 + 2,000 ms`; no new lease after `t0` | first reason wins, including `media_incompatible` and `media_overload` |
| Data lease expires | gate stops by the effective expiry, with **no additional 2 s grace**; expiry is a local terminal fact even if the control plane is unreachable | `lease_expired`; terminal session cannot be revived by a late renewal |
| Establishment authority expires before redemption | refuse redemption and all bytes, regardless of an open socket | `expired`; no ready handle |
| Negotiation incompatible before readiness | never open a data gate or publish a ready handle; tear down admitted partial resources | `media_incompatible` |
| Host drain | no new offers; current data leases and renewals capped at the announced drain deadline; all data stops by that deadline | `revoked` if still active at the deadline; teardown/accounting within 5 s of that terminal fact |
| Control continuity lost / owner restart | local gate stops on detected loss and every remaining gate stops by its existing lease expiry; no reattach or lease re-creation | `lost`, reason `transport_lost` if no earlier terminal fact; in-flight request outcomes unknown |

**Data leases.** The 60 s establishment token is not the live-session lease. Each serving, receiving, relaying or device endpoint must gate data using authenticated host authority bound to the exact session, admitted revision, endpoints and permitted streams/directions. The effective deadline of a data lease is at most **2,000 ms after authoritative issuance**, including delivery delay, clock uncertainty, scheduling delay and already-buffered output. A partition immediately after the last renewal therefore cannot extend access beyond 2 s after host revocation. Renew only while the host still admits the session, under the same serialized authority check as revocation, and never beyond a drain or other earlier deadline.

A binding must document and prove its deadline translation and maximum uncertainty; it must fail closed when that bound cannot be established. Starting a fresh 2 s timer on receipt, replaying a renewal, or adding grace for network latency is forbidden. Sequence/expiry checks reject reordered or replayed authority. A renewal arriving after local expiry or any terminal decision cannot reopen the same session. Suspend/resume and clock changes cannot reset authority: before any further send/delivery, recheck the effective deadline. Lease and clock algorithms are binding obligations, not assurances supplied by ESS timestamps.

A trusted live-data-lease expiry, revocation or inability to verify current authority observed during readiness, data admission or renewal atomically enters `closing` and starts local cutoff/accounting. It cannot leave a renewable `ready` state awaiting a separately queued close command. Live-data-lease expiry (`GateDecision.expired` in the private model) records `lease_expired`, revocation records `revoked`, and unverifiable authority records `error` (`rejected` before readiness), unless an earlier terminal already won. Expiry of an **unredeemed establishment token** instead enters closing through `BeginClose(reason=expired)` before readiness; it is not the private live-lease expiry decision. The authoritative observation fixes the terminal time; later bookkeeping cannot restart cutoff or teardown deadlines. Actual owner/continuity loss during `closing` becomes `lost` while preserving the first terminal. Merely lacking a peer close acknowledgement does not establish owner/continuity loss.

Announce a planned drain at least 2 s before its deadline, unless the host has already confirmed an earlier cutoff for every gate. A shorter requested drain that cannot meet this condition is refused as an unsupported deadline; an emergency revoke still uses the 2 s ceiling. A terminal first observed at a disconnected peer cuts off that peer immediately; the global revocation clock starts when the authoritative host accepts the fact. This does not claim instantaneous knowledge across a partition. The disconnected path still cannot continue after its existing lease expires.

**Queues and enforcement boundary.** At a gate's cutoff, reject new input, output, DTMF and other effectful controls, discard queued input and output in both directions, clear pending playback and bridge forwarding, and cancel queued effectful requests. There is **zero data-drain allowance** after cutoff; the closing period is for bounded control cleanup only. Buffers below the application queue (transport, kernel, codec or audio device) must be purgeable or included in a proven last-emission deadline within the same 2 s ceiling. An unbounded device/transport queue is not an admitted implementation. Explicit discard on termination is recorded in local bounded accounting; it is distinct from silent overflow loss during a live session. A closed peer need not receive a loss notification for cutoff to succeed.

The guarantee covers further emission and delivery across the last boundary controlled by each admitted endpoint, including local audio output. It cannot recall packets already beyond that boundary or stop an untrusted remote device playing bytes already delivered. Peer protocol shutdown is recorded as confirmed or unconfirmed independently of local cutoff. Direct media uses the same serving/receiving enforcement and expiring authority even when a gateway carries no bytes. A bridge gates both legs under their own authority and propagates terminal facts without minting extra lifetime; a multi-hop path must satisfy the **end-to-end 2 s ceiling**, not 2 s per hop. Closing a control socket alone is insufficient.

**Teardown.** By `t0 + 5,000 ms`, stop/join local tasks, release owned transport/device resources, and finish accounting. An unresponsive peer cannot extend this deadline: attempt bounded protocol close, then close local resources. `closed` records confirmed local release; it does not assert remote cooperation or a successful outcome for an unanswered request. If continuity or resource release cannot be established, report `lost` and the unresolved resources/outcomes by the same deadline; do not fabricate `closed` or hide a bound violation. Preserve an already-recorded terminal reason, even when later cleanup fails. A host crash is not proof of teardown: surviving endpoints enforce existing leases, and the restarted owner records loss/accounting without resurrecting authority.

## 5. Ordering and limits

| Bound | Value |
|---|---|
| Offer deadline | 10 s |
| Authority lifetime | 60 s (old profile) |
| In-flight requests per direction | 16 |
| Control message size | 64 KiB |
| Revocation to controlled data cutoff | 2 s maximum, normative (§4.1) |
| Live data-lease lifetime | at most 2 s from authoritative issuance, all uncertainty included |
| Data drain after cutoff | 0 frames / 0 ms; queued data discarded |
| Terminal fact to local teardown/accounting | 5 s maximum, normative; forced local close independent of peer |
| Sessions per instance | configuration |

Other table values are first-profile defaults; the authority lifetime is inherited from the old profile. They do not relax the normative cutoff/teardown ceilings.

## 6. Conformance scenarios (`docs/design.md:983`)

- Both participants issue requests with the same id concurrently → both answered to the correct sender.
- Offer cancelled 1 ms before accept → accept refused; adapter fixture observes teardown; no data accepted.
- Two terminals (remote hangup, then local close) → reason `remote_hangup`; second recorded as observation.
- Data flood on a stream → control stays responsive; revocation cuts off all controlled directions within 2 s, clears queued frames/signals, and completes local teardown/accounting within 5 s even if the peer ignores close.
- Transport dropped → state `lost`, in-flight requests reported unknown; no fabricated `closed` success.
- Authority presented twice → second refused; bytes before redemption refused.
- Same suite in-process, over the WebSocket binding, and through a one-hop fake relay (`docs/design.md:1035`).
- Direct path partitioned immediately after a renewal → data stops by that lease's original effective expiry, no receipt-time reset or grace; delayed renewal cannot revive it.
- Drain deadline precedes normal lease expiry → last emission/delivery no later than drain deadline.
- Negotiation incompatible → `media_incompatible`, never ready; overload → `media_overload`; both reasons are in this shared vocabulary.
- A prior `remote_hangup` followed by revocation or a cleanup failure → the original terminal reason remains; unconfirmed cleanup becomes `lost`, not a fabricated successful close.

The [verification record](verification.md) separates compiled lifecycle scenarios from the future timed transport/device tests required by §4.1.

## 7. Compatibility

- Old VoiceSession state vectors (`contracts/voice-session/v0alpha1/vectors.json`) are characterization input; the renamed states map `created → establishing`, others unchanged.
- The old `sip-dial` response shape (`call`, `session`, `channel`) is not carried; a facade maps `session` and derives `call`/`channel` from streams.
- [Service compatibility](../../service/compatibility.md) is authoritative for the binding. Unary establishment requires the mutation observation binding. Duplex control, correlation, live authority and every selected transport need their own complete versioned bindings before advertisement or readiness; the unary wire version does not supply them.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Session control types and state machine | `crates/connectors-core` (kernel) |
| `duplex_transport` binding (WebSocket candidate, `docs/design.md:1131`) with size limits and decoder independent of the network library | new host module |
| Session host: offer admission, authenticated live leases, serialized revoke/renew, cutoff receipt and bounded teardown/accounting | new host module; all direct/relay/device bindings must implement §4.1 |
| Federation: relay only supported profiles, or advertise direct-session arrangement with scoped endpoint authority (`docs/design.md:789`) | `crates/connectors-host/src/federation.rs` |
| Operation result type for `session_establishment` | `operations` document |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `Session` (identity: session ref), lifecycle `Offered → Establishing → Ready → Closing → Closed`, plus `Lost` | modeled in [sessions.yaml](../../../ess/domains/sessions.yaml); opaque connection/authority coordinates, their typed relations are UNMAPPED pending those owner models; stream/path metadata are values |
| `Offer` | value on `Session` while `offered` |
| Tenant/application assignment of an inbound destination | UNMAPPED (`resources` deferred) |

The ESS lifecycle describes one supervising owner's decisions. It does not derive the trustworthiness of lease observations, compare deadlines, stop remote gates, enforce queue/device limits, assign immutable terminal fields, or account for real tasks. Author/synthesize checks declarations, names and types and constructs obligations; it does **not execute the sequential scenario trace** against the model. The [verification record](verification.md) includes a separate manual lifecycle/deadline audit. These executable obligations remain explicit; a compiled scenario is not a runtime cutoff measurement or proof that every authored state sequence is possible.

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Transport | WebSocket first; local in-process channel for tests |
| Reattach/resume | not in v1alpha1 |
| Offer deadline | 10 s |
| Inbound destination model | one configured endpoint per ingress; assignment resolver deferred |

Persistence ownership is consolidated in [design §31](../../../docs/design.md#31-host-persistence-ownership-and-atomicity). SessionRedemptionPort and SessionSupervisorPort distinguish one-use establishment from live lease/terminal/accounting state. Restart cannot restore a live session. This inventory does not supply a backend or execute its atomicity predicates.
