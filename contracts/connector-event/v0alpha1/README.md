# ConnectorEvent v0alpha1

`b10x.connector-event.v0alpha1` is the generic durable data-event pull/replay contract. It is
provider-neutral: a client searches admitted Channels, receives after an opaque cursor, and replays
one event by opaque `event_ref`.

Every event carries Connection and Channel attribution, a catalog event type, honest `native` or
`polled` provenance, receipt time, and the normalized provider payload. It never carries provider
credentials, credential addresses, Connect Session endpoints, transport tickets, or raw transport
envelopes. Operational events are a separate family and are not smuggled into this data stream.

The personal-local binding uses bounded long poll over the owner-authenticated Connector Unix
socket. Search grants nothing; receive and replay re-evaluate the current owner, Connection,
Channel, and closed inbound event grant. General multiplexed subscription and signed push delivery
remain M4 work.
