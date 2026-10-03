# Protected local operation curation

This selects the metadata source for the local governed projection. The shared
value declarations are in `ess/domains/service_wire.yaml`. Full MCP serving and
the private execution budget binding remain unfinished.

The receiver reads `CONFIG.operation-curation.json`, beside its selected local
configuration. Only the admitted local owner opens it, using the existing bounded
private-file checks: regular file, owner-only permissions, no symlink or hardlink.
No MCP input, operation input or projection flag selects a different source.
Failure to read required policy is an unavailable/invalid-configuration refusal,
never a successful empty catalog. Reading requires no provider, credential probe,
connection acquisition or approval spend.

The closed JSON document has format `connectors-operation-curation/1` and an
`adapters` array. Every entry contains a unique `adapter_alias`, the exact
`executable_selection`, `bootstrap_sha256`, and an `operations` array. The first
pin is the existing configured adapter selection digest; the second is the shared
canonical JSON digest of the original validated cached bootstrap, before any
permission filtering. Both are lowercase SHA-256 hex. Neither pin substitutes for
current permissions or live readiness. The whole file is limited to 1 MiB, with
at most 1,000 adapters and 256 operations per adapter. Duplicate object members,
selectors or operation ids, unknown members, null in place of required values and
unknown formats are refused. An empty adapter/operation array is explicit policy.

An operation entry is `{operation, metadata}`. `metadata` uses the complete shared
OperationMetadata shape in [compatibility](compatibility.md#4-extended-descriptor-and-invocation-surface).
Every entry must name an operation in the pinned original descriptor. Decode it
against that operation's exact profile and compare executable write effect with
the private requirement. Public auth alternatives, when present, must name the
selected private profile and the exact required scopes; absence does not mean
anonymous access. Unsupported or inconsistent declarations refuse selection.
No field is inferred from an operation name, description, HTTP method or the
private Read/Write flag. Risk, semantic effects and repeat semantics must be
curated from the native contract and reviewed implementation.

This file is declaration input, not proof that every declaration is implemented.
Advertisement additionally requires the selected native binding to implement the
profile, effects, idempotency, approval and every advertised limit. In particular,
the legacy private transport does not acquire extended budget support by reading
this file. A well-formed unbound declaration remains unavailable. An operation
absent from curation is unbound to the extended projection; it is not assigned
defaults. The existing legacy descriptors and cached bootstrap shape retain
their meaning and closed codecs.

Composition captures the native projection configuration, curation, original
bootstrap and adapter selection as one projection snapshot. Its revision covers
all of them. The owner rechecks that snapshot after audit admission, filters both
operations and metadata by current metadata permissions, and returns only the
admitted values on its authenticated same-build private socket. It never exposes
the curation pins or original private bootstrap directly to MCP. Execution repeats
current lookup/scope policy, the complete revision comparison, binding/enablement
and final dispatch admission in their existing order. Curation changes invalidate
old projection revisions even when the legacy descriptor revision stays unchanged.

The private `ProjectedDescribe` carrier includes the required `operation_metadata`
object beside `bootstrap` and `projection_revision`, because its owner connection
already requires the exact same build. This
does not extend the closed legacy Bootstrap, public Descriptor or service response
codecs. Old builds refuse the owner handshake before consuming a new carrier.
