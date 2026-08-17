# voice-runtime

This nested runtime leaf is the only production composition point for an admitted native voice
session. It resolves operation-scoped SIP credentials, establishes the `driver-sip` neutral
telephony port, issues an ephemeral proof-bound authority, connects outward to the exact admitted
application endpoint, starts the `b10x.voice.v1` RTVBP binding, and supervises control,
signals, duplex media, keepalive, lease, and termination as one task.

Every `CredentialSource` must also implement a value-free readiness check. A stored SIP source uses
that check to prove only its injected Secret Store dependency; it never resolves a credential or
probes the SIP provider before an admitted operation.

`dial_establishment_channel()` is the operation-serving seam. Its observer emits exactly one
serializable `SipDialEstablished` receipt after both SIP and the authenticated application binding
are ready, while `run_outbound` continues supervising the live session. A terminal result that wins
before readiness returns a closed establishment refusal rather than a plausible session handle.

The crate owns no SIP, RTVBP, or product semantics. Those remain in `driver-sip`,
`rtvbp-voice-endpoint`, and the protocol-neutral owner contract respectively. DNS/TCP/proxy/TLS
establishment remains behind the deployment-provided `ApplicationConnector`; the connector must
return an already TLS-protected stream for the exact route it receives.

It is a nested Cargo workspace because it intentionally joins both isolated runtime dependency
closures. It must never enter the deterministic catalog compiler workspace or its lockfile.
