# Adapter design: media session (SIP via sipx, RTVBP, bridge, local audio)

- **Status:** design, not implemented. Adapter services: `connectors.sip`, `connectors.rtvbp`; composition components: bridge, local audio device; WebRTC is an interoperability target only.
- **Old baseline:** `../connectors` at `81459ac4`: `providers/b10x.toml:741-760` (`sip-dial`), `contracts/voice-session/v0alpha1/`, `crates/driver-sip/`, `crates/integration-sip/`, `crates/rtvbp-voice-endpoint/`, `crates/voice-runtime/`, `crates/voice-local-audio/`, `crates/driver-audio/`, `crates/driver-speech/`, `docs/design/05-native-sip-and-rtvbp.md`.
- **Contract index:** [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

Two independent adapters implement the same two contracts, [sessions](../../contracts/sessions/v1alpha1/semantics.md) and [media](../../contracts/media/v1alpha1/semantics.md): SIP terminates SIP/SDP and RTP toward a trunk or PBX; RTVBP carries the session and media to an application endpoint over its own transport. A bridge joins two media sessions and imports neither protocol crate (`docs/design.md:643-654`). A local audio device binding plays and captures on this machine's sound stack (`crates/voice-local-audio`, `driver-audio`). Placement: the SIP adapter sits where the trunk or PBX is reachable (a satellite beside a private PBX in the old design, `docs/design/05:306-309`); RTVBP connects outward to the application endpoint; SaaS never dials into the private network.

Rebuild means: outbound dial parity with `sip-dial`, the narrow PCM profile, DTMF and barge-in, the RTVBP `b10x.voice.v1` binding, and the bridge, on the new contracts; inbound offers to one configured destination; tenant assignment stays deferred.

## 2. Old surface and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `sip-dial`: `session_establishment`, `sip_v1`, risk high, non-idempotent, `send_external`, `human_visible`; `target` opaque trunk alias (never URI/host/port), `number` digits only; response `call`, `session`, optional `channel`, `state = established` | `providers/b10x.toml:741-760` | preserve → `operations` `mutation` with `effects: [session_establishment, send_external]` returning a session ref (`sessions` `outbound` profile) |
| Connectors owns the configured SIP Connection, trunk credentials, destination/listener aperture, grant admission, inbound tenant/channel binding, driver selection, placement, call lifecycle, session-authority issuance, audit; `sip_v1` driver uses pinned `codewandler/sipx`; the endpoint is a user agent, not a proxy/registrar/PBX/TURN/IVR; caller ID untrusted | `docs/design/05:33-49` | preserve; tenant/channel binding deferred |
| `driver-sip`: only crate opening SIP/RTP sockets; consumes non-serializable `AdmittedSipPlan`; implements `TelephonySession`; sipx `v1.0.0-rc.23`; loopback by default; `OperatorAuthorizedDevelopment` mode for explicit non-loopback apertures; sipx binds its media port internally and can transmit to SDP-learned peers before validation, so every learnable address and port range must be pre-admitted; production deps contain sipx but not RTVBP | `crates/driver-sip/README.md:1-25` | preserve the socket-ownership and aperture rules; drop the service-layer `AdmittedSipPlan` dependency (`docs/design.md:82`): the adapter takes configuration and a `sip-credential-lease` |
| Credentials: exactly username then password, passed to `sipx_call::Credentials` | `crates/driver-sip/src/lib.rs:40-42,104-114` | preserve → `auth.capability` `sip-credential-lease` |
| DTMF as RFC 4733 telephone-events; refusal when the media session declines | `crates/driver-sip/src/lib.rs:359-381` | preserve → `media` capability `dtmf` |
| VoiceSession v0alpha1: opaque refs, untrusted participant context, bounded audio, signals, interruption, loss, typed termination; `created → ready → closing → closed`; first terminal wins; input drops oldest with loss; output overload degrades or terminates `media_overload` | `contracts/voice-session/v0alpha1/README.md` | preserve, split across `sessions` and `media` |
| Profile `pcm_s16le`, 8 kHz, mono, 20 ms, 320 bytes, asserted at bind; no transcoding at the seam | `crates/voice-local-audio/README.md:18-26` | preserve as `media` profile `pcm-s16le-8k-mono-20ms` |
| `rtvbp-voice-endpoint`: binding of VoiceSession to the exact `b10x.voice.v1` RTVBP profile; consumes only `TelephonySession` and a serving-endpoint-redeemed authority; generic envelope with finite in-memory transport; production WebSocket gated on bounding the upstream runtime's unbounded queues; does not use the stock upstream `Session` runtime | `crates/rtvbp-voice-endpoint/README.md:1-11` | preserve as the RTVBP adapter's binding facts |
| 60-second proof-bound single-redemption authority with DPoP on the WebSocket upgrade; `(iss, jti)` redeemed before bytes; application endpoint serves, voice endpoint connects; no federation relay of RTVBP or audio; live dialogs never migrate; bounded drain | `docs/design/05:297-318` | preserve → `auth.capability` `session-authority` / `inbound-verifier`; `sessions` lease/drain rules |
| `voice-runtime`: resolves operation-scoped SIP credentials, establishes the driver port, issues authority, connects outward to the admitted application endpoint, starts the RTVBP binding, supervises control, signals, duplex media, keepalive, lease, termination as one task; `dial_establishment_channel()` emits exactly one established receipt after both sides are ready; nested workspace to join both dependency closures | `crates/voice-runtime/README.md` | change: this becomes the composition (bridge + host session supervision); no nested workspace (`docs/design.md:168`) |
| `voice-local-audio`, `driver-audio` (`audio_v1`, PipeWire/PulseAudio/ALSA, `NullAudioDevice`), `driver-speech` (`speech.speak`, `speech.status`, piper) | crate READMEs | preserve local audio as a `media` binding for a workstation composition; speech operations are outside this document |
| Asterisk ARI provider (HTTP, `https://{host}:8089/ari`) | `providers/asterisk.toml` | not a media contract; an ordinary HTTP adapter if wanted later |
| Inbound calls | `docs/design/05:33-49` names inbound binding; `providers/b10x.toml` ships only `sip-dial` | new: `inbound_offer` to one configured destination; assignment resolver deferred (`docs/design.md:1057`, `resources`) |

## 3. Contracts needed and why

SIP adapter:

| Contract | Profile / use | Why |
|---|---|---|
| `operations` `mutation` | `sip.dial` with `effects: [session_establishment, send_external, human_visible]`, `idempotency: none`, `approval: required` | placing a call rings a real endpoint; needs approval binding and unknown-outcome semantics |
| `sessions` | `outbound`, `inbound_offer`, `duplex_transport` | call lifecycle, offer admission, lease, revocation, terminal races |
| `media` | `pcm-s16le-8k-mono-20ms`; capabilities `dtmf`, `interrupt` | the negotiated duplex audio and controls the driver already implements |
| `auth.profile` | `sip.trunk` (`sip_digest`, purpose `trunk_registration`) | trunk username/password |
| `auth.capability` | `sip-credential-lease`, `session-authority` (issue) | bytes released to sipx for one establishment; per-session authority toward the application side |
| `auth.custody` | `versioned` or `read_only` | trunk secret storage |
| `auth.evidence` | `custody_reachable` only | value-free readiness; never probe the trunk before an admitted operation (old `voice-runtime` rule) |
| `auth.connection` | `configured` | one trunk binding per connection; `target` alias selects among configured trunks |

RTVBP adapter:

| Contract | Profile / use | Why |
|---|---|---|
| `sessions` | `duplex_transport` binding over RTVBP's envelope; `outbound` (connect outward to the application endpoint) | carries the session control across RTVBP |
| `media` | `pcm-s16le-8k-mono-20ms` mapped to `b10x.voice.v1` (`L16/8000/1` label stays inside the binding) | same semantics, independent implementation |
| `auth.capability` | `session-authority` (present with DPoP on upgrade), `inbound-verifier` (when serving) | proof-bound establishment |
| `auth.profile` | `rtvbp.session_authority` (`session_authority`, subject `none`) | no vendor credential; host-issued per session |

Bridge (composition, not an adapter): consumes `media` twice and `sessions` for joint termination; no auth contracts of its own.

Local audio (composition binding): consumes `media` with the same profile; device selection is deployment configuration, never caller input (`crates/driver-audio/README.md`, enumeration reserved).

## 4. Operation map

| New id | Contract / profile | Effects | Old id |
|---|---|---|---|
| `sip.dial` | mutation → sessions `outbound` | session_establishment, send_external, human_visible | `sip-dial` |
| `sip.sessions.list` | records list (safe session summaries) | read | — |
| `session.close` | sessions control | terminal | hangup via driver |
| `session.signal` (dtmf) | media capability | — | driver DTMF |
| `session.interrupt` | media capability | — | barge-in |
| (inbound) `offer` → host `accept`/`reject` | sessions `inbound_offer` | — | designed, not shipped |

## 5. Auth

| Adapter | Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|---|
| sip | `sip.trunk` | `sip_digest` | `static_entry` (two fields) | `sip-credential-lease` (one establishment, bounded) | `custody_reachable` |
| sip → application | issued per session | `session_authority` | `host_issued` | `session-authority` | redemption ledger |
| rtvbp | `rtvbp.session_authority` | `session_authority` | `host_issued` | `session-authority` present / `inbound-verifier` redeem | redemption ledger |

## 6. Configuration outline

SIP:

```json
{ "adapter": {
    "trunks": [ { "alias": "staging-pbx", "signaling": { "transport": "tcp", "host": "…", "port": 5060 }, "credential": { "$ref": "urn:connectors:config:v1:credential" }, "allow_dialed_numbers": true } ],
    "default_trunk": "staging-pbx",
    "network": { "mode": "loopback | operator_authorized", "media_port_range": [40000, 40100], "admitted_peers": ["10.0.0.0/24"] },
    "media": { "profiles": ["pcm-s16le-8k-mono-20ms"], "capabilities": ["dtmf", "interrupt"] },
    "inbound": { "enabled": false, "listener": { "transport": "tcp", "bind": "…" }, "destination": { "application_endpoint": "…" } },
    "limits": { "max_sessions": 8, "max_call_seconds": 3600 } } }
```

`network.mode` and `admitted_peers` carry the old README's rule that every SDP-learnable address and the port range are pre-admitted.

RTVBP: application endpoint(s), TLS expectations, `b10x.voice.v1` profile, queue bounds.

## 7. Discovery and routes

None. Media bytes follow the negotiated data path; federation relays neither RTVBP nor audio (`docs/design/05:306-309`; `docs/design.md:789`).

## 8. Specification profile

`connectors.adapter/v1` handwritten for both adapters; no OpenAPI source exists for SIP or RTVBP. The specification declares provided contracts and profiles; the implementation obligations are the protocol code (`docs/design.md:907`).

## 9. Deferred

`resources` (numbers, trunks, tenant assignment; inbound routing by assignment), `execution`, recordings and transcripts (forbidden from the contract), WebRTC (no claim until it passes the media suite), hold/transfer capabilities, speech operations.

## 10. Evidence required

- Fixture (`docs/design.md:1002`): two fake conforming endpoints before any protocol stack: establishment, duplex traffic, DTMF, interruption, overload, cancellation, terminal races; then SIP and RTVBP implementations through the contract-only bridge; substituting either endpoint leaves bridge and application unchanged.
- Live (separately authorized, not in default tests): TCP SIP plus RTP echo against a dev PBX as the old `sip_dial_characterize` example did (`crates/driver-sip/README.md:15-19`).
- Decoupling: the SIP crate has no RTVBP dependency and vice versa; the client and host build without either (`docs/design.md:1010-1015`).
