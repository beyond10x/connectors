# rtvbp-voice-endpoint

Voice-side binding of `b10x.voice-session.v0alpha1` to the exact
`b10x.voice.v1` RTVBP profile. It consumes only a protocol-neutral `TelephonySession` and a
serving-endpoint-redeemed authority. It does not use `babelforce.v1` or expose RTVBP types to the
domain port.

The first proof uses the upstream generic envelope and finite in-memory transport. Production
WebSocket support remains gated on replacing or bounding the upstream runtime's internal unbounded
queues; this crate does not use the stock upstream `Session` runtime.

