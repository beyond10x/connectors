---
format: aep.planning-md/1
id: specification:core-model-closure-20260909
kind: specification
status: draft
title: Finite ESS model closure for the selected three-adapter specifications
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: specification:spec-completion-parallel-20260909
revision: 2
---
# Finite ESS model closure, 2026-09-09

The operator authorized completing reviewed specifications with ESS, local commits and parallel agents. This is an interactive specification authoring stage, not runtime decomposition. The prior 48-item review intake is complete at 8a5cf563fc717fd4b23e4dd7d470d0c967f39461 with both independent approvals. The finite scoping report is retained at docs/evidence/spec-completion-20260909/model-closure-scope-original.md. Its conservative proposals are accepted for authoring/review: they must be selected in owning prose before ESS relations are claimed settled.

## Scope and identities

Complete only shared auth/audit state, Kubernetes discovery state and immutable artifact provenance needed by Kubernetes (including discovery), GitLab and SQL. Existing eleven entity identities remain unchanged. Connection and custody/profile/acquisition records gain explicit typed homes; administrative lifecycle is separate from readiness/current evidence. Collections own bounded scope-qualified observations; a child route stores non-pinning historical coordinates. Audit retains one acknowledged admission and at most one idempotently recoverable final observation. Composition, curation and fixed route bindings are values, not invented durable process entities.

No runtime, public codec, strict adapter schema implementation, backend, provider access, Python/helper program, optional Prometheus scoped binding, catalog publishing, SaaS assignment or external publication. Actual storage/clock/crypto/native predicates remain explicit UNMAPPED implementation obligations. Missing ownership/lifecycle decisions in this finite scope must be resolved in prose; do not hide them as runtime gaps.

## Parallel allocation

Common base is 8a5cf563fc717fd4b23e4dd7d470d0c967f39461. Root is the sole planning writer. Every tree has its own target/ and .local/model-closure-20260909 scratch/TMPDIR. No shared build cache or nested trees.

| Agent | Managed id / branch | Exact tracked write allocation |
|---|---|---|
| auth | specs-model-closure-20260909 / specs/model-auth-20260909 | new ess/domains/auth_bindings.yaml; contracts/auth/{profile,connection,acquisition,custody}/v1alpha1/semantics.md; docs/evidence/model-auth-closure-20260909/ |
| discovery | specs-model-discovery-20260909 / specs/model-discovery-20260909 | new ess/domains/discovery_state.yaml; ess/domains/discovery.yaml; contracts/discovery/{resources/v1alpha1/semantics.md,mediated_route/v1alpha1/semantics.md,composition.md}; docs/evidence/model-discovery-closure-20260909/ |
| audit | specs-model-audit-20260909 / specs/model-audit-20260909 | new ess/domains/execution_audit.yaml; new contracts/service/audit.md; docs/evidence/model-audit-closure-20260909/ |
| coordinator | specs-integration-20260909 / integrate/spec-completion-20260909 | new ess/domains/artifact_provenance.yaml; ess/system.yaml; existing declarations, credentials, refresh, credential_evidence, connection_admission, mutations, delegation and service_wire models; contracts/catalog/v1alpha1/semantics.md; service compatibility/delegation; contracts/README.md; docs/design.md; planning; integration evidence |

All paths are under /home/timo/.local/state/worktree/trees/b10x/connectors_v2/<managed-id>. Workers return exact patches for coordinator-owned joins; they never edit another allocation. Add new domain registrations only in clearly labeled scratch copies for independent compile checks; root applies authoritative system.yaml registration and real cross-domain joins. Do not invent fake authoritative stub entities. Agreed identity/type manifest and briefs live in each unit's scratch. Root validates actual integrated references before closure.

## Acceptance and checks

Record conservative retention, publication, correlation/expiry, ownership/cardinality and non-pinning references in prose; model settled commands and causation without introducing unsupported grants. Preserve earlier entity identities and distinguish observations from current permission. Author focused meaningful model/scenario cases for concurrency/refusal/retention boundaries. Pinned ESS 0.20.0 validate/compile and deterministic projections establish declared structure; manual traces do not execute provider or persistent transactions. No cold/full Rust build per worker. Coordinator runs one full gate on integrated changes and two independent reviews of a frozen source. Stop expanding once no ownership/lifecycle decision blocks the three-adapter slice and optional bindings remain explicitly unadvertised.

## Recovery

Verified local checkpoint 8a5cf56 was published only to local-recovery and fast-forwarded into clean primary main. Five completed previous worker/reviewer trees were finished and removed by exact-id GC after successful dry-run; all raw scratch was moved to /home/timo/.local/state/worktree/recovery/connectors_v2-spec-completion-20260909/<id>/spec-completion-20260909 and selected evidence is committed. Current model/integration trees remain active for this stage. Preserve own lease rules and evidence; root makes verified bot commits and local recovery publication before any later finish/GC. Older retained design/Kubernetes trees remain untouched.

## Pre-story allocation tooling

AEP refused specification-level machine scope with this exact message:

```text
error: `scope` is a field of `story`, and `specification:core-model-closure-20260909` is a `specification`: a task inherits the surface of the story it decomposes, and nothing above a story is a unit anybody puts in a wave. Record the scope on the stories instead
```

This stage drafts the missing ESS models before entity-bearing story decomposition, as required by the planning/ESS workflow. It is not an AEP story wave or engine-governed drive run. The coordinator records exact disjoint worker allocations in docs/evidence/core-model-closure-20260909/work-units.json and checked all three pairs for file/directory-prefix overlaps (zero). That file is coordination evidence, not an AEP-validated scope field or a new executable helper. Root owns all shared joins and planning. Once the real models validate, any later implementation story may cite their typed homes and use AEP story scopes; no placeholder entity or story was invented to evade the refusal.

Auth reference agreement is retained from specs-model-closure-20260909/.local/spec-completion-20260909/auth-join-proposal.md: Connection.connection_ref:String; immutable profile_record_ref qualifies adapter/local profile/declaration revision while existing profile_ref stays logical; active generation uses the existing CredentialGenerationId; current custody references remain on the binding owner and do not make captures pin bytes forever. Discovery supplies Optional<connectors.discovery.MediatedRouteBinding> as a coordinator-only child field with no live observation FK. Audit uses only existing ServiceConfiguration and optional AttemptRecord references.
