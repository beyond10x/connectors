---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-parallel-r1
kind: review-result
status: active
title: Plan critic parallel, round 1, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
needs-revision
story:connection-binding-excludes-operations — it and draft story:configuration-change-error both claim `crates/connectors-host/src/local/registry/lifecycle.rs` (cited, both) and `docs/local-catalog-provider.md` (cited, both). The two also meet on the `lifecycle_conflict` refusal: one keeps today's `next_action`, the other replaces the code. The body names neither. Order them with an edge that records the shared file as its reason, or split the surface so they no longer share it — .engineering/planning/story/connection-binding-excludes-operations.md:47, .engineering/planning/story/configuration-change-error.md:25
story:connection-binding-excludes-operations — it and story:catalog-client-credentials-profile (status `proposed`) both land on `adapters/catalog/src/local.rs` (cited, both) and `docs/local-catalog-provider.md` (cited, both). This story splits the `Local::configure` digest. The other requires "existing configuration revisions unchanged" (`oauth_existing_configuration_revisions_unchanged`). Neither body mentions the other. Order them with an edge that records the file as its reason, or split the surface — .engineering/planning/story/connection-binding-excludes-operations.md:13, .engineering/planning/story/catalog-client-credentials-profile.md:14
story:connection-binding-excludes-operations — it shares `docs/local-catalog-provider.md` (cited, both) with draft stories story:catalog-zendesk-reads and story:catalog-validates-enum-and-date-time, and says nothing about them. Name the pair with an edge that records the shared doc, or move this story's doc change to a section the others do not touch — .engineering/planning/story/connection-binding-excludes-operations.md:63, .engineering/planning/story/catalog-zendesk-reads.md:46
story:invoke-definition-file — it and draft story:spec-models-operation-admission both regenerate `apps/connectors-cli-contract` (cited, both), and neither body mentions the other. A third draft, story:toolchain-pins-newest-release-20261001, lists the same directory as inferred only. Order them with an edge that records the shared directory, or split the surface — .engineering/planning/story/invoke-definition-file.md:15, .engineering/planning/story/spec-models-operation-admission.md:11
story:invoke-definition-reuse-credential — its entire scope is inferred (15 of 15 paths, body "Inferred (no custody copy exists today; the scoper should confirm)"). Inferred overlaps exist with story:configuration-change-error (`registry/lifecycle.rs`, `contracts/cli/v1alpha1/semantics.md`) and story:mcp-cli-journey-discovery-contract (`contracts/cli/v1alpha1/semantics.md`). Neither is named, so these collision claims are weaker. Record a scope with `aep plan artifact scope` and name or order the two — .engineering/planning/story/invoke-definition-reuse-credential.md:Scope

**What I read.**
- 3 drafted stories (`aep plan artifact show` on each) and the parent ADR relations.
- `aep plan artifact waves` and `relations`.
- The bodies of configuration-change-error, spec-models-operation-admission, catalog-client-credentials-profile and mcp-cli-journey-discovery-contract.
- The waves output filtered to non-terminal stories (95 draft, proposed or active).
- Placement: cited 2 (binding, file), inferred 1 (reuse, fully inferred), unplaceable 0. The binding story has one inferred path (`metadata_conformance.rs`) and the file story two.
- Acceptance reads traced to a producer in the set: 3. The file story reads the binding story's no-stale behaviour, the reuse story reads the file story's `--definition` command, and the reuse story also reads the binding story's lifecycle rules. A `depends_on` edge records 2 of them directly (reuse to file, file to binding). The third (reuse to binding) runs through that chain, and the waves put them in order 14, 16, 23.
- Within the set, the pairs that share files are ordered by `depends_on` (binding to file to reuse), so those collisions are named.

**What I could not establish.**
- Whether story:toolchain-pins-newest-release-20261001 and story:mcp-cli-journey-discovery-contract really touch the surfaces they are inferred on.
- Out of my lane, for the design critic: the binding story claims 30 paths across the adapter, host, app, contracts, ess and website, and so collides with about 80 stories. Most of them are `implemented`, so this does not set my verdict.
- Out of my lane, for the scope critic: the file story's cache and its owner-only configuration file have no surface of their own.

```findings
- file: .engineering/planning/story/connection-binding-excludes-operations.md
  line: 47
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it and draft story:configuration-change-error both claim crates/connectors-host/src/local/registry/lifecycle.rs and docs/local-catalog-provider.md (cited, both) and meet on the lifecycle_conflict refusal, and the body names neither; add an ordering edge recording the shared file or split the surface"
- file: .engineering/planning/story/connection-binding-excludes-operations.md
  line: 13
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it and proposed story:catalog-client-credentials-profile both land on adapters/catalog/src/local.rs and docs/local-catalog-provider.md (cited, both; the digest split versus existing revisions unchanged) and neither body names the other; add an ordering edge recording the shared file or split the surface"
- file: .engineering/planning/story/connection-binding-excludes-operations.md
  line: 63
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it shares docs/local-catalog-provider.md (cited, both) with draft stories story:catalog-zendesk-reads and story:catalog-validates-enum-and-date-time and names neither; add an ordering edge recording the shared doc or split the doc sections"
- file: .engineering/planning/story/invoke-definition-file.md
  line: 15
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it and draft story:spec-models-operation-admission both regenerate apps/connectors-cli-contract (cited, both; story:toolchain-pins-newest-release-20261001 inferred only) and neither body names the other; add an ordering edge recording the shared directory or split the surface"
- file: .engineering/planning/story/invoke-definition-reuse-credential.md
  line: 0
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its whole scope is inferred (15 of 15 paths), and the inferred overlaps with story:configuration-change-error (registry/lifecycle.rs, contracts/cli/v1alpha1/semantics.md) and story:mcp-cli-journey-discovery-contract (semantics.md) are unnamed, so these collision claims rest on inferred surfaces; record a scope and name or order the pairs"
```
