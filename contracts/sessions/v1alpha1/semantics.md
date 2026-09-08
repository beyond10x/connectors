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
{ "state": "closed", "reason": "remote_hangup | local_close | cancelled | rejected | expired | revoked | lease_expired | media_overload | transport_lost | error", "by": "adapter | application | host", "at_unix_ms": 0 }
```

Control messages over the `duplex_transport` binding (each with `session` and a direction-qualified `id`):

| Message | Direction | Rule |
|---|---|---|
| `request { id, method, body }` / `response { id, outcome }` | either | ids are unique per (session, sender); a response is matched to its sender and id |
| `event { seq, type, body }` | either | `seq` per sender; ordering per sender only |
| `offer { deadline, streams, participant_context }` | adapter → host | `inbound_offer` only |
| `accept { admitted }` / `reject { reason }` / `cancel { reason }` | host → adapter, adapter → host | one serialized decision |
| `close { reason }` | either | begins `closing` |
| `lease { expires_unix_ms }` / `revoke { reason }` | host → adapter | must be honored within the declared bound |
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
| Lease/revocation | host may set a lease and revoke; adapter must reach `closing` within the declared bound (first-profile default 2 s) |
| Shutdown | host drain: no new offers, existing sessions finish or are revoked at the drain deadline; tasks are joined and accounted (`docs/design.md:418`) |

Establishment (`outbound`): the operation returns `ready` only after every declared stream and the application binding are ready; a terminal before readiness returns `offer_rejected`/`error` with no session ref (old `SipDialEstablished` rule). The session ref is bound to the descriptor revision and connection it was admitted under; a later configuration change follows explicit continuation or revocation (`docs/design.md:485`).

Inbound (`inbound_offer`): the adapter presents `offer` with a verified ingress ref, interpreted destination, stream offer, and untrusted participant context (`docs/design.md:663-669`). The host resolves the destination from receiver-owned configuration (first profile: one configured application endpoint per ingress), checks capacity and stream compatibility, and answers within the deadline. No data byte flows before `accept`. Caller identity in `participant_context` remains untrusted.

Authority: each session has one `session-authority` issued at accept (host-issued, proof-bound, single redemption, short-lived); the transport binding presents it once; the verifier redeems before accepting bytes. Expiry of an unredeemed authority closes the session as `expired`.

## 5. Ordering, limits (first-profile defaults, to be measured)

| Bound | Value |
|---|---|
| Offer deadline | 10 s |
| Authority lifetime | 60 s (old profile) |
| In-flight requests per direction | 16 |
| Control message size | 64 KiB |
| Revocation bound | 2 s to `closing` |
| Sessions per instance | configuration |

## 6. Conformance scenarios (`docs/design.md:983`)

- Both participants issue requests with the same id concurrently → both answered to the correct sender.
- Offer cancelled 1 ms before accept → accept refused; adapter fixture observes teardown; no data accepted.
- Two terminals (remote hangup, then local close) → reason `remote_hangup`; second recorded as observation.
- Data flood on a stream → `ping`/`pong` latency stays within bound; `revoke` reaches `closing` within 2 s.
- Transport dropped → state `lost`, in-flight requests reported unknown; no fabricated `closed` success.
- Authority presented twice → second refused; bytes before redemption refused.
- Same suite in-process, over the WebSocket binding, and through a one-hop fake relay (`docs/design.md:1035`).

## 7. Compatibility

- Old VoiceSession state vectors (`contracts/voice-session/v0alpha1/vectors.json`) are characterization input; the renamed states map `created → establishing`, others unchanged.
- The old `sip-dial` response shape (`call`, `session`, `channel`) is not carried; a facade maps `session` and derives `call`/`channel` from streams.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Session control types and state machine | `crates/connectors-core` (kernel) |
| `duplex_transport` binding (WebSocket candidate, `docs/design.md:1131`) with size limits and decoder independent of the network library | new host module |
| Session host: offer admission, lease, revoke, drain, accounting | new host module |
| Federation: relay only supported profiles, or advertise direct-session arrangement with scoped endpoint authority (`docs/design.md:789`) | `crates/connectors-host/src/federation.rs` |
| Operation result type for `session_establishment` | `operations` document |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `Session` (identity: session ref), lifecycle `offered → establishing → ready → closing → closed`, plus `lost` | relations: `connection → Connection` (one), `authority → SessionAuthority` (one), `streams` values |
| `Offer` | value on `Session` while `offered` |
| Tenant/application assignment of an inbound destination | UNMAPPED (`resources` deferred) |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Transport | WebSocket first; local in-process channel for tests |
| Reattach/resume | not in v1alpha1 |
| Offer deadline | 10 s |
| Inbound destination model | one configured endpoint per ingress; assignment resolver deferred |
