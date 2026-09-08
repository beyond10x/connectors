# Credential evidence verification (F05)

Status: proposed semantics and compiled ESS scenarios; **no auth runtime implementation or runtime conformance execution**. Baseline for this unit is wave opening commit `1e567571d9ac62070933c7099b93a4e030613e58`. The pinned executable is ESS `0.20.0`. The refresh domain remains the coordinator's empty registered surface on this unit branch; final combined verification belongs to the integration wave.

The acceptance statement is: every credential-replacement scenario uses evidence valid for the dispatched credential identity. [The normative contract](semantics.md#41-credential-generation-and-identity) binds a host-private immutable material generation to evidence and to final dispatch. [The model](../../../../ess/domains/credential_evidence.yaml) preserves evidence as a value on Connection and gives the host's transient per-invocation admission decision a real lifecycle. It does not add a persisted evidence entity.

## Decision and failure matrix

| Situation | Required decision and observable result | Authored case / remaining runtime evidence |
|---|---|---|
| Configured A token replaced by B token at the same path/ref/revision | Declared admitted validation observes B; refuse ordinary replacement, keep the intended A binding; zero business requests with B | [configured-replacement-identity](scenarios/configured-replacement-identity.yaml); actual file replacement, provider observation and zero requests remain runtime obligations |
| New generation still carries A's evidence, or has no identity evidence | Refuse `generation_changed` / `validation_required` internally; public `connection_not_ready`; no implicit identity call from the business operation | [unvalidated-configured-replacement](scenarios/unvalidated-configured-replacement.yaml); actual generation comparison and effect budgets remain unexecuted |
| Same-account generation B publishes after A admission but before dispatch | Publication cuts off old pending A admission; it cannot substitute B. A new admitted decision may dispatch validated B | [rotation-before-dispatch](scenarios/rotation-before-dispatch.yaml); the external publication event, byte pin and atomic cutoff are not injected by ESS |
| Source changes but the host has not observed/published it | A still-current, still-usable pin can only dispatch its original validated bytes; detection blocks later dispatch until admitted validation | Textual contract; transport capture and source-observation timing require host tests |
| Same-account refresh preserves proven identity lineage but narrows scopes | Preserve only permitted check provenance and original deadlines; recompute new validity/scopes, invalidate old permission/verification; refuse a write whose grant disappeared | [same-identity-refresh](scenarios/same-identity-refresh.yaml), using a fake profile, not a claim about a vendor's refresh response |
| New permission evidence applies to a newly admitted read | Bind it to that generation and exact operation/target; a wider provider grant still does not widen host authorization | Same authored case records the decision; actual target and SaaS policy comparisons remain runtime tests |
| Pin exists but credential is revoked, expired, or undergoing unresolved rotating refresh | Refuse pending admission; it cannot be reopened or dispatched. A generation UUID cannot evade refresh exclusion for aliases | [revocation-before-dispatch](scenarios/revocation-before-dispatch.yaml); actual revocation, clock and refresh state checks are unexecuted |
| Identity cannot be established or changes authority/kind/subject | Refuse; reassignment requires a separately authorized flow and invalidation of old admissions/evidence/cursors/sessions | Textual contract; no implicit reassignment flow or caller-supplied proof is modeled |
| Exec helper would return different bytes on a second run; certificate/key files change independently | Pin one validated helper output or coherent cert/key capture; never substitute a second result at dispatch | Textual contract plus Kubernetes/capability obligations; future transport fixtures required |
| Public evidence serialization | Omit secret bytes, generation IDs, snapshot handles and secret-store versions | Removed the misleading public `version` example; future serializer redaction tests required |

## What compilation proves and does not prove

The authored traces select trusted decision commands and expected outcomes. They are useful for type, event, view, enum and lifecycle consistency, including terminal refusal and one generation reference per admission. They do **not** cause file replacement, verify an external identity, compare nested evidence generations, invoke provider permission checks, calculate freshness or implement atomic publication. In particular, a well-typed trace selecting `AdmitGeneration` for mismatched observations is not rejected by this model's structural compiler. The host must enforce the documented predicate; claiming otherwise would convert an authored expectation into invented execution evidence.

`UNMAPPED` obligations are: trustworthy identity and grant observations; exact request/permission target binding; immutable snapshot allocation/pinning (including process recovery, exec and coherent certificate/key capture); generation/binding/identity equality; check-source retention and timestamp freshness; final host policy; shared publication, refresh and revocation ordering; transport placement; and public projection redaction. Connection, AuthProfile, request and permission-target entity relations await their owner models rather than guessing ownership/cardinality. The shared generation relation is explicit and references exactly one immutable capture; capture's expected identity is not validated identity.

## Verification results

Runtime cases: executed **0 → 0**, red **0**. ESS has no Connectors runtime conformance target in this repository. The unchanged runtime count is intentional scope, not evidence that a runtime fix passed.

| Compiler lane | Baseline → unit | Exit |
|---|---|---|
| ESS declarations | 50 → 66 | 0 |
| Authored F05 scenarios | 0 compiled with 5 missing-declaration refusals → 5 compiled with 0 refusals | red 1, final 0 |
| Generated structural scenarios | 52 → 67 | 0 |
| Combined synthesis on this unit | 52 (0 authored) → 72 (5 authored) | 0 |
| Existing operations authored scenarios | 15 → 15 | 0 |
| Whitespace validation | `git diff --check` | 0 |

Authored and generated scenario counts come from the compiler summaries below; the generated final count is `72 - 5 = 67`. No Rust file changed in this unit, so the full Rust gate belongs to the coordinator's integrated branch. Unit commands use its own scratch as `TMPDIR`, `CARGO_BUILD_JOBS=2`, and no shared `CARGO_TARGET_DIR`.

### Red authoring run, before declarations

The account-replacement acceptance case was written first. Its initial compiler refusal was preserved before the model existed; this is a structural missing-declaration failure, not a failed runtime identity test.

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/evidence/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/red-authored.json
refusal[ESS-AUTHOR-005]: `connectors.credential_evidence/authored/configured-replacement-identity` in contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
  `connectors.credential_evidence.DispatchAdmission` is not an entity this specification declares
  help: name an entity the specification declares; an authored scenario acts on the model's own instances and invents none
refusal[ESS-AUTHOR-006]: `connectors.credential_evidence/authored/configured-replacement-identity` in contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
  `connectors.credential_evidence.InspectGeneration` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.credential_evidence/authored/configured-replacement-identity` in contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
  `connectors.credential_evidence.RejectAdmission` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.credential_evidence/authored/configured-replacement-identity` in contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
  `connectors.credential_evidence.DispatchGeneration` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-012]: `connectors.credential_evidence/authored/configured-replacement-identity` in contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
  `connectors.credential_evidence.AdmissionStates` is not a view this specification declares
  help: name a view the specification declares
0 authored scenario(s) from 1 file(s), 5 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/red-authored.json
exit: 1
```

### Baseline compiler output

```text
$ .local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/baseline-ir.json
connectors v1 — 7 file(s), 50 declaration(s), compiled to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/baseline-ir.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/baseline-operations.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/baseline-operations.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/baseline-synthesized.json
52 scenario(s) (0 authored), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/baseline-synthesized.json
exit: 0
```

### Final unit gate output

```text
$ .local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 7 file(s), valid
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/ir.json
connectors v1 — 7 file(s), 66 declaration(s), compiled to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/ir.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/evidence/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/authored.json
5 authored scenario(s) from 5 file(s), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/authored.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/auth/evidence/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/synthesized.json
72 scenario(s) (5 authored), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/synthesized.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/operations-regression.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/operations-regression.json
exit: 0

$ git diff --check
exit: 0
```

The coordinator will preserve separate adversarial review and the combined full gate in the wave's AEP evidence. This unit report does not pre-claim either result.

## Adversarial extension

A separate adversary added [retained-lineage-expires-before-dispatch](scenarios/retained-lineage-expires-before-dispatch.yaml) and returned no findings in `review-result:auth-evidence-adversary-p1-20260908`. The original unit command outputs above are historical. After the addition, the private author lane compiled 6 scenarios and synthesis compiled 73 (6 authored), with zero refusals; each command exited0. The existing operations lane remained15. Runtime execution remained0. The immutable review contains the exact commands, outputs and source-write audit; the [wave record](../../../../.engineering/planning/specification/auth-hardening-wave-20260908.md) owns combined gate evidence.
