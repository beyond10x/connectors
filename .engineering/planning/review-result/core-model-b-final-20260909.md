---
format: aep.planning-md/1
id: review-result:core-model-b-final-20260909
kind: review-result
status: active
title: Independent model reviewer B approves the complete specification baseline
relations:
- reviews: specification:core-model-closure-20260909
revision: 1
---
approve

Independent final semantic/ESS review B, 2026-09-09. Reviewed commit: `7e8c718303a43bc7ec4a02d6a76581d92802bf6b`; reviewed delta starts at `8a5cf563fc717fd4b23e4dd7d470d0c967f39461`. The baseline's earlier semantic remediation was an input to this review, not independently re-certified here.

No concrete contradiction, unsafe authority grant, or missing selected ownership/lifecycle/retention decision was established in the shared model closure for Kubernetes including discovery, GitLab and SQL. This approves the reviewed specification delta, not a runtime implementation or release action.

Review coverage and reasoning:

- Artifact provenance preserves the existing logical adapter/operation identities. Source identity conflicts refuse replacement; Bundle's logical adapter relation is distinguished from its exact specification digest. Source/Bundle reuse and independent retention agree with the removal of index deletion ownership. Curation does not admit execution or extend a strict reader: `ess/domains/artifact_provenance.yaml:46`, `ess/domains/declarations.yaml:64`, `contracts/catalog/v1alpha1/semantics.md:191`.
- AuthProfile's immutable revision-qualified record is separate from the existing logical profile coordinate. Connection is administratively Live/Revoked, with readiness reduced from current facts. Allocation before acquisition/capture resolves the initial reference ordering without granting dispatch. Acquisition consumption, guarded completion, terminal retention and custody retirement preserve the existing exchange/publication fences: `ess/domains/auth_bindings.yaml:63`, `contracts/auth/profile/v1alpha1/semantics.md:123`, `contracts/auth/connection/v1alpha1/semantics.md:118`, `contracts/auth/acquisition/v1alpha1/semantics.md:142`, `contracts/auth/custody/v1alpha1/semantics.md:70`.
- The added Connection baseline carrier uses the existing EvidenceSnapshot type for one current material generation. Its six-check bound, baseline scope restriction, absence on material-free bindings and separation from exact-operation evidence agree with the existing admission and transfer rules. Current-generation invalidation cannot be replaced by retained positive history: `ess/domains/auth_bindings.yaml:96`, `contracts/auth/connection/v1alpha1/semantics.md:144`, `contracts/auth/evidence/v1alpha1/semantics.md:93`.
- DiscoveryCollection owns bounded scope-epoch observations and whole publication state. Withdrawn observations cannot reobserve; view retirement advances the publication fence; unknown acknowledgement only permits observation of the same current attempt. Child route coordinates do not pin expired rows or permit rebinding across epochs. Composition remains an admitted value with explicit instance/revision resolution: `ess/domains/discovery_state.yaml:57`, `ess/domains/discovery.yaml:113`, `contracts/discovery/resources/v1alpha1/semantics.md:177`, `contracts/discovery/mediated_route/v1alpha1/semantics.md:84`, `contracts/discovery/composition.md:41`.
- Audit's private identity injectively qualifies the public instance/ref pair. Separate gateway/leaf records, one final observation, idempotent append acknowledgement and independent retention agree with compatibility and delegation. Neither an early refusal, anchor alone, append recovery nor a historical attempt/Connection reference becomes dispatch authority: `ess/domains/execution_audit.yaml:47`, `contracts/service/audit.md:12`, `contracts/service/audit.md:60`, `contracts/service/audit.md:82`, `contracts/service/audit.md:101`.
- Additive joins preserve the original eleven entity identities, existing field types and lifecycles in the source diff. The compiled root contains twenty entities, matching the inventory. Design and CHANGELOG distinguish this stabilization milestone from the older first-slice runtime and keep deferred bindings explicit: `docs/design.md:1354`, `CHANGELOG.md:3`.

Checks actually executed in the assigned tree:

1. `TMPDIR="$PWD/.local/model-closure-20260909" /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess` — exit 0: `connectors v1 — 18 file(s), valid`.
2. `TMPDIR="$PWD/.local/model-closure-20260909" /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/model-closure-20260909/ir.json` — exit 0: 18 files, 314 declarations. The resulting IR has twenty entities.
3. `git diff --check 8a5cf563fc717fd4b23e4dd7d470d0c967f39461..HEAD` — exit 0.
4. `git rev-parse HEAD` remained the exact reviewed commit; `git status --short` was empty before report creation. Tracked source and planning files were not changed.

Logs and compiled IR are retained beside this report. I read the assigned AGENTS.md, design document, relevant source delta and owning contracts, with the worktree and ESS skill boundaries. This was a non-interactive delegated read-only review; no planning mutation or approval bypass was needed. No other reviewer's findings or reports were read. No provider access, Rust/full build, new helper program, runtime code, commit, tag or publication was performed. The coordinator separately reported a passing full gate; this report does not present that as a check I ran. Schema/lifecycle compilation does not execute storage atomicity, clock, cryptographic, cross-record predicate or public-codec obligations, and approval does not claim otherwise.

Managed tree retained for coordinator handoff: `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-model-review-b-20260909`; session lease `specs-model-review-b-20260909` was acquired and heartbeated. Own-lease release is checked separately in the final handoff.

```findings
[]
```
