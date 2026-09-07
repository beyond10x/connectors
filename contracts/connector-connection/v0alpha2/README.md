# ConnectorConnection v0alpha2

Exact identity: `b10x.connector-connection.v0alpha2`. The strict reader and pure
version adapters are in
[`connection_v2`](../../../crates/protocol/src/connection_v2.rs). The complete
v1 bundle remains unchanged. Ordinary commands, results, errors and trusted
session validation reuse the v1 reader losslessly, including the pending status
compatibility correction documented below.

Three commands are additive:

| Method | Required parameters |
| --- | --- |
| `remediation_start` | `operation_ref`, `connection_ref`, `input` |
| `remediation_status` | `connect_session_ref` |
| `remediation_acknowledge` | `connect_session_ref`, `operation_ref`, `connection_ref` |

Start's serialized input is at most 64 KiB. A v2 request frame is at most 128 KiB;
selected v1 requests retain their 64 KiB bound. Responses remain at most 128 KiB.
Bound parameters contain no integration, profile, scope set, credential, label,
route, callback or additional authority. The receiver derives these facts from
the exact configured target. Conversion of any bound request or result to v1
returns a typed non-retriable `protocol` refusal; it cannot create an unbound
session. Unknown identities refuse without fallback.

Start and status return `BoundRemediationStatus`: `connect_session_ref`,
`operation_ref`, `connection_ref`, `integration_ref`, `auth_profile`, `need`,
`session_state`, `expires_at_unix_ms`, `resume_state` and nested `session` of the
existing `ConnectSessionStatus` type. The duplicated session reference,
integration, state and expiry must match exactly. A completed nested session
must name the bound Connection. Pending requires a pending session; Ready and
Consumed require Completed. Expired permits Expired or Completed, since
publication can win before the acknowledgement deadline expires. Failed requires
Failed. Every terminal projection has no endpoint or browser capability.

Only this trusted management response may contain the existing pending
`completion_endpoint` or `browser_completion_url`. The inherited v1 validator still
checks its WHATWG-parsed local/hosted route and fragment capability grammar.
This trusted DTO is unsuitable for direct model/MCP output.

Pending `connect_session_status` and `remediation_status` responses may omit both
completion locators: polling does not reissue a one-use capability. Pending
`connect_session_create` and `remediation_start` responses still require at least
one usable completion route. Any supplied locator retains its existing validation,
and terminal responses retain their endpoint and Connection-reference constraints.
This corrects the v1 reader's conflation of creation with polling to admit the
existing hosted status response; it changes no v1 schema, field or lifecycle state.

Acknowledgement returns only `connect_session_ref`, `operation_ref`,
`connection_ref` and `next_action:fresh_description_then_explicit_invoke`. It is
not execution authority and contains no input or description lease. `conflict`
remains the typed error for an unusable acknowledgement. Current principal,
grant, exact-target readiness, expiry, one-use acknowledgement and zero dispatch
are later service obligations; a parsed DTO proves none of them. The adapter
stores no input and never invokes or resends an operation.

The deterministic typed schema covers every ordinary and bound variant,
closed fields, numeric bounds, reference/profile grammar and lifecycle/endpoint
conditions. Its specific reader-only checks are serialized UTF-8 input/frame/
response budgets, byte rather than code-point string limits, equality of
independently supplied references/expiry, and the inherited WHATWG URL parser's
route/capability checks. Draft 2020-12 has no instance-data equality operator;
ordinary URI format assertions do not implement WHATWG normalization. Vectors
name these limits and record schema and reader outcomes independently. No schema
annotation substitutes for runtime validation. Selected v1 compatibility follows
the deployed strict reader, including its known differences from the frozen v1
schema. Duplicate fields are tested on original bytes.

The manifest pins this README, generated schema, vector schema, reviewed vectors
and unchanged artifact-bundle schema. Reproduce with:

```sh
cargo run --locked -p protocol --example connection_v2_bundle -- write
cargo run --locked -p protocol --example connection_v2_bundle -- check
```

This is the additive contract slice from
[design 22](../../../docs/design/22-authentication-remediation.md). Reader support
precedes producer adoption; SDK precedes embedding consumers. Runtime, trusted
clients, model-output reduction and external adoption remain separate work.
