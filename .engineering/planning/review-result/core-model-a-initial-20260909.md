---
format: aep.planning-md/1
id: review-result:core-model-a-initial-20260909
kind: review-result
status: active
title: 'Independent model reviewer A initial: MCA-01 missing evidence carrier'
relations:
- reviews: specification:core-model-closure-20260909
revision: 1
---
needs-revision

Initial independent model review A. Exact source: b5abe434fbb34dfd889303f19933a9bff9e2e23a; compared with 8a5cf563fc717fd4b23e4dd7d470d0c967f39461. Review mode: non-interactive, read-only source and planning. This verdict covers the selected auth/artifact first pass only; it does not approve the whole model stage.

MCA-01 (P2) — Add Connection's selected embedded credential-evidence carrier — ess/domains/auth_bindings.yaml:95.

The new Connection fields at lines 82–98 carry current generation, custody and identity, but no EvidenceSnapshot. The compiled IR confirms that absence. Yet ess/domains/credential_evidence.yaml:6 explicitly places EvidenceSnapshot on Connection, contracts/auth/connection/v1alpha1/semantics.md:215 calls it a value embedded in Connection, and contracts/auth/evidence/v1alpha1/semantics.md:247 selects the same home with no separately persisted entity. The only entity field using EvidenceSnapshot is the transient DispatchAdmission, whose declaration at credential_evidence.yaml:59–60 expressly owns no Connection evidence or durable admission storage.

Concrete trace: an admitted managed completion validates generation G, acknowledges custody and publishes Connection C; the temporary validation/dispatch admission then ends. The complete declared C/G/custody graph can retain G's identity and expected identity but cannot carry the baseline evidence that made C's publication valid. A subsequent admitted describe must either invent another unmodeled evidence owner or lose the selected per-check provenance/deadlines. This is an absent model join, not a demand that ESS execute validation or implement durable storage.

Smallest remedy: embed the existing EvidenceSnapshot type on Connection with explicitly selected optionality/multiplicity, active-generation/binding equality, publication/replacement invalidation and bounded retention rules. Prepublication and explicit no-child-material bindings must not manufacture credential evidence. If another owner is intended, select and type that owner and reconcile both normative statements instead of relying on DispatchAdmission.

Manual scope and reasoning:

- Read AGENTS.md, relevant design direction (§§1–3, 12, 18, 31), the four auth contract owners, the current evidence/refresh/admission joins, the service compatibility and operations revision/auth rules, catalog §9 and its provenance/curation rules, and both unit evidence directories' verification/scenario/identity material.
- Inspected all of auth_bindings.yaml and artifact_provenance.yaml, their registration and declarations/credentials/credential_evidence changes, and the actual existing types and relations they join. Private Connection allocation resolves the generation/public-publication cycle; existing String instance identity and CredentialGenerationId remain intact. AuthProfile revision-qualified records are distinct from the existing logical profile_ref. Acquisition consumption, terminal non-reuse, expiry and uncertain acknowledgement rules name the owning boundary; Connection Live is administrative rather than readiness.
- Checked custody references versus ownership, historical capture retention, guarded retirement and non-reuse; no captured-generation-to-custody deletion ownership is added. The selected absence of child material does not imply an anonymous fallback. Reviewed the current non-pinning route prose; the acknowledged pending route field and final entity inventory are excluded from first-pass findings as instructed.
- Artifact Source/Bundle relationships permit shared immutable reuse, while optional index selection owns no artifact deletion. Original source digests, transform digests, license evidence, curation and exact bundle specification digests remain distinct. Inspected current strict v1/v2 adapter reader declarations; no reader implementation changes occur in this delta. I found no additional actionable defect in those selected rules during this pass.

Independently executed checks (all from the assigned tree):

1. git rev-parse HEAD — exit 0; exact source above. git status --short — exit 0, empty before review.
2. /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess — exit 0: connectors v1 — 16 file(s), valid.
3. /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/model-closure-20260909/initial-ir.json — exit 0: 16 files, 263 declarations.
4. jq '.entities[] | select(.name == "connectors.auth_bindings.Connection")' .local/model-closure-20260909/initial-ir.json — exit 0; Connection's actual compiled fields have no EvidenceSnapshot carrier.
5. git diff --check 8a5cf563fc717fd4b23e4dd7d470d0c967f39461 b5abe434fbb34dfd889303f19933a9bff9e2e23a — exit 0. Read the full selected delta to verify the existing identities/lifecycles/old field types were preserved.

Limits: compiler success checks declared types/references/causation only. Unit negative fixtures and deterministic-projection logs were inspected as claims, not rerun as my executions. No runtime-conformance claim, Rust build, integration call, broad test suite, model-specific helper, source edit, planning mutation or commit was performed. Actual trusted owner predicates, atomic field assignment, storage/durability, credential material, clocks and external authority remain runtime obligations. Whole-stage remaining coverage and correction recheck await the coordinator's final exact source. This initial report is preserved unchanged.

```findings
[
  {
    "file": "ess/domains/auth_bindings.yaml",
    "line": 95,
    "category": "design",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "MCA-01 (P2): Add Connection's selected embedded credential-evidence carrier; the new entity has no EvidenceSnapshot field despite both owning contracts placing that value on Connection, leaving completed baseline publication without its selected typed evidence home."
  }
]
```
