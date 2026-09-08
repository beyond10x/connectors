# Adapter kind ownership and implementation boundary

`connectors.adapter/v1` belongs to this repository. `schema.json` defines its syntax;
`connectors-spec` enforces semantic checks and projects canonical descriptors.
These are Connectors documents, not inputs accepted directly by the ESS CLI.

The compiler validates configuration and operation JSON Schemas, identifiers,
unique operation IDs, and source references. It refuses unknown fields and
ambiguous duplicate JSON keys. Source URLs identify the official documentation
reviewed for this repository-authored declaration; they do not claim vendor
publication of these adapter documents. Source access was 2026-09-08.

The SHA-256 revision covers recursively canonicalized specification content.
JSON object ordering is independent of Cargo feature unification. Regeneration
and `--check` are explicit source maintenance operations. Ordinary builds consume
committed descriptors and never fetch source definitions.

Every declared operation is a handwritten binding obligation in this first
implementation. Adapter construction verifies the exact supported-handler set
before applying configuration narrowing. A compiler projection alone does not
prove that a provider call works; conformance and live evidence supply that proof.

`ess/` models the declaration entities and relations through the installed ESS
version. It does not claim to synthesize provider behavior. Automatic OpenAPI
import/lowering into executable ESS realizations remains a future authoring feature
from the full design, separate from this implemented three-adapter service slice.
No generic ESS extension API or generated production server is assumed.

## Shared configuration schemas

Both adapter versions accept these closed offline `$ref` imports inside
`configuration_schema`: `urn:connectors:config:v1:service`,
`urn:connectors:config:v1:http`, and `urn:connectors:config:v1:credential`.
Their single editable owner is the existing Rust `ServiceConfig`, `HttpConfig`
and `CredentialRef` types and schema annotations in `connectors-host`.
The compiler enables the host's optional `schema` feature to derive and inline
those schemas. Runtime adapter libraries do not gain a host dependency.

Imports never fetch a URL or file. Unknown library versions are refused, sibling
constraints remain intersections, and instance data in defaults/examples/consts
is not interpreted as a schema. Generated descriptors are self-contained JSON
Schemas. The v1 revision covers the expanded specification; v2 covers its authored
specification and expanded configuration schema. A shared type/schema change
therefore changes descriptor revisions and is caught by descriptor/bundle checks.
Existing inline schemas and the closed v1 document syntax remain supported.
