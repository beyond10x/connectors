# protocol

Strict versioned wire DTOs and executable conformance readers. The semantic owner is the
protocol-neutral `VoiceSession` bundle under `contracts/`; RTVBP names remain outside this crate.

## Operation versions

[`operation`](src/operation.rs) exposes ConnectorOperation v0alpha2 types and an explicit
[version adapter](src/operation/wire.rs). Local and hosted operation boundaries serve
`b10x.connector-operation.v0alpha1` and `b10x.connector-operation.v0alpha2` through the same
admission and backend owners. They validate the declared identity and request before dispatch,
refuse unknown versions, and encode operation replies in the requested supported version.

The [v0alpha2 bundle](../../contracts/connector-operation/v0alpha2/README.md) covers the complete
request/response vocabulary, including structured rate limits. A definite provider HTTP 429 is
`rate_limited`; optional `retry_after_seconds` is an unsigned 64-bit number, including zero, legal
only on that error code. Only one admitted numeric Retry-After header supplies the delay.
Missing, duplicate, malformed or HTTP-date values supply none. Provider headers and error bodies
are not echoed. An uncertain mutating outcome remains `outcome_unknown`.

Descriptions may carry `rate_advice`: the unchanged fixed declaration and conditional alternatives
with applicability text, a source URL and an optional published rate. `minimum_allowance` and
`ceiling` retain their different meanings. A numeric alternative includes spacing advice
`ceil(per_seconds * 1000 / requests)` milliseconds. Callers receive every alternative; credentials
do not determine an application category. Advice does not schedule calls or authorize a retry.

The [frozen v0alpha1 bundle](../../contracts/connector-operation/v0alpha1/README.md) remains
byte-identical. V1 replies omit rate advice and retry delay, and map `rate_limited` to legacy
`unavailable`, preserving its message and retriable flag. The deployed v1 reader also has existing
`purpose` and `session_signal` extensions absent from the frozen schema; compatibility tests keep
that distinction explicit. Adding v2 fields to a v1 envelope is not a supported extension path.

Upgrade a local CLI and daemon together. Version mismatch, a rate refusal or an uncertain result
does not trigger automatic invocation fallback or resend. Independently pinned consumers may keep
using v1; these source contracts do not establish their adoption of v2.

## Conformance and catalog compatibility

The v2 generator validates reviewed vectors independently against Rust and Draft 2020-12, including
URI-format assertions. Exact interval arithmetic and serialized UTF-8 byte budgets remain explicit
reader checks. From the repository root, check the generated schema and manifest without writing:

```text
cargo run --locked -p protocol --example operation_v2_bundle -- check
cargo test --locked -p protocol
```

Canonical catalog schema 3 is a separate version boundary. Its producer, generated documents/pack,
typed reader and resolver move together; external pack users must update their reader/resolver
before loading a newer pack. Richer input/output schemas do not alone change an operation wire
identity. See the [caller-contract amendment](../../docs/design/04-the-callers-contract.md#2026-09-06-amendment-source-fidelity-and-catalog-schema-3)
for source fidelity and the [public interface guidance](../../docs/architecture/interfaces.md#operation-versions-and-rate-advice)
for caller behavior.
