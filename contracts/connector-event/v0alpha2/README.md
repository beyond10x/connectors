# ConnectorEvent v0alpha2

Exact protocol identity: `b10x.connector-event.v0alpha2`. This version adds explicit
`subscribe` and `unsubscribe` for endpoint-backed provider channels. Search, receive and replay
retain their v0alpha1 shapes and admission policy. Older contract bundles remain immutable.

Subscribe takes `endpoint_ref`, a catalog-declared `channel_binding`, and a required `parameters`
object containing only provider-declared channel configuration. No credential values or arbitrary
route settings belong in this object. At most 32 keys and 16 KiB of encoded parameters are accepted.
The runtime validates the provider's declared parameter names and types before opening a route.

The response identifies the subscription and its durable event channel. Receive and replay use the
existing event references and cursor semantics. Unsubscribe requires the exact subscription reference.
The runtime retains the route while subscribed and releases it on stop, shutdown, or authority change.
Subscriptions belong to the verified principal, immutable endpoint identity and current channel grant.

Hosted lifecycle requests require the existing operator event-read authority; endpoint inventory alone
does not grant subscription access. Local requests use the owner-only daemon socket and source grants.
Frames are bounded to 64 KiB and responses to 1 MiB. SDKs select this version explicitly and validate
request correlation, result method and channel/subscription references, without version fallback.

`cargo run --locked -p protocol --example endpoint-contracts` verifies the deterministic schema and
bundle hashes. Schema bounds supplement the documented byte budgets and runtime authorization checks.
