---
format: aep.planning-md/1
id: architecture-decision-record:declarative-http-provider-runtime
kind: architecture-decision-record
status: accepted
title: Ordinary HTTP providers run declarative templates with provider auth profiles
relations:
- decides: initiative:complete-local-connectors
- informed_by: specification:contract-driven-connectors-design
revision: 2
---
## Decision and authority

On 2026-09-11 the operator clarified that GitLab → Kubernetes → PostgreSQL → MCP
was chosen to exercise different execution semantics. Once ordinary HTTP execution
and its authentication boundary are implemented, further ordinary HTTP providers
must use declarative templates and provider-owned authentication profiles. They
must not require a new handwritten or LLM-authored Rust handler for each endpoint.

The operator directed the existing implementation session
`01a08aa9-247d-7da0-9115-74df052a8a55` to commit its verified HTTP semantics and
authentication work, integrate that usable foundation on `main`, and continue with
the remaining execution families. The catalog/OpenAPI session owns the separate
catalog, importer and template-runtime work. The operator explicitly authorized
sending this handoff after creating these planning resources.

This is an interactive, operator-selected architecture and scheduling decision.
Acceptance of this ADR records that decision; it does not establish runtime
completion. No approval bypass is used.

## Architecture

Pinned provider source plus reviewed declarative overrides is compiled
deterministically into a versioned provider bundle. One reusable Rust HTTP runtime
loads a selected bundle on demand and executes its validated request templates
through host-admitted capabilities. Provider-specific authentication profiles
declare requirements and select reusable authentication mechanisms; credentials,
connection admission and mutation authority retain their existing owners.

Bundles may be compressed per provider and expanded or decoded on demand, with
digest verification and bounded loading. The exact container/compression format
remains a catalog implementation decision. A plain local index is sufficient;
running a central catalog service is optional. Generated endpoint definitions are
data, not arbitrary scripts or caller-supplied methods, URLs or credential headers.

Import endpoint names, parameter and body serialization, schemas and supported
response mappings mechanically from OpenAPI. Use reusable defaults, provider
profiles and reviewed declarative exceptions for missing or inaccurate source
facts. Do not replace endpoint-specific Rust with manually retyping every endpoint
into templates. The full source inventory must be accounted for, including named
unsupported constructs and unresolved semantics. Inventory, executable support and
current caller authorization are distinct facts.

Generate models, schemas or optional typed client bindings through deterministic
tooling where useful. Bulk generation of per-endpoint Rust is not required for
catalog invocation. The current mandatory per-operation prepare/finish methods
are suitable for explicit native obligations, not the default for ordinary HTTP.

Shared HTTP execution includes supported serialization and response handling,
bounded transport, authentication capability use, and the existing approval,
audit, attempt, dispatch and recovery rules for mutations. An imported HTTP method
alone does not establish risk, retry safety, effect knowledge or permission.
Provider-specific auth handshakes may require reusable mechanism implementations;
declaring a profile does not imply every auth scheme is already supported.

## Lightweight authored HTTP actions

The operator additionally selected a generic HTTP action defined through a small
TOML file in user-local configuration or the current project directory. OpenAPI
is optional for this input path. The compiler expands the authored method, target,
parameters/body, response behavior and auth-profile selection into the same full
validated HTTP profile/template used by imported operations. Shared defaults keep
simple definitions short; explicit overrides retain their declared meaning.

These are authored operation declarations, not a separate execution engine or a
new entity. User-defined HTTP actions use all reusable authentication mechanisms
and flows provided by the platform, including its normal connect, protected
credential entry, custody, verification, repair and supported acquisition/refresh
journeys. Custom auth means provider-specific declarative configuration of those
mechanisms, such as a named header, token placement or OAuth profile parameters.
An extension requiring a new signing or handshake algorithm belongs in a reusable
auth mechanism with its own implementation and evidence, rather than an inline
script repeated per action.

The file contains non-secret requirements and references; credentials enter and
remain under the existing protected custody boundary. The action definition does
not itself grant connection use, enable writes or authorize an OAuth exchange.
User-local and project-local definitions must have explicit discovery, selection,
conflict and precedence rules. Project discovery alone must not start a flow or
dispatch a request. The exact TOML filename/schema and these loading rules are
selected and validated during implementation, with convenient explicit selection
available; this ADR does not claim a currently supported CLI syntax.

Every platform auth capability intended for this path needs equivalent behavior
for imported and authored operations. Current implementation gaps remain explicit
work; they must not become a second reduced-auth path for local TOML actions.

Higher-level native workflows retain their explicit semantics and reuse common
HTTP mechanics. Kubernetes additionally needs discovery and execution/session
behavior; PostgreSQL needs database protocol and read-only transaction semantics;
MCP needs its protocol, transports and capability lifecycle. Kubernetes REST calls
can reuse HTTP without reducing its complete behavior to unary templates.

## Ownership and sequencing

The existing implementation session owns the current host/SDK/local CLI/auth
foundation and its verified integration, followed by Kubernetes including the
selected Helm workflows, PostgreSQL, then MCP. The catalog session owns the
OpenAPI importer, provider bundles/index and generic template execution through
the agreed capabilities. The exact handoff is specified by
`specification:catalog-http-runtime-handoff`.

This scheduling decision supersedes the requirement in earlier initiative prose
to finish every selected GitLab workflow before starting the next execution
family. Once the reusable HTTP/auth foundation has a verified, usable `main`
checkpoint, progression may continue while catalog coverage advances separately.
Full GitLab API coverage, catalog completion and unresolved GitLab-specific
workflows are not prerequisites for starting Kubernetes. Existing workflow and
sandbox obligations remain recorded and open until actually satisfied; they are
not silently cancelled or marked implemented by this handoff.

Keep one planning-store writer on integration `main`. Separate implementation
tracks use managed worktrees and separate build outputs. Import this decision and
its handoff through AEP at the primary writer's next checkpoint, then reconcile
the initiative's current sequencing through AEP without rewriting historical
evidence. Do not merge conflicting planning journals by editing them manually.

A source integration checkpoint is not a claim of a cut release. Existing release
instructions still govern any source release; this ADR does not authorize a new
deployment, package publication, Atlas registration or new source remote.

## Existing ownership and evidence

- [Catalog contract](../../../contracts/catalog/v1alpha1/semantics.md), especially
  sections 8–10, already proposes generic execution and native semantic bindings.
- [Catalog adapter design](../../../adapters/catalog/design.md) owns the generic
  engine and provider-bundle realization. It remains unimplemented at this decision.
- [Artifact provenance ESS](../../../ess/domains/artifact_provenance.yaml) already
  declares Source, Bundle and Curation; [declarations](../../../ess/domains/declarations.yaml)
  own adapter and operation declarations. This ADR introduces no entity or relation
  in the runtime model and selects no unmodeled catalog publication lifecycle.
- [Generation guide](../../../docs/gitlab-generation.md) and the
  [current generator](../../../crates/connectors-spec/src/v2.rs) establish generated
  requests/dispatch plus mandatory native prepare/finish bindings today.
- At baseline `857569291b1a0601b27ad2bb8b050cdefa022f2f`, the retained GitLab source
  contains 1,847 operation IDs; the authored adapter selects eleven read operations
  and one write. [Source provenance](../../../adapters/gitlab/upstream/README.md)
  records the exact vendor digest and known response-description limitations.
- [Recorded ESS import](../../../adapters/gitlab/generated/ess-import.json) refuses
  OpenAPI 3.0.0 because the pinned importer accepts only 3.1. Full import is not
  established by existing generated requests. Support 3.0 and 3.1 deliberately,
  or prove a semantic normalization; never relabel a version string to bypass a
  refusal. [The ESS pin](../../../crates/connectors-spec/toolchain.json) changes
  only through reviewed toolchain work.

## Consequences and validation

Ordinary HTTP coverage grows primarily by source import and declarative data.
LLMs can implement and review shared mechanisms and explicit exceptions; they do
not author thousands of repetitive endpoint handlers. Specialized operations
continue to carry native contracts and conformance obligations.

This documentation decision requires link/source checks and AEP consistency.
Runtime evidence remains with implementation artifacts. No new story or epic is
decomposed here, so the planning decomposition critic panel does not apply.
