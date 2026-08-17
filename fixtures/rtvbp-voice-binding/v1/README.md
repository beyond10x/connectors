# RTVBP binding `b10x.voice.v1`

This directory is the Connectors-owned adaptation of neutral
`b10x.voice-session.v0alpha1` semantics onto RTVBP. It pins the final upstream Rust SDK
release `sdk/rust/v0.1.0` at commit `dc0a60f7425b4899885f372152028457791b1e72`, selects its
generic `classic.v1` envelope/transport APIs, and defines an explicit local profile. It neither adds
B10x operations to RTVBP nor accepts RTVBP's product profile by default.

The voice endpoint presents its issued Session Authority and proof during the exact WebSocket
upgrade. The serving application endpoint verifies and atomically redeems that authority once. No
redeemed token crosses back into the voice endpoint.

`manifest.json` is the complete mapping and bounds declaration. `vectors.json` contains binding
conformance cases. The Rust endpoint currently exercises exact negotiation, authority direction,
initialization, and duplex in-memory media against a fake neutral `TelephonySession`; production
WebSocket serving and the complete overload/lifecycle vector set remain separate runtime work.
