# ConnectorOperation v0alpha4

`b10x.connector-operation.v0alpha4` adds endpoint selection without changing provider operation
identities or their input/output schemas. Local and hosted clients select this identity explicitly;
there is no protocol negotiation or retry through another version.

`invoke` requires exactly one non-null `endpoint_ref` or `connection_ref`. `describe` accepts one
target, or neither when inspecting the operation catalog. Targets are envelope fields outside
provider input. The other operation methods and result shapes retain v0alpha3 semantics.

The runtime resolves an endpoint into current non-secret Connection metadata, after verifying
caller and source policy. A targeted description contains exactly that admitted Connection and
the ordinary description lease. Invocation revalidates the endpoint, then passes through the
existing lease, Grant, approval, and credential checks. Credentials and routes are resolved only
after admission. A resource replacement cannot silently retarget an old endpoint reference.

Description references remain opaque. Clients obtain a fresh target-aware description before
invoking; stale authority requires another description and never an automatic operation replay.
The existing frame, input, result, and authentication-response bounds remain in force.

Predecessor readers and immutable bundles remain unchanged and refuse endpoint fields. Public
discovery clients use ConnectorEndpoint directly instead of candidate/observation materialization.

The schema is projected from the Rust contract. Regenerate or verify the bundle with
`cargo run --locked -p protocol --example endpoint-contracts -- --write` or the same command
without `--write`. Runtime authorization and serialized byte budgets are additional constraints.
