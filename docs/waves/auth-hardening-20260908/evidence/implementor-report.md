unit: story:contracts-credential-evidence
verdict: green
cases: executed 0→0, red 0
origin: n/a
wrote-outside-worktree: none
needs-coordinator: yes

Runtime executed counts did not move: this is the approved semantic-only adaptation. No Connectors runtime conformance target or auth implementation exists here. Five authored F05 scenarios compile; compiler success does not prove credential capture, identity checks or dispatch atomicity.

1. Unit and acceptance

Bind readiness evidence to the credential actually dispatched. Acceptance: every credential-replacement scenario uses evidence valid for the dispatched credential identity. The normative rule now requires exact generation/binding evidence, separately admitted replacement validation, immutable material pinning, a final current-generation/authority check, and refusal on identity drift or unvalidated replacement. New publication cuts off old pending admissions, and unresolved possibly-consuming refresh also withholds old dispatch.

2. Observed diff

`git --no-pager diff --stat` (tracked modifications; new authored/verification files follow in status):

```text
 contracts/auth/capability/v1alpha1/semantics.md |  18 ++-
 contracts/auth/connection/v1alpha1/semantics.md |  15 +-
 contracts/auth/evidence/v1alpha1/semantics.md   |  70 +++++++++-
 docs/adapters/kubernetes.md                     |   7 +
 ess/domains/credential_evidence.yaml            | 174 +++++++++++++++++++++++-
 5 files changed, 265 insertions(+), 19 deletions(-)
```

`git status --short --untracked-files=all`:

```text
 M contracts/auth/capability/v1alpha1/semantics.md
 M contracts/auth/connection/v1alpha1/semantics.md
 M contracts/auth/evidence/v1alpha1/semantics.md
 M docs/adapters/kubernetes.md
 M ess/domains/credential_evidence.yaml
?? contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
?? contracts/auth/evidence/v1alpha1/scenarios/revocation-before-dispatch.yaml
?? contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml
?? contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml
?? contracts/auth/evidence/v1alpha1/scenarios/unvalidated-configured-replacement.yaml
?? contracts/auth/evidence/v1alpha1/verification.md
```

Scope checked: all four cited prose paths contain the reported credential-placement/evidence/rotation surfaces. The inferred private ESS file was the expected reserved domain, and the three inferred required scenario paths plus verification file were absent and were created. None of those inferred edit locations proved incorrect. Added precise adversarial cases `unvalidated-configured-replacement.yaml` and `revocation-before-dispatch.yaml` in the assigned scenario directory; coordinator was notified to record both inferred scope paths. Shared credentials/system/gate files, all planning files and runtime code were left untouched. No mechanical code fix was asserted: the preserved initial red is a compiler missing-declaration refusal under the approved semantic adaptation.

Owned diff hunk headers:

```text
diff --git a/contracts/auth/capability/v1alpha1/semantics.md b/contracts/auth/capability/v1alpha1/semantics.md
@@ -37 +37 @@ HttpCapability
@@ -58 +58 @@ InboundVerifier
@@ -67 +67,2 @@ Every capability carries the binding it was created for:
@@ -72 +73 @@ Every capability carries the binding it was created for:
@@ -94 +95,4 @@ Every capability carries the binding it was created for:
@@ -106,0 +111 @@ Every capability carries the binding it was created for:
@@ -116 +121,2 @@ Every capability carries the binding it was created for:
diff --git a/contracts/auth/connection/v1alpha1/semantics.md b/contracts/auth/connection/v1alpha1/semantics.md
@@ -78 +78 @@ Errors: base `ErrorCode` plus `connection_not_ready` (state is not `ready`; mess
@@ -84 +84,2 @@ Errors: base `ErrorCode` plus `connection_not_ready` (state is not `ready`; mess
@@ -100 +101,2 @@ Positive:
@@ -104,0 +107,2 @@ Adversarial (`docs/design.md:985`, repair without identity drift):
@@ -118 +122 @@ Adversarial (`docs/design.md:985`, repair without identity drift):
@@ -128 +132,2 @@ Adversarial (`docs/design.md:985`, repair without identity drift):
diff --git a/contracts/auth/evidence/v1alpha1/semantics.md b/contracts/auth/evidence/v1alpha1/semantics.md
@@ -36 +36 @@ Discovery, readiness, authentication, authorization, and execution are separate
@@ -46,0 +47,2 @@ Discovery, readiness, authentication, authorization, and execution are separate
@@ -52 +54,3 @@ Operation-time predicate (host-internal):
@@ -65 +69 @@ admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(rea
@@ -73 +77 @@ admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(rea
@@ -75,0 +80,40 @@ admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(rea
@@ -81 +125 @@ admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(rea
@@ -92,0 +137,8 @@ admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(rea
@@ -103 +155,3 @@ admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(rea
@@ -109 +163,3 @@ admit(connection, operation, input) -> Admitted(evidence_snapshot) | Refused(rea
diff --git a/docs/adapters/kubernetes.md b/docs/adapters/kubernetes.md
@@ -66,0 +67,6 @@ Rebuild adds what the old integration had and the first slice did not: pod logs,
@@ -100,0 +107 @@ Extends the implemented schema:
diff --git a/ess/domains/credential_evidence.yaml b/ess/domains/credential_evidence.yaml
@@ -2 +2,173 @@ domain: connectors.credential_evidence
diff --git a/contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml b/contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
@@ -0,0 +1,44 @@
diff --git a/contracts/auth/evidence/v1alpha1/scenarios/revocation-before-dispatch.yaml b/contracts/auth/evidence/v1alpha1/scenarios/revocation-before-dispatch.yaml
@@ -0,0 +1,281 @@
diff --git a/contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml b/contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml
@@ -0,0 +1,160 @@
diff --git a/contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml b/contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml
@@ -0,0 +1,210 @@
diff --git a/contracts/auth/evidence/v1alpha1/scenarios/unvalidated-configured-replacement.yaml b/contracts/auth/evidence/v1alpha1/scenarios/unvalidated-configured-replacement.yaml
@@ -0,0 +1,150 @@
diff --git a/contracts/auth/evidence/v1alpha1/verification.md b/contracts/auth/evidence/v1alpha1/verification.md
@@ -0,0 +1,111 @@
```

3. Red authoring evidence, before declarations

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

4. Unit gate and lane counts

Runtime lane: executed 0 → 0, red 0; no runtime runner invoked. Compiler lanes: authored F05 0 → 5 (initial 5 refusals, final 0); generated structural 52 → 67; combined synthesis 52 (0 authored) → 72 (5 authored); existing operations authored 15 → 15. ESS declaration count 50 → 66. The compiler's own summaries and each command's exit status:

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

Relative-link paths: checked 37, broken 0. Final `git diff --check` exit 0 after writing verification. No Rust changed, so no Rust test/build/full gate was run in this unit. Baseline compiler commands and exact outputs are preserved in `.local/waves/auth-hardening-20260908/evidence/baseline.log` and in the tracked verification document.

5. Semantic decision matrix and coordinator work

| Class | Rule | Evidence |
|---|---|---|
| Configured account A → B | Ordinary replacement refuses; cached A evidence cannot authorize B, even with unchanged filename/ref/revision | configured-replacement-identity and unvalidated-configured-replacement |
| Same-account publication between admission and dispatch | Old admission terminally refuses; fresh admission can dispatch the new validated generation | rotation-before-dispatch |
| Same-account refresh | New presence/validity and scope checks; only explicitly proven identity lineage may retain original time bounds; permission/verification invalidated | same-identity-refresh (fake profile) |
| Revocation, expiry or unresolved rotating refresh | Pin refuses; it cannot be reopened to dispatch | revocation-before-dispatch |
| Exec/certificate substitution | Use one validated helper output or coherent cert/key capture; no second helper/read at dispatch | normative capability/Kubernetes prose; runtime fixture required |
| Public projection | No secret/version/generation/snapshot references | public example corrected; runtime redaction tests required |

Coordinator must run adversarial review and the full combined gate, manage the two extra typed scenario scopes, persist review/planning evidence, commit/merge the unit, and own worktree disposition. No shared-file patch was necessary. Known remaining `UNMAPPED` guarantees are explicitly documented in `ess/domains/credential_evidence.yaml` and `contracts/auth/evidence/v1alpha1/verification.md`: trusted observation, generation/binding/identity equality, exact permission targets, freshness and transfer predicates, live policy, immutable transport capture, atomic publication/revocation/refresh cutoff, and redaction. Connection/AuthProfile/request relations await their actual owner models. Evidence remains a Connection value; DispatchAdmission is transient and does not invent a persistence port.

No runtime host implementation, wire/versioning migration, generated file, integration client, AEP mutation or commit was performed. No failing check was removed or weakened. Initial scenario shape acquired the newly required per-check provenance fields without changing its account-mismatch refusal or no-dispatch expectation.

6. Worktree handoff

Worktree id: `connectors-v2-auth-evidence-20260908`.
Path: `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908`.
Branch: `impl/contracts-credential-evidence`, opening base `1e567571d9ac62070933c7099b93a4e030613e58`.
Evidence: tracked `contracts/auth/evidence/v1alpha1/verification.md`; private scratch `.local/waves/auth-hardening-20260908/evidence/` (brief, red/baseline/unit-gate logs, compiled JSON and this exact report). Build target was not used. Wanted changes remain unstaged. Next owner: coordinator. Only the implementor's own `codex-auth-evidence-impl-20260908` lease is released at handoff; the coordinator lease is untouched. No directories were cleaned or removed, and no publication is claimed.

Outside-worktree artifact paths: none. Worktree CLI lease bookkeeping is the required managed lifecycle operation, not an external deliverable.
