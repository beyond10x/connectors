# Design 04: the document carries the caller's contract

**Status:** landed with S-001 · **Date:** 2026-08-13 · **Story:** [S-001](../stories/S-001-the-document-carries-the-callers-contract.md)

The canonical document now stores, per operation, everything a caller or a model receives:
the caller-facing **symbol** beside every declared parameter, the model-facing **contract**
(`description` + lowered `input_schema`), the **`credential_requirement`** token, and — when a
provider declares one — the **`produces_credential`** minting join. This is the predecessor's
C-552, ported from the private predecessor migration set, plus the two schema gaps M1 found in code
(`table.rs`'s two derivations). Five decisions the story left open are recorded here.

## 1. The port is an application, not a re-derivation

The M1 import pinned `flux-connectors` at `3650a136` — the direct parent of the predecessor's
reviewed C-552 commit — so the diff applied to the surviving crates verbatim:
`connector-resolve/src/document.rs` byte-identically, the document builder and schema with only
its two engine call sites replaced. The allocator (`names.rs`) is pure string code and moved
whole into `connector_spec::names`, which now **owns** the wire-name → symbol mapping; the one
genuine engine dependency — `OpSpec::lower`'s input-schema projection — is restated engine-free
in `catalog-build/src/contract.rs` as the closed narrowing it always was.

**The one-time S-001 differential** (the §7.6 pattern, run against the predecessor's C-552
regenerated documents rather than its pack): all **835 operations** and **1518 parameter
symbols** — `symbol`, `contract.description`, `contract.input_schema` — match the
engine-derived values exactly. Effects were excluded; they are S-002's.

## 2. The S-001 release used schema 1; the coordinated axis wave is schema 2

S-001 itself was an additive release and stayed on schema 1. The later coordinated S-002/S-015/S-023
wave deliberately moved to schema 2: it removed the `quirks` object and old mixed `runtime` noun,
made effects and five execution axes required, and therefore must refuse old documents instead of
quietly defaulting facts used by authorization and dispatch. Readers and the resolver now require
those fields and refuse a different schema version before serving records.

## 3. The extended description belongs to the artifact

The design edge C-552 recorded: the error-envelope-extended description states host-envelope
behaviour ("a non-2xx response is returned as data…"), so does it belong in the artifact or in a
projection layer? **In the artifact.** The platform's consumers (grant admission, the invocation
surface, any model-facing projection) must build the contract from document data alone —
architecture §5's rule — and a projection layer that composes prose would be a second author of
the contract, exactly the two-derivations disease S-001 exists to end. The document stores what
the declaration states; an operation with an empty summary and no envelope stores the empty
description it has.

## 4. `credential_requirement` is data, in C-206's tokens

The document's `auth` list is the *effective* requirement, which collapses "declared `auth = []`"
into "declared nothing anywhere". The distinction only exists at build time, where
`Operation::auth`'s `Option` still holds it — so the build publishes it:
`declared` / `no-credential-required` / `no-credential`, the predecessor's published status
tokens. `table.rs` now **reads** the token; the derivation from the connector default is gone,
and the pair of documents it could not tell apart is pinned as a fixture test.

## 5. The minting join is carried, not deferred

S-001 allowed deferral with a recorded reason; carrying the join was smaller than recording why
not. The operation-level `produces_credential { credential, secret }` mirrors the provider TOML
block, costs zero bytes on every shipped document (none declares one), and makes
`Acquisition::Minted` constructible from document data — C-136's property, *a caller can use a
credential it can never read*, now reaches the typed views. Conflicting provenance (OAuth2 and a
mint on one credential, two mints of one credential, a mint of an undeclared credential) is
refused by name at table build.

## What S-002 now carries, and what remains

Per-operation host **effects** (the predecessor's fourth C-552 field) now ship as required declared
data with a closed vocabulary. Every existing HTTP operation explicitly states its directional
effect plus `network`; no consumer derives that pair. The separate `semantic_effects` tier remains
required and unchanged. S-002 is still blocked only on M2's grant-admission consumer and its
non-vacuous no-derivation test in `crates/domain`.

## 2026-09-06 amendment: source fidelity and catalog schema 3

The operator's source-fidelity requirement supersedes the historical projection choice for newly
migrated operations. The compiler retains literal vendor schemas and translates the supported
OpenAPI 3.0 Schema Object vocabulary into Draft 2020-12 caller schemas. Integer types, enums,
defaults, nullable values, unions, required fields and object constraints survive. Translation
does not apply defaults or replace an unsupported schema with `{}`. For example `nullable: true`
adds null only to an explicitly declared type; it does not add null to an enum or change `oneOf`
into `anyOf`. The [OpenAPI Schema Object rules](https://spec.openapis.org/oas/v3.0.3.html#schema-object)
govern this translation.

Schema 3 requires the closed `request_semantics` value on every operation. `legacy_v1` retains the
previous contract and request interpretation. `openapi_3_0_json_v1` supports complete JSON bodies,
default scalar path/query serialization and the explicitly supported schema subset. Missing or
unknown profiles are refusals. Unsupported source constructs remain diagnosed importer gaps,
including recursive response schemas rather than broadening their recursive tails.

A source body stays one `body` parameter carrying its whole schema. Request-body presence is
independent of required properties inside it. Omitted optional values stay absent, explicit null
is checked against the source contract, and JSON-looking strings remain strings. Logical path
parameter values are percent-encoded as data, separately from endpoint configuration validation.
The retained parameter position/name/symbol mapping makes this transport projection reversible;
body properties are not flattened or omitted. The build also stores translated `contract.output_schema`
when the source declares a supported successful response, leaving the literal `response_schema`
available as source evidence.

These are semantics a consumer must act on, so the producer, schema, pack reader and resolver move
together to schema 3. The new generated schema is `catalog/connector-document-v3.schema.json` with
its own identity. The old `catalog/connector-document.schema.json` remains byte-identical under
its schema-2 identity. Existing ConnectorOperation input/output schema fields already carry JSON
values; this change does not alter that frozen protocol's DTO shape.

The first source migration is the four GitLab pipeline-schedule operations. The complete pinned
1,847-operation source remains inventoried, including legacy and importer gaps. The official
OpenAPI describes objects where endpoint documentation shows arrays; those source schemas remain
objects. Its heterogeneous input array branch omits `items`, contrary to OpenAPI 3.0 structural
requirements. Both the literal and caller schemas retain the missing constraint, with an
operation-level source diagnostic. Preserving stated constraints does not certify source validity.
Credentials, selected Connections, grants and egress continue to enforce authority independently.
