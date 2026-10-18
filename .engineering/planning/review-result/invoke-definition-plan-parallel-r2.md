---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-parallel-r2
kind: review-result
status: active
title: Plan critic parallel, round 2, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
needs-revision
story:connection-binding-excludes-operations — it cites `crates/connectors-host/src/local/metadata/er.rs` (binding projection) and `crates/connectors-build/src/metadata_conformance.rs`, and draft story:spec-models-operation-admission lists both (inferred on its side) to regenerate the metadata model beside `entity-runtime-definitions.json`. Its Shared surfaces table does not name that story. It also edits `contracts/cli/v1alpha1/semantics.md` (cited), which draft story:mcp-cli-journey-discovery-contract appends a section to (inferred on its side), and the table does not name that story either. Name both pairs with an edge or an "either order, regenerate second" row, or split the surface — .engineering/planning/story/connection-binding-excludes-operations.md:162, .engineering/planning/story/spec-models-operation-admission.md:19, .engineering/planning/story/mcp-cli-journey-discovery-contract.md:51
story:invoke-definition-file — its Shared surfaces table names story:spec-models-operation-admission and story:configuration-change-error but not story:mcp-cli-journey-discovery-contract. That story appends to `contracts/cli/v1alpha1/semantics.md`, and this one adds a `--definition` subsection to the same file (cited here, inferred there). The table also omits draft story:toolchain-pins-newest-release-20261001, which regenerates `apps/connectors-cli-contract` (inferred) and `ess` (inferred), and draft story:kubernetes-spec-service, which claims the whole `crates/connectors-conformance` directory, containing this story's cited `cli_surface.rs`. Add rows or edges for these — .engineering/planning/story/invoke-definition-file.md:146, .engineering/planning/story/mcp-cli-journey-discovery-contract.md:51, .engineering/planning/story/toolchain-pins-newest-release-20261001.md (scope `apps/connectors-cli-contract`, `ess`, inferred)

**What I read.**
- 3 drafted stories, the round-1 record and the parent relations, with `aep plan artifact show`.
- `aep plan artifact scope` and `waves`.
- The recorded scopes of all 95 non-terminal stories (88 draft, 5 active, 2 proposed), compared by exact path and by directory prefix against each drafted story. I read the bodies of configuration-change-error, spec-models-operation-admission, mcp-cli-journey-discovery-contract, toolchain-pins-newest-release-20261001 and kubernetes-spec-service.
- Placement: cited 2 (binding, file), inferred 1 (reuse, 15 of 15 paths), unplaceable 0.
- Acceptance reads traced to a producer in the set: 3.
  - Edges: file to binding, reuse to file, and reuse to binding through the chain. All three hold by `depends_on`; the reuse to binding link is transitive.
  - The `depends_on` edge from story:configuration-change-error to the binding story exists, as the binding body says.
- Round-1 findings fixed, checked against the revised Shared surfaces tables and the edge on story:configuration-change-error:
  - story:configuration-change-error is now named and ordered.
  - story:catalog-client-credentials-profile, story:catalog-zendesk-reads and story:catalog-validates-enum-and-date-time are named.
  - story:spec-models-operation-admission is named on the file story.
  - story:mcp-cli-journey-discovery-contract and story:configuration-change-error are named on the reuse story.

**What I could not establish.**
- Whether story:spec-models-operation-admission, story:mcp-cli-journey-discovery-contract and story:toolchain-pins-newest-release-20261001 touch the surfaces they list as inferred. The two findings above rest partly on those inferred surfaces.
- Reuse's remaining overlap with story:toolchain-pins-newest-release-20261001 (`ess`, inferred) is too weak to report.
- Out of my lane (scope critic): story:kubernetes-spec-service claims whole directories such as `crates/connectors-build` and `crates/connectors-conformance`, so it collides with nearly any story that edits those crates. Its own body says to re-scope before scheduling it beside other source edits.
- Out of my lane (design critic): the binding story spans 30 paths across six areas.

```findings
- file: .engineering/planning/story/connection-binding-excludes-operations.md
  line: 162
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Shared surfaces table omits draft story:spec-models-operation-admission (er.rs cited here, er.rs and metadata_conformance.rs inferred there, so this claim rests on an inferred surface on that side) and draft story:mcp-cli-journey-discovery-contract (contracts/cli/v1alpha1/semantics.md cited here, inferred there); name both pairs with an edge or an either-order row, or split the surface"
- file: .engineering/planning/story/invoke-definition-file.md
  line: 146
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Shared surfaces table omits story:mcp-cli-journey-discovery-contract (semantics.md, inferred there), story:toolchain-pins-newest-release-20261001 (apps/connectors-cli-contract and ess, inferred there) and story:kubernetes-spec-service (crates/connectors-conformance directory versus this story's cited cli_surface.rs); the claims on the first two rest on inferred surfaces; add rows or edges naming each"
```
