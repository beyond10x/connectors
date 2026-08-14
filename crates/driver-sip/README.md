# driver-sip

The only Connectors crate allowed to open SIP/RTP sockets. It consumes a non-serializable
`server::AdmittedSipPlan` and implements the protocol-neutral `domain::TelephonySession` port with
the exact `codewandler/sipx` `v1.0.0-rc.23` source commit.

This first implementation is intentionally restricted to explicit loopback development. The
pinned sipx call path binds its media port internally and can transmit to SIP/SDP-learned peers
before an outer adapter can inspect them. The adapter validates those peers before exposing the
session, but that is characterization—not a stable network-aperture guarantee.

The crate's production dependencies contain sipx but not RTVBP. Its test-only dependency closure
joins `rtvbp-voice-endpoint` solely for the model-free repository conformance call; neither adapter
imports or implements the other's protocol semantics.
