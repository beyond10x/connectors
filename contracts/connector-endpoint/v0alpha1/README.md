# ConnectorEndpoint v0alpha1

`b10x.connector-endpoint.v0alpha1` is the shared credential-free endpoint inventory contract.
Local clients send one bounded JSON frame on the owner-only daemon socket; hosted clients POST
the same envelope to `/endpoints` with independently verified Identity authority.

The closed methods are `list`, `show`, `refresh`, and `bind`. Lists use opaque cursors and report
partial-source warnings. Endpoint identity incorporates the immutable resource UID and interface;
resource names and labels neither grant access nor choose credentials.

`list` and `show` require catalog-read authority and remain scoped by source policy. `refresh` and
`bind` require connection-management authority and hosted operator policy. Credential bindings
contain exact Secret references and key mappings, never values. Direct routes are explicit
operator bindings. SQL requires TLS unless the operator explicitly selects `disabled`.

Inventory readiness is advisory. Invocation revalidates source policy and immutable resource
identity, then follows ordinary Connection and Grant admission. Discovery reads no credential
values and starts no provider calls, subscriptions, or tunnels.

Requests are limited to 65,536 bytes and responses to 524,288 bytes. List pages contain at most
100 endpoints. Clients must validate the original frame, response correlation, result method,
and exact endpoint identity. An invalid frame is refused before backend access.

The schema is projected from the Rust contract. Regenerate or verify the bundle with
`cargo run --locked -p protocol --example endpoint-contracts -- --write` or the same command
without `--write`. The repository tests cover structural agreement and the live authorization
boundary. Runtime byte budgets and source policy are additional constraints.
