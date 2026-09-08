# RTVBP session binding/v1alpha1

**Status:** proposed independent `connectors.rtvbp`, not implemented.
Implements shared sessions/duplex_transport/outbound and negotiated media over the
exact native `b10x.voice.v1` profile. This adapter imports no SIP implementation.

The application endpoint serves; the voice/session endpoint connects outward to
the admitted application endpoint. Native envelope, transport and authority
binding belong here. The predecessor used finite in-memory transport and explicitly
did not use the stock upstream Session runtime. Production WebSocket support
requires bounding the upstream runtime's unbounded queues before advertisement.

A host-issued 60-second proof-bound single-redemption authority carries DPoP on
the WebSocket upgrade. Redeem (iss,jti) at the serving endpoint before data bytes.
The rtvbp.session_authority profile selects session_authority, host_issued,
session-authority presentation and inbound-verifier redemption with a ledger.
No vendor credential or parent grant is invented.

Establishment authority is separate from the live data lease, at most 2 s from
host issuance. Direct paths expire without a reachable control plane. Enforce
shared revocation/cutoff and terminal rules: <=2 s to controlled data cutoff,
zero queued-data drain after cutoff, <=5 s local teardown/accounting. Closing a
WebSocket does not establish cutoff in another binding or device. Remote shutdown
may remain unconfirmed but cannot extend controlled traffic/resource ownership.

Configuration owns admitted application endpoints, TLS expectations, exact
b10x.voice.v1 profile and bounded queues. Native control/data codecs, authority
upgrade proof, queue accounting and native transport fixtures remain explicit
authoring/implementation obligations. The shared session/media documents are not
a completed RTVBP wire codec.

Historical evidence: ../connectors/crates/rtvbp-voice-endpoint/README.md:1–11 and
docs/design/05-native-sip-and-rtvbp.md:297–318 at 81459ac4. Adopt exact upstream
inputs/licenses/pins and fixtures with this adapter before extraction. Planned
adapter-kind v1 is handwritten; no HTTP OpenAPI source describes this protocol.
