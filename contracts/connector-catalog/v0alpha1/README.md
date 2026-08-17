# ConnectorCatalog v0alpha1

`b10x.connector-catalog.v0alpha1` is the credential-free descriptive catalog contract between
Connectors and clean-room clients such as Agent. It exposes the complete provider catalog through
bounded, stable `search` pages and exact `describe` reads. Catalog membership is not a Connection,
callability, authority, placement, entitlement, or invocation route.

The projection contains only authored provider and operation metadata. It never contains a
credential, configured value, daemon address, socket path, provider response, Connection reference,
Grant, or approval evidence. `configurable` means that the serving Connectors runtime publishes a
Connector-owned setup flow for that provider; it does not mean that Agent may collect provider
secrets.

`connector-catalog.schema.json` is the strict JSON request/response schema. `vectors.json` contains
positive and adversarial request frames consumed by the Rust `protocol` crate. A released
clean-room client copies this bundle or generated DTOs; it never imports Connectors source crates.

Every transport frame is one request or response. Local transport authentication and the owner
context are independent gates. The owner context controls access to the catalog route, while the
returned membership remains descriptive. Unknown fields, invalid references, invalid pagination,
oversize values, and malformed authority snapshots refuse before catalog work.

- `search` performs a case-insensitive metadata match and returns at most 64 provider summaries in
  stable catalog order. `next_offset` is an opaque continuation for the same catalog generation.
- `describe` returns one provider summary and its complete authored operation index. An operation's
  `exposed` bit is curation metadata, not proof that any Connection can call it.
- Clients obtain connected and callable state only from the separate ConnectorConnection and
  ConnectorOperation contracts. They must not infer those states from this contract.

