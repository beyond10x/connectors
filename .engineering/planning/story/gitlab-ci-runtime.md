---
format: aep.planning-md/1
id: story:gitlab-ci-runtime
kind: story
status: active
title: Read exact-commit GitLab pipelines, jobs and bounded traces through the local CLI
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:persistent-gitlab-journey
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/gitlab
- confidence: cited
  path: contracts/cli/v1alpha1
- confidence: cited
  path: crates/connectors-build
- confidence: cited
  path: crates/connectors-conformance
- confidence: cited
  path: crates/connectors-host
- confidence: cited
  path: crates/connectors-sdk
- confidence: cited
  path: crates/connectors-spec
- confidence: cited
  path: docs
- confidence: cited
  path: ess
- confidence: cited
  path: spec-kinds/adapter/v2
- confidence: cited
  path: website
revision: 10
---
## Acceptance

Through the production local CLI and a saved GitLab connection, an operator completes C09 by selecting pipelines for one exact full commit SHA, observing the selected pipeline from pending/running to terminal without substituting another SHA, paging its current jobs and reading the failed job's trace with explicit bounded completeness.

## Verification prerequisites

Before this work only project.get, issues.list and file.get exist. Deliver the five declared CI operations through generated bindings, preserving target/provenance and failure semantics. Retain deterministic native tests, production CLI fixtures, generation/drift checks and the required repository gate. Dedicated GitLab sandbox C09 evidence is required before this story is implemented; local substitutes do not clear that requirement. All prerequisites below support the single acceptance statement above. Complete this GitLab work before Kubernetes.

## Semantic owners and prior review

adapters/gitlab/contracts/ci/v1alpha1/semantics.md and adapters/gitlab/spec/ess/domains/ci.yaml define native Pipeline, Job and Trace values, exact SHA/ID predicates, current-attempt pagination and UTF-8 prefix behavior. No SaaS resource lifecycle or local persistent identity is introduced. Both the shared root and native root passed pinned ESS validation before this decomposition. Existing auth profiles, Connection/CredentialGeneration/CustodyVersion and DispatchAdmission continue to own authority. The static profile's read_api grant and configured operation allowlist remain necessary.

spec-kinds/adapter/v2/semantics.md and ess/domains/declarations.yaml add the optional fixed response_prefix_limit mapping; ess/domains/transport.yaml gives HttpResponsePrefix its typed home. The shared capability retains a bounded body prefix and only reports complete after observed EOF; it adds no retry, range/resume identity, provider interpretation or business authority. Native finish methods own response codecs. The existing complete-body transport signature remains compatible when the field is absent. The reviewed decomposition must retain these boundaries before dependent runtime implementation.

The existing pinned GitLab OpenAPI bytes contain all five GET endpoints. Do not refresh the vendor source or repair its incorrect list/trace response schemas. Preserve those limitations in coverage and native finish obligations. The public docs were checked, and the unavailable exact-revision jobs implementation is recorded; prefix transport avoids depending on optional provider byte-limit parameters.

## Implementation sequence and evidence

1. Implement the additive SDK/host prefix capability and v2 schema/frontend/code generation. Unsupported bindings refuse; zero/over-limit declarations refuse. Verify empty, exact-limit, oversized, chunked and prematurely closed bodies, deadline/cancellation, header bounds and no redirection/retry. Keep old mappings and consumers byte/behavior compatible when the field is absent.
2. Author and regenerate pipelines.list, pipeline.get, pipeline.jobs, job.get and job.trace. Implement native project/SHA/pipeline/job checks, projection, paging, prefix/UTF-8 handling and scoped cursor partitioning. Bind all five as explicit read operations in the existing private local bootstrap. No provider logic enters host/generator/SDK.
3. Run deterministic native tests and an actual production CLI/private HTTPS/keyring journey: pending to running to failed for one SHA, another SHA success cannot satisfy it, multiple job pages, exact failed trace, explicit truncation, unknown status preservation without interpreting it as success, and permission/service/schema/cursor failures. Preserve connect/restart/revalidation/repair/revoke and the original three reads. Obtain dedicated GitLab sandbox C09 proof when access is available; the blocker remains authoritative meanwhile.
4. Regenerate and compare exact outputs, run cargo run --locked --offline -p connectors-build -- gate --msrv and affected website checks, retain commands/source/dependency/executable identities, then commit locally through verified Atlas bot authority. Packaging advances with the selected native descriptor and existing local realization; reproducible full product distribution remains owned by the initiative.

## Scope and serialization

Cited scope: Cargo.toml, Cargo.lock, README.md, adapters/gitlab, crates/connectors-sdk, crates/connectors-host, crates/connectors-spec, crates/connectors-build, crates/connectors-conformance, ess, spec-kinds/adapter/v2 and docs. The generated GitLab bundle changes only through the owning pinned generator. No planning-store path is edited directly; one AEP writer records evidence and lifecycle changes.

This story overlaps persistent-gitlab-journey in host/SDK, native adapter, model and documentation surfaces. Implement them serially in the primary checkout under the repository-specific single-agent rule; they are not a parallel wave. The predecessor's committed persistent-runtime checkpoint supplies the implementation dependency, while its missing sandbox acceptance does not stop this independent native CI implementation. The dependency is informed_by, not a claim that the predecessor's full acceptance is complete.

## Initiative coverage and boundaries

This story decomposes the C09 portion of initiative:complete-local-connectors. The first story owns persistent connection delivery; neither story claims the full GitLab phase. GitLab MR/changed-record reads and C14 governed create/update/merge, their required approval/audit/attempt/dispatch/idempotency foundation, Kubernetes, PostgreSQL, MCP and remaining providers retain their full required order and ownership in the initiative until their missing semantics are modeled and reviewed for decomposition. Do not proceed to Kubernetes merely because C09 or the persistence fixture passes.

The active goal and initiative remain open. No external Connectors publication, desktop keyring access, unrelated Harness/MCP change, new bare recovery repository or paid governed run is authorized by this story. Use task-owned TMPDIR under .local/tmp and two Cargo jobs. Dedicated GitLab sandbox access remains missing, separately from independent implementation progress. Existing driver protocol-loading and historical specification evidence remain unchanged.

## Implementation scope and review record

The final implementation also touches website/publication.json and the GitLab/status pages under website/docs, with generated public references produced only by their Rust owner. Add website as cited machine scope: these files present the selected native CI contract and current runtime boundaries. The story remains serialized with one planning writer.

All four required planning critics ran independently in two bounded rounds. The acceptance finding in review-result:gitlab-ci-acceptance-round-1 was fixed by revision 3 and recorded as review_outcome=fixed. Round 2 returned four approvals with empty findings lists. Immutable review records retain those exact blocks; the installed AEP validator nevertheless lists empty-list approvals as having no findings block. No finding or sandbox requirement was suppressed. The unavailable Sonnet pin was replaced by the inherited session model; three available child slots required scheduling the fourth critic after another completed.

The first promotion was refused because serves was missing. The existing parent/persistence objective, vision:independent-contract-adapters, was read and related through AEP; proposed and active moves then succeeded. No third critic round or claimed runtime completion follows from that administrative correction.

## Provider failure preservation

The production CLI fixture exposed an existing binding discrepancy: a missing provider trace became local not_found at admission, while contracts/cli/v1alpha1/semantics.md section 6 requires other admitted provider failures to retain Failure.service_code. The implementation now preserves native not_found, rate_limited and internal through closed private failure variants and the existing CLI service_failure carrier. Host selection/admission errors and unavailable/timeout handling keep their declared meanings; no raw provider body/message crosses the control channel.

The typed owners already exist in ess/domains/cli.yaml (connectors.cli.Failure.service_code) and the shared service-wire ErrorCode. No new entity or unresolved relation is introduced. contracts/cli/v1alpha1/private-adapter.md records the binding correction and is now cited machine scope. Host serialization/projection tests and the actual CLI missing-trace/rate-limit cases verify it. Initial fixture failures are retained as failed runs, not successful acceptance evidence.

## Verified local CI increment — 2026-09-10

The runtime now adds pipelines.list, pipeline.get, pipeline.jobs, job.get and job.trace through the pinned v2 generator and production local CLI. Six native CI tests, six bounded HTTP-prefix tests, existing redirect/TLS fixtures and a safe private provider-code projection test pass. The four production CLI/private HTTPS/keyring journeys pass together in 176.03 seconds, including C09-shaped exact-SHA polling, job paging, complete/truncated UTF-8 traces, errors and current permission cutoff, plus restart/revalidation/repair/revoke regressions.

The final cargo run --locked --offline -p connectors-build -- gate --msrv passes after the provider error correction: shared/native ESS, generation/reproducibility/refusals, workspace tests, Clippy, library boundaries, shared conformance, CLI drift and Rust 1.88. The local GitLab service image builds and its ESS realization validates. docs/evidence/gitlab-ci-20260910/README.md retains commands, identities, results, failed-run explanations and limits. Two generated output trees match under the pinned inputs; full reproducible distributable/image proof remains pending.

Dedicated GitLab sandbox C09 evidence is still unavailable under credential-blocker:gitlab-runtime-sandbox, so this story remains active. No specification check, local fixture, image build or critic approval clears that blocker. GitLab MR/changed-record reads and the selected governed MR writes still precede Kubernetes, PostgreSQL, MCP and remaining providers. No source publication or deployment was performed.
