# ConnectorOperation v0alpha3

Exact identity: `b10x.connector-operation.v0alpha3`. This additive alpha bundle is the
credential-free operation contract. Its reader is
[`operation::v3`](../../../crates/protocol/src/operation/v3.rs), with explicit
three-version transport adapters in
[`operation::versions`](../../../crates/protocol/src/operation/versions.rs).
The internal `operation::*` API continues to reexport v2. The complete v1 and v2
bundles, their readers and the v2 generator remain frozen.

Requests retain every v2 method and payload. Results retain every v2 result,
including advisory rate metadata and definite rate-limit refusals. Rate advice
does not authorize an invocation, choose a quota branch or resend work. The v2
RFC 3986 HTTPS source-URI grammar and integer bounds apply unchanged.

`OperationError.code` adds `authentication_required`. Exactly that code requires a
non-null `authentication` object containing `operation_ref`, `connection_ref`,
`integration_ref`, `auth_profile`, `need`, `attempt` and `next_action`. References
are nonempty printable ASCII, at most 512 bytes; the profile is 1–128 lowercase
ASCII letters, digits, `.`, `_` or `-`. `need` is `authorize_configured` or
`reauthorize_existing`; `attempt` is `not_attempted`; `next_action` is
`start_trusted_remediation`. This error requires `status:error`, no result,
`retriable:false` and no retry delay. Other codes forbid the authentication field,
including explicit null. Unknown fields and enum values refuse.

The safe payload has no URL, session capability, provider code, registration,
credential, input or execution authority. Emitting it after authentication,
admission and credential preflight is the receiver's obligation. These DTOs do
not perform those checks or create a session. Trusted remediation is a separate
Connection v2 action; completing it requires a fresh description and a new,
explicit invocation. No adapter performs fallback, dispatch or resend.

Projection to requested v1 or v2 replaces an authentication error with exactly
`unavailable`, message `The operation is unavailable.`, `retriable:false`, and
no authentication object or retry delay. It drops the original message too.
Ordinary and rate responses use the unchanged v2-to-v1 projection. Unknown
identities refuse; dispatch always parses the selected strict DTO from the
original bytes, preserving duplicate-field rejection.

The schema is generated from the typed v3 graph and the unchanged v2 constraints.
Every inherited definition except the versioned envelopes and extended error is
equal to v2. As in v2, serialized UTF-8 input/frame/result budgets and derived
rate-interval arithmetic remain reader checks: JSON Schema length counts Unicode
code points and cannot measure serialized JSON bytes or calculate that interval.
The vectors state Rust and Draft 2020-12 outcomes independently, retaining all
85 predecessor cases with only the supported identity changed. Duplicate JSON
keys are covered by original-byte tests rather than JSON object vectors.

The manifest pins this README, schema, vector schema, vectors and the unchanged
artifact-bundle schema. Reproduce deterministically with:

```sh
cargo run --locked -p protocol --example operation_v3_bundle -- write
cargo run --locked -p protocol --example operation_v3_bundle -- check
```

Reader/producer adoption remains a coordinated migration: readers first, producers
after compatibility is verified, SDK before embedding consumers. This bundle
implements the contract slice from [design 22](../../../docs/design/22-authentication-remediation.md);
it makes no runtime wiring, external adoption or architecture-acceptance claim.
