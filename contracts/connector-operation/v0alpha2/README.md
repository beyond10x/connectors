# ConnectorOperation v0alpha2

This additive contract identifies credential-free operation requests and responses as
`b10x.connector-operation.v0alpha2`. The schema covers every deployed request and result,
including `session_signal`, connection `purpose`, and the new structured rate-limit facts.

A definite provider HTTP 429 is `rate_limited`. Its optional `retry_after_seconds` is an unsigned
64-bit delta, including zero, and is legal only on that error code. Absence establishes no delay.
Only one numeric Retry-After header, trimmed of ASCII space and tab, is trusted; duplicates,
signed/fractional/comma-joined/overflowing values and HTTP dates supply no delay. Raw provider
headers and bodies are not error messages. An uncertain mutating outcome remains `outcome_unknown`.
Neither version negotiation nor a retriable refusal authorizes automatic invocation resends.

An operation description may include `rate_advice`, with an unchanged fixed declaration shape
and bounded conditional alternatives. Each alternative preserves its `declaration`:
`applies_when`, optional `rate` (`requests`, `per_seconds`, `basis`), and `source_url`.
Sources use RFC 3986 ASCII URI spelling with literal lowercase `https`, a nonempty host,
no userinfo or fragment, and an optional decimal port from 0 through 65535. An empty port
denotes the default. Leading zeros and percent-encoded spelling are preserved; spaces and
non-ASCII path/query characters must be percent-encoded. The readers do not normalize URLs.
`minimum_allowance` is a published minimum tier allowance; `ceiling` is a published maximum.
No numeric rate means that source establishes none. Applicability is explanatory data; a runtime
must expose all alternatives and must not infer an application category from credentials.
When a rate is present its advisory interval is exactly
`ceil(per_seconds * 1000 / requests)` milliseconds. This is spacing advice, not automatic pacing
or a promised burst ceiling. Selected vendor request and response schemas are preserved verbatim.

The standalone Draft 2020-12 schema is projected from the Rust DTO graph and adds the reader's
structural, numeric and conditional presence constraints. Exact cross-property interval arithmetic
and serialized UTF-8 byte budgets require the Rust validator; JSON Schema cannot express those
comparisons. Vectors declare independent `valid` (Rust) and `schema_valid` outcomes and explain
intentional differences. Both validators exercise requests, responses, rate alternatives,
all error codes, malformed envelopes and existing session semantics.

The explicit adapter decodes v0alpha1 and v0alpha2 and encodes replies in the requested version.
Transport integration must retain one admission/backend path. V1 projection loses rate advice
and retry delay, mapping `rate_limited`
to legacy `unavailable` while preserving its message and retriable flag. Other errors, authority
context, schemas, purpose and session signals retain deployed semantics. The v0alpha1 schema and
all its bundle bytes remain frozen: deployed v1 purpose/session_signal extensions are deliberately
snapshotted separately and are not claimed to conform to that older schema.

Reproduce the v2 schema and manifest with:

```text
cargo run --locked -p protocol --example operation_v2_bundle -- write
cargo run --locked -p protocol --example operation_v2_bundle -- check
cargo test --locked -p protocol
```

The generator reads reviewed README/vector inputs, validates both readers, and renders only the
v0alpha2 schema and manifest. Its explicit check mode refuses drift without writing files.
No release, deployed consumer upgrade or retirement of v0alpha1 is implied by this bundle.
