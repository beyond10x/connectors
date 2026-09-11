---
format: aep.planning-md/1
id: specification:catalog-http-runtime-handoff
kind: specification
status: draft
title: Catalog, OpenAPI import and generic HTTP template runtime handoff
relations:
- derived_from: architecture-decision-record:declarative-http-provider-runtime
- specifies: initiative:complete-local-connectors
- informed_by: specification:core-model-closure-20260909
revision: 1
---
## Outcome

Deliver ordinary HTTP providers through reusable authenticated execution of
deterministically generated templates, while the existing implementation track
continues through the distinct Kubernetes, PostgreSQL and MCP execution semantics.
This specification applies
`architecture-decision-record:declarative-http-provider-runtime` within
`initiative:complete-local-connectors`.

## Handoff from the current implementation session

Owner: session `01a08aa9-247d-7da0-9115-74df052a8a55`, currently implementing the
local GitLab and shared mutation/auth foundation.

1. Finish a bounded, usable HTTP/auth checkpoint; run the checks required for its
   actual changed surface and retain exact evidence and remaining limitations.
2. Commit the verified work and integrate it on `main` under repository bot
   authority. Report the full commit ID, capability/IPC/contract versions, public
   interfaces the catalog runtime should consume, evidence paths and known gaps.
3. Preserve reusable connection/auth capability, approval, audit, mutation
   preparation/dispatch and recovery ownership. Document what is still
   GitLab-specific or needs a separately reviewed extraction; do not claim the
   current PUT-only write generator is a complete HTTP-template runtime.
4. Continue Kubernetes including the selected Helm behavior, then PostgreSQL,
   then MCP. Do not expand GitLab endpoint-by-endpoint in order to cover its full
   vendor specification, and do not wait for the catalog track to finish.
5. Retain open GitLab sandbox and native workflow obligations with accurate states.
   This checkpoint does not prove provider completion or satisfy missing evidence.
   Follow existing release instructions whenever cutting a release.

The sender creates these resources in a managed checkout; the existing primary
writer remains the sole integration-store writer. It should import the two
resources via `aep plan artifact`, record the operator's decision, and update the
initiative's current delivery section to reference the ADR. Preserve all dated
historical checkpoints. The sender will provide body paths and exact artifact IDs
in the authorized session message; replay the AEP commands instead of manually
resolving divergent journal content.

## Catalog and OpenAPI track

Owner: the session recording this handoff, in a managed worktree. Implementation
begins from the reported HTTP/auth checkpoint, with importer/design work allowed
independently before that checkpoint where it does not touch the runtime owner's
files.

Build a deterministic ingest and coverage pipeline for the complete pinned
GitLab OpenAPI inventory, then prove reusable execution with a second ordinary
HTTP provider using its own auth profile and declarations. Select that fixture
provider from available source and local acceptance evidence during decomposition;
this record does not invent its auth requirements or claim sandbox access.

Support OpenAPI 3.0 and 3.1 explicitly. Preserve original source bytes, revision,
digest, license and any named transformation. The recorded pinned ESS refusal
must be closed through proper importer/toolchain support or verified normalization;
changing only the document's version string is invalid. Generic OpenAPI support
belongs in the appropriate reusable tooling owner; Connectors-specific curation
and auth/runtime bindings remain in Connectors.

Produce per-provider bundles containing the complete operation inventory,
supported request/response schema and serialization data, auth-profile references,
reviewed curation and named unsupported/unresolved cases. Load verified bundles
on demand through the catalog adapter's generic HTTP realization, using a local
index without requiring a central catalog service. Compression is allowed;
container, compatibility and cache details require an explicit implementation
selection before claiming them supported.

Use a bounded, declarative runtime rather than arbitrary code inside templates.
Build mechanical endpoint definitions from source; author only reusable rules and
reviewed exceptions. Optional deterministic model/SDK generation shares the same
normalized facts. Full inventory availability does not grant unrestricted
invocation, skip mutation controls or promise support for missing source semantics.

## Lightweight local TOML input

The same track owns the operator's generic HTTP action input: a small TOML
definition in user-local configuration or the current project directory. Compile
it into the same complete validated template/profile and execute it through the
same engine as imported operations. OpenAPI is not required for an authored action.
Apply shared defaults and explicit overrides for method/target, typed inputs,
parameter/body serialization, response handling, bounds and auth-profile selection.

Reuse the platform's authentication profiles, protected credential setup/custody,
connect, verification, repair, and acquisition/refresh flows where the selected
profile requires them. Support provider-specific declarative auth configuration,
including supported custom header/query placement and OAuth parameters. Missing
auth mechanisms or flows remain named platform implementation work; do not
silently offer local actions a separate, weaker authentication implementation.
New algorithms use reusable reviewed auth mechanisms, not embedded shell code.

Specify user/project file discovery, explicit selection, precedence/conflict
handling, configuration schema and validation before implementing the loader.
Project discovery grants no execution or credential authority and starts no auth
flow by itself. Keep secrets out of action files and preserve the existing
protected input and connection-use rules. Exact filename and CLI/TOML syntax are
deliberately not invented as a current supported interface in this record.

The authored action maps to the existing OperationDeclaration and AuthProfile
model, rather than creating an additional runtime entity. Model any genuinely
missing configuration semantics through ESS before decomposing dependent work.

## Scope and coordination

Existing typed homes are
[artifact provenance](../../../ess/domains/artifact_provenance.yaml) and
[adapter/operation declarations](../../../ess/domains/declarations.yaml).
Existing design owners are the
[catalog contract](../../../contracts/catalog/v1alpha1/semantics.md),
[catalog adapter](../../../adapters/catalog/design.md),
[auth capability contract](../../../contracts/auth/capability/v1alpha1/semantics.md)
and [local mutation binding](../../../contracts/service/local-mutations.md).
No new runtime entities, guessed ownership/cardinality, storage lifecycle or
strict codec extensions are selected by this scheduling record.

The runtime track owns changes under `crates/connectors-host/`, the current SDK
capabilities, local CLI/IPC and its native GitLab obligations until handoff. The
catalog track's intended surfaces are `adapters/catalog/`, the generic HTTP engine
named in that design, importer/bundle tooling in `crates/connectors-spec/` and
`crates/connectors-build/`, and its owning contracts/models. These are planning
boundaries, not evidence that the generic engine already exists. Compiler,
toolchain, root manifests and SDK changes can overlap: coordinate and serialize
those edits explicitly after the checkpoint rather than assuming file isolation.

Keep one writer for the planning store being integrated. Draft implementation
stories and run the required decomposition reviews only once their affected
interfaces, supported constructs and shared-file ownership are concrete. This
record creates no implementation-story decomposition and no concurrent dispatch.

## Acceptance for subsequent implementation

- Every operation in the pinned source is represented in inventory/coverage with
  an executable or explicit unsupported/unresolved disposition and source pointer.
  A selected handful is not a complete inventory acceptance result.
- Supported ordinary operations invoke through the same runtime with no new
  per-operation handwritten Rust prepare/finish handler. Adding a supported
  endpoint changes source/declarations and deterministic output only.
- At least two ordinary HTTP provider bundles run with their own auth profiles
  through the same execution mechanics; provenance and provider boundaries hold.
- A user-local and a project-local TOML action each compile and invoke through
  that same engine without a vendor specification or per-action Rust. Tests prove
  defaults/overrides, selection/precedence and named invalid-definition refusals.
- Imported and TOML-authored actions share equivalent auth behavior for each
  supported platform profile/flow, including protected setup, restart reuse and
  applicable acquisition/refresh cases. Missing mechanisms remain explicit gaps;
  discovery alone starts no flow or request, and files contain no credential values.
- Declared input/body serialization, response media types, pagination and errors
  have applicable fixture evidence; unsupported cases refuse with named coverage.
- Applicable auth, scope, approval, audit, dispatch and recovery conformance remains
  effective for template execution, including conservative uncertain write outcomes.
- Identical pinned inputs/toolchains produce identical generated bundle contents;
  invalid digests and unsupported format versions refuse, and normal generation
  does not refresh vendor sources. Test bounded on-demand loading separately.
- Final claims distinguish complete inventory, executable support, fixture coverage
  and real-provider acceptance. Documentation and plan state reflect those facts.

This specification records intended work and the handoff, not implementation or
test-result evidence. The initial planning status remains unchanged until its own
requirements are actually satisfied.
