# driver-sip

The only Connectors crate allowed to open SIP/RTP sockets. It consumes a non-serializable
`server::AdmittedSipPlan` and implements the protocol-neutral `domain::TelephonySession` port with
the exact `codewandler/sipx` `v1.0.0-rc.23` source commit.

This implementation admits loopback by default. An operator can select the explicit
`OperatorAuthorizedDevelopment` network mode for exact non-loopback signaling and media apertures;
that path has completed a TCP SIP plus RTP echo characterization against the dev-cluster Asterisk.
It is not a stable-network claim. The pinned sipx call path binds its media port internally and can
transmit to SIP/SDP-learned peers before an outer adapter can inspect them. The adapter validates
those peers before exposing the session, but the operator must have pre-admitted every possible
learned address and bounded port range.

`examples/sip_dial_characterize.rs` is the operator-only proof tool. It resolves the generated
`sip-dial` catalog member, applies Connection initiation plus Grant evidence, selects only the fixed
`asterisk-dev` alias, establishes the call, verifies the neutral 8 kHz/20 ms RTP echo, emits opaque
references, and terminates the dialog. Environment fields configure the reviewed route; none is
caller/model input and the example is not a general dial proxy.

The crate's production dependencies contain sipx but not RTVBP. Its test-only dependency closure
joins `rtvbp-voice-endpoint` solely for the model-free repository conformance call; neither adapter
imports or implements the other's protocol semantics.
