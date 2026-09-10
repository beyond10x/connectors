---
format: aep.planning-md/1
id: story:guarded-gitlab-merge
kind: story
status: active
title: Perform an approved GitLab merge without duplicate effects after restart
relations:
- decomposes: initiative:complete-local-connectors
- depends_on: story:local-approval-binding
- depends_on: story:local-approval-keys
- depends_on: story:local-bounded-clock
- depends_on: story:local-execution-audit
- depends_on: story:local-mutation-ledger
- informed_by: story:gitlab-mr-validation
- informed_by: story:persistent-gitlab-journey
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: adapters/gitlab
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: contracts/cli
- confidence: cited
  path: contracts/service
- confidence: inferred
  path: crates/connectors-build
- confidence: inferred
  path: crates/connectors-conformance
- confidence: cited
  path: crates/connectors-host/src/local
- confidence: cited
  path: crates/connectors-sdk
- confidence: cited
  path: crates/connectors-spec
- confidence: inferred
  path: docs
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: ess/domains/declarations.yaml
- confidence: cited
  path: ess/domains/local_approval_policy.yaml
- confidence: cited
  path: spec-kinds/adapter
- confidence: inferred
  path: website
revision: 6
---
## Outcome

Make the local production CLI perform an explicitly approved, SHA-guarded GitLab merge through a single connection-bound dispatch, including honest recovery when the merge response is lost. This is one integrated runtime slice: policy/issuance, native preparation, host ledger admission and final write must work together before it is complete.

## Acceptance

After an approved CLI merge changes a fixture MR and its response is lost, invoking the same business key after CLI/owner restart returns the original uncertain attempt without issuing a second native merge.

## Selected contracts and model

contracts/service/local-mutations.md owns serialized local policy, owner identity, canonical subject preparation, protected issuance, audit and dispatch ordering. ess/domains/local_approval_policy.yaml declares the retained policy entity and its references to the existing service configuration and issuer; existing delegation/approval/attempt/audit entities keep their ownership. Its process-only NativePreparation value is not a persistent session. Pinned ESS validation before decomposition reports `connectors v1 — 23 file(s), valid`. Runtime predicates marked UNMAPPED require implementation tests, not additional invented lifecycle states.

contracts/cli/v1alpha1/private-mutations.md selects explicit local configuration/private transport version two and bounded prepare/commit/cancel exchanges, preserving version-one codecs and read projections. spec-kinds/adapter/v3/semantics.md selects a closed write array, scalar JSON body mappings, separate descriptor projections and one-use generated write execution; ess/domains/declarations.yaml gives write mapping values a typed home. Author and test each closed schema before activating its new format; current v1/v2 inputs remain usable. Generated outputs are never edited manually.

adapters/gitlab/contracts/guarded-merge.md owns merge semantics against the unchanged pinned upstream. Reuse native ValidationInput/Observation in adapters/gitlab/spec/ess/domains/merge_requests.yaml. Native preflight checks current MR/head/pipeline before the host final gate; commit performs one SHA-guarded PUT with immediate merge and no source-branch deletion. Pipeline-ID checks are preflight observations, not an atomic provider guarantee. GitLab remains the owner of native scopes, schemas, request mapping and effect interpretation.

## Runtime implementation

Publish a revisioned owner policy using metadata migration eight, active issuer identity, admitted operation metadata, exact config/descriptor/executable/clock selections and bounded shared/exclusive use leases. The initial metadata/lease port was prototyped under the parent before this decomposition; its twelve tests are preliminary evidence only, not a CLI capability or completed story. All subsequent changes to that port and its integration belong here.

Generate production `approvals policy-set`, `policy-status`, `prepare` and `issue` handlers. Reconstruct the exact subject from authenticated owner/configuration and retained selected connection; explicit `--approve-subject` must match. Obtain current bounded authenticated time, hold current policy/key leases, read the qualified signing seed and durably publish a protected proof file without stdout disclosure or overwrite. Update authored CLI input/output types and parser bindings, including protected approval input, idempotency key and existing MutationObservation/SourceAudit results. No new public delegated protocol is advertised.

On invocation, perform current result-access admission before key lookup. Exact retained key observations do not do native preflight, secret reads or new approval spend. For a new candidate, validate proof and credential readiness, perform bounded native preflight through the owned child, then recheck current time/policy/key/connection use and separately acknowledge audit admission, attempt/key prepare, approval spend, connection dispatch admission and attempt dispatch gate. Only its original live winner can send the matching private commit. Consume the native preparation and authenticated write capability once. Any post-gate uncertainty remains quarantined across restarts; neither timeout nor a later exact-target read authorizes a resend or attributes another actor's merge to this attempt. Audit finalization cannot rewrite a known effect.

## Verification and completion boundary

Run production CLI fixtures with disposable HTTPS GitLab, qualified Secret Service and authenticated clock fixtures. Prove known applied and refused merges as well as the acceptance's lost-response path. Include protected proof publication failure, policy/key revision changes and held-use exclusion, wrong subject/caller/clock/key, absent/expired/reused approval, duplicate business keys and conflicts, concurrent repair/revoke/dispatch, schema/permission refusals, preflight changes, every storage acknowledgement fault, preparation substitution/cancel/expiry, duplicate commit, old-peer refusal and exact child cleanup. Preserve already valid credentials after failed repair, and prove no new effect on replay after restart. Verify migrations one through seven remain unchanged and setup stays at version three.

Run affected shared/native ESS, CLI and adapter generation/drift, conformance, the full Connectors gate with Rust 1.88, website/reference/example checks and existing GitLab CLI journeys using task-owned TMPDIR and two Cargo jobs. Retain exact source, tool and artifact identities and failures/corrections. Reproduce generated output and the selected distributable payload from identical pinned inputs; retain time-stamped receipts separately. Dedicated GitLab sandbox merge/read verification followed by exact cleanup is additionally required before this story is implemented; credential-blocker:gitlab-runtime-sandbox remains open until explicit access exists. Fixture evidence alone cannot close it.

## Scope and sequencing

Root is the sole implementation/planning writer on primary main. Cited surfaces: crates/connectors-host/src/local (policy, existing ledgers, owner and private child transport), crates/connectors-sdk (separate one-use write capability), crates/connectors-spec and spec-kinds/adapter (v3 generation), adapters/gitlab (native semantics/model/spec/bindings/fixtures), apps/connectors and apps/connectors-cli-contract (authored/generated CLI), contracts/service and contracts/cli, ess/domains/declarations.yaml, ess/domains/local_approval_policy.yaml and ess/domains/cli.yaml. Inferred verification/publication surfaces: crates/connectors-build, crates/connectors-conformance, docs, website and Cargo.lock. All overlap with existing local approval/key/clock/audit/attempt and GitLab read stories is serialized; no parallel implementation or shared build is scheduled. The four required planning critics are read-only; root records their outputs.

The parent continues to own full GitLab completion, C14 create/update under its open atomic-head decision, dedicated read acceptance, other recorded GitLab collection, complete management across three adapters and remaining reproducible distribution acceptance. Kubernetes including Helm follows full GitLab, then PostgreSQL, then MCP, then remaining providers. This merge slice neither substitutes for those workflows nor moves their lifecycle. MCP publication/exact pin is later; Connectors publication, deployment, Atlas integration and paid governed runs remain outside this story. Preserve the unrelated AGENTS.md release edit without claiming it as this work's authorization.

## Guarded merge implementation checkpoint — 2026-09-11

The proposed contracts/model and governed decomposition are recorded in story:guarded-gitlab-merge, now active. The initial policy metadata port is implemented with migration eight, retained identity/revision CAS, exact issuer/instance binding, sorted bounded operation sets, passive inspection and process-bound shared/exclusive leases. Twelve policy tests pass, including actual child exit after commit before acknowledgement, cross-process held-use exclusion, fork refusal, failed writes, revision exhaustion and restart. This storage port consumes an already admitted set from its trusted host caller; it does not itself authenticate callers, classify native operations, issue proofs or dispatch writes.

The complete Connectors gate with Rust 1.88 passes, including 103 host tests, six independent native model compilations, CLI/adapter generation, Clippy, conformance and planning validation. Retained evidence is docs/evidence/local-approval-policy-20260911/README.md. CLI policy/prepare/issue, private protocol two, v3 write generation, one-use write capability and native guarded merge remain required implementation under the active story. Its lost-response CLI acceptance, provider sandbox evidence and full distribution reproducibility have not been satisfied.

Design, scope and parallel-safety critics approved round one. The first acceptance reading reused the scope critic's context; it is retained as an archived review-result and is not counted as the independent acceptance review. A fresh-context acceptance reviewer approved revision five without reading other critic outputs. Sonnet was unavailable, so inherited models were used; the fourth lane was delayed by available slots. No reviewer wrote code or planning state. AEP initially refused promotion because the existing-vision serves edge was missing, then refused a direct draft-to-active move; the edge was added and the legal draft-to-proposed-to-active transitions succeeded. These are planning checks, not runtime acceptance.

The owner remains root, with serialized implementation on main. Full GitLab still precedes Kubernetes, PostgreSQL, MCP and remaining providers. The dedicated GitLab sandbox and create/update atomic-head decision remain open. Preserve the unrelated AGENTS.md release edit; this checkpoint does not publish Connectors or register Atlas.
