---
format: aep.planning-md/1
id: specification:core-model-closure-20260909
kind: specification
status: draft
title: Finite ESS model closure for the selected three-adapter specifications
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: specification:spec-completion-parallel-20260909
revision: 6
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

## Integration and independent review pipeline

Auth unit 6cee258b34ae0c9e0b0a5de5a5ed7fd04e336e4e is integrated by b5abe434fbb34dfd889303f19933a9bff9e2e23a with coordinator registration/reference joins and preserved artifact additions. Unit/coordinator leases on the auth tree are released; it is retained for evidence/recovery, not finished merely because committed. The actual combined root validates/compiles as 16 files and 263 declarations. No full gate or whole-stage approval is claimed yet.

Independent reviewer A starts the frozen auth/artifact source at b5abe434 in managed tree specs-model-review-a-20260909 under the same external worktree root; target/ and .local/model-closure-20260909 remain tree-local. Its initial-brief.md fixes exact scope and read-only rules; session specs-model-review-a-20260909. The real discovery/route field, audit domain and final design inventory are acknowledged pending joins, not silently considered complete. A final exact commit will be supplied for remaining whole-stage coverage and any correction recheck; a second independent reviewer receives the combined source. Root remains sole planning writer.

## Updated operator finish criteria

The operator extended the active goal to: All specs reviewed and stabilized - no impl code written yet - a local clean commit + CHANGELOG.md file and v0.1.0 tagged.

The final source must pass the combined gate and two independent reviews, contain CHANGELOG.md for the specification baseline, and be committed cleanly on local main with an annotated local v0.1.0 tag. No existing tag or changelog was found at this update. Tag only the final verified commit, after all known findings are resolved; preserve local recovery before managed cleanup. The existing first-slice runtime predates the hardening goal and is not erased or represented as newly implemented. No external publication, runtime work or Cargo version change is implied. Root owns changelog/tag/clean-main verification after review.

## Combined source ready for final review

Discovery 84be6ef is integrated by 0a20e43. Corrected audit 28c4e666 is integrated by adb15715; public host-qualified refs remain separate from injective private record keys. Initial reviewer A's immutable MCA-01 report is recorded as review-result:core-model-a-initial-20260909; correction 95b1ebdd is integrated by f2e4a429, preserving the child route carrier alongside one bounded baseline evidence snapshot. The outcome is recorded fixed, pending independent recheck.

Root's final joins explicitly reference retained Connection records from AttemptRecord and optionally AuditRecord, specify the audit key's byte order/canonical decoding, reconcile the twenty-entity inventory and stale model comments, and add historical-context notes without changing earlier execution claims. Coordinator scope therefore includes these small service/operations/audit joins, idempotency/session comments and historical auth/idempotency verification addenda. No worker writes these paths concurrently. Source audits show no runtime, adapter, strict-schema or Cargo changes in this model stage and zero tracked Python files.

The actual shared root validates as 18 files / 314 declarations. Repeated canonical compilation is byte-identical. All eleven earlier entity identities/lifecycles/field types and original named types are preserved; new relations are additive. The shared ownership boundary and all five authored adapter roots validate and compile independently. Results are retained in docs/evidence/core-model-closure-20260909. The full gate and whole-stage final reviewer A/B approvals remain pending at this source freeze; the milestone is not complete merely because unit compiles pass.

## Final reviewed specification closure

Both independent reviewers approve exact 7e8c718303a43bc7ec4a02d6a76581d92802bf6b with no remaining findings. Review-result:core-model-a-final-20260909 explicitly closes MCA-01; review-result:core-model-b-final-20260909 independently approves the whole delta. Their immutable reports and initial A report are retained with fixed/no-op outcomes. The full gate passed on that exact source: 58 Rust tests, MSRV1.88, shared/native ownership and ESS checks, 315 synthesized structural scenarios including 34 authored, zero refusals. Two schema projections produced 336 byte-identical files; all 26 new links/five anchors resolve. Compiler/scenario synthesis is not persistent/provider execution.

The selected specification hardening is complete. Final local release packaging adds only evidence/planning to the reviewed source and uses the already reviewed CHANGELOG.md. The clean local main commit is to receive annotated v0.1.0; root verifies exact peeled commit and bot tagger, publishes recovery only to the local bare repository, and completes own reviewer/integration cleanup. No further runtime work is part of this goal. The prior 48-finding/27-story closure is unchanged. Optional bindings and future runtime conformance cannot expand this finite milestone.

docs/evidence/core-model-closure-20260909/checkpoint.md links source archive, validation, reviews and recovery. The three model authoring trees were finished and GC-removed only after local recovery proof; exact dry-run/apply outputs are retained. All raw scratch remains under /home/timo/.local/state/worktree/recovery/connectors_v2-model-closure-20260909/. Remaining final cleanup is operational verification after the release commit, not an unresolved semantic decision.
