# SIP adapter design

**Status:** proposed independent `connectors.sip`, not implemented.
Native dial/effect/media semantics live in [the dial binding](contracts/dial/v1alpha1/semantics.md).

SIP terminates SIP/SDP and RTP toward a configured trunk/PBX, where that peer is
reachable. The old satellite placement is evidence, not a mandatory host topology.
Outbound dial and inbound offers to one configured application destination are
selected; SaaS tenant/channel assignment remains deferred. The adapter imports no
RTVBP stack. Session composition and authority issuance use shared host ports.

## Contracts

| Contract | Profile / use | Why |
|---|---|---|
| `operations` `mutation` | `sip.dial` with `effects: [external_write, network, send_external, session_establishment]`, `semantic_effects: [human_visible]`, `idempotency: none`, `approval: required` | placing a call rings a real endpoint; needs approval binding and unknown-outcome semantics |
| `sessions` | `outbound`, `inbound_offer`, `duplex_transport` | call lifecycle, offer admission, lease, revocation, terminal races |
| `media` | `pcm-s16le-8k-mono-20ms`; capabilities `dtmf`, `interrupt` | the negotiated duplex audio and controls the driver already implements |
| `auth.profile` | `sip.trunk` (`sip_digest`, purpose `trunk_registration`) | trunk username/password |
| `auth.capability` | `sip-credential-lease`, `session-authority` (issue) | bytes released to sipx for one establishment; per-session authority toward the application side |
| `auth.custody` | `versioned` or `read_only` | trunk secret storage |
| `auth.evidence` | `custody_reachable` only | value-free readiness; never probe the trunk before an admitted operation (old `voice-runtime` rule) |
| `auth.connection` | `configured` | one trunk binding per connection; `target` alias selects among configured trunks |


## Operation map

| Operation | Selected behavior |
|---|---|
| sip.dial | approved non-idempotent mutation returning a ready outbound session |
| sip.sessions.list | bounded records with safe session summaries |
| session.close | terminal session control, native hangup |
| session.signal | negotiated DTMF media signal |
| session.interrupt | negotiated interruption/barge-in |
| inbound offer, accept/reject | shared inbound_offer, designed but unimplemented |

## Native authentication

sip.trunk selects sip_digest, static_entry of two fields, a bounded one-establishment
sip-credential-lease and custody_reachable evidence. No protocol code owns durable
credential custody or application authority.


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


## Sources, verification and deferred scope

Original evidence: ../connectors/providers/b10x.toml:741–760, driver-sip README
and lib.rs:40–42,104–114,359–381, integration-sip and design 05 at 81459ac4.
The old codewandler/sipx v1.0.0-rc.23 dependency is a historical pin. Reverify/adopt
its exact source/license and current obligations under this adapter before new
implementation; no source refresh occurred during this layout change.

The planned Connectors adapter-kind v1 is handwritten; no HTTP OpenAPI describes
the SIP protocol. Native schemas, establishment/no-effect proof and enforcement
fixtures remain authoring/implementation prerequisites. Shared terminal cutoff is
2 s from authoritative revocation and teardown/accounting 5 s from terminal
acceptance, independent of max_call_seconds; prove SIP/RTP transmit-buffer cutoff
even if a remote PBX does not confirm shutdown.

Inbound assignment/provisioning, hold/transfer, codecs beyond the narrow selected
profile and arbitrary PBX behavior remain deferred. [Media composition](../../docs/compositions/media-session.md)
owns cross-binding fixtures and separately authorized live PBX tests.
