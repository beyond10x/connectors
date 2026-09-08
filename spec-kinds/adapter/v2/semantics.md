# Connectors adapter specification v2

Connectors owns `connectors.adapter/v2`, its [closed schema](schema.json), and the
Rust frontend in `crates/connectors-spec`. v1 retains its closed syntax and existing inline-schema support.
Both versions support the [shared offline configuration imports](../v1/semantics.md#shared-configuration-schemas).
The authored declaration values have a typed home in
[ess/domains/declarations.yaml](../../../ess/domains/declarations.yaml).
`UpstreamSource` and `RequestMapping` are embedded declaration values, not provider
resources with invented identities or lifecycles. The existing adapter-to-operation
ownership relation remains authoritative. The compiler checks each mapping's
`operation` against exactly one owned operation declaration.

The first generation profile supports selected OpenAPI 3.0/3.1 GET operations,
individually encoded path segments, scalar query parameters, and closed local
request objects containing strings, integers, and optional nullable strings.
The upstream declaration records a repository-relative path, HTTPS source URL
containing its exact Git revision, SHA-256 of the original bytes, and API base path.
Generation is offline: it reads and verifies the source; it never refreshes it.

Each operation has exactly one mapping. Parameters come from typed caller input,
a declared constant, or an explicit `prepare` binding. `prepare` and `finish` are
required methods with no default implementation. Unknown fields, absent mappings,
unknown inputs, unsupported encodings/types, unbound required source parameters,
source digest changes and unresolved source references are refusals. Constrained
source scalars require explicitly matching caller constraints; this conservative
profile does not attempt general JSON Schema subset inference. Binding parameters
are limited to unconstrained string/integer source scalars; their preparation and
value validity remain explicit handwritten obligations.

The frontend retains the exact selected operations, referenced response schemas,
source pointers, excluded optional parameters, and implementation obligations in
`upstream.openapi.json` and `coverage.json`. Unselected operations create no runtime
capability. Source authentication declarations never supply credentials or admission.

The pinned ESS release cannot directly import this GitLab OpenAPI 3.0 document. The complete
attempt and refusals remain in `ess-import.json`; that file is not a successful ESS
import. Connectors validates its supported source mappings and lowers the selected
local request types into an `ess/1` domain and an owning component. ESS validates,
compiles and synthesizes those types. The generated Rust dispatch constructs and
uses them, validates the full local input schema, calls `prepare`, builds the
provider request, calls the injected authenticated transport, and calls `finish`.
The generated handler set is checked against the service descriptor at construction.
The asynchronous Connectors host owns the outward wire service and infrastructure.
ESS's component skeleton is retained as synthesis evidence; it is not the HTTP server.

Response interpretation, pagination, current admission and provenance remain
handwritten. GitLab's selected issue-list source schema describes one issue and its
file-read source has no response schema. These facts are preserved in the source;
no source correction or external GitLab lifecycle is invented. The existing service
contract and provider conformance tests govern those response obligations.

A manifest records exact source bytes, the pinned ESS version, rustfmt version, and
every generated file digest. The pipeline runs rustfmt over ESS-synthesized Rust
before recording those digests, so Cargo formatting and regeneration agree.
Identical inputs/toolchain produce identical bundles.
`--check` compares all outputs and the manifest. Regeneration preflights ownership
and paths, refuses symlinks, preserves unowned files, and retires only paths in the
previous manifest. Initial v1 migration may adopt only the descriptor whose bytes
exactly match the unchanged v1 declaration fields. I/O failure is still a failed
run; regeneration can repair owned partial output after storage is restored.

Normal Cargo builds consume the checked-in generated files and do not invoke ESS,
rustfmt, or vendor access. Regeneration and the generation tests require the pinned
ESS and recorded rustfmt toolchain. `TMPDIR` should name a writable directory with
space for the temporary bundle. No entity or relation outside this configured
service profile is implied; SaaS ownership cardinality remains UNMAPPED in ESS.
