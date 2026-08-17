# ConnectorOperation v0alpha1

`b10x.connector-operation.v0alpha1` is the credential-free control contract between a
Connectors service and clean-room clients such as Agent. It exposes bounded `search`, `describe`,
`invoke`, and generic session status/termination/reconciliation. It never exposes a daemon address,
provider credential, SIP URI, socket destination, tenant-selected placement, or driver type.

Local transport authentication and the request's owner context are independent gates. Connectors
re-evaluates the current authority snapshot, Connection, Grant, description lease, approval
evidence, catalog generation, and operation input at invocation. Search is not authority; describe
is not a reservation. A session-producing invocation returns an opaque `execution_ref`, and the
Connectors process continues owning the runtime after the establishment receipt.

`connector-operation.schema.json` is the strict JSON frame schema. `vectors.json` contains positive
and adversarial request frames consumed by the Rust `protocol` crate. A released clean-room client
copies this bundle or generated DTOs; it never imports Connectors source crates.

Every transport frame is one request or response. The personal-local binding uses one
newline-delimited request per owner-credentialed Unix-socket connection; that binding is not part of
the semantic contract and a later hosted transport must authenticate independently.

- `search` returns bounded summaries and callable Connections. Each Connection carries its target
  Provider plus curated catalog audiences for Explorer filtering. These are discovery metadata,
  never authorization, visibility, entitlement, or a transport selection. Search grants nothing.
- `describe` returns exact input/output schemas, effect/approval posture, and an opaque
  `description_ref` lease bound to the current catalog, owner snapshot, Connection, Grant, and
  approval configuration.
- `invoke` requires that lease, an exact Connection, structured input, and (when declared) an
  externally issued approval evidence reference. There is no caller-written `approved` boolean.
- A session-producing invoke may return an `execution_ref`. `session_status` observes the owning
  runtime. `session_terminate` may request only `completed`, `cancelled`, or `revoked`; remote end,
  lease expiry, failure, and unknown outcome are observations, not caller assertions.
- `session_reconcile` never invents continuity. A process generation that cannot prove custody
  returns `outcome_unknown`.

Inputs and results are bounded independently from the transport frame. Unknown fields, unknown
methods, invalid references, stale authority and oversize values refuse before backend work.
Diagnostics use a closed code vocabulary and contain no credential, route, provider response,
media, or approval contents.
