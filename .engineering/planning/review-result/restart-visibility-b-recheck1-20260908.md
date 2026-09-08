---
format: aep.planning-md/1
id: review-result:restart-visibility-b-recheck1-20260908
kind: review-result
status: active
title: Independent restart and visibility review B recheck1
relations:
- reviews: story:contracts-restart-idempotency
- reviews: story:contracts-mutation-visibility
revision: 1
---
needs-revision

story:contracts-restart-idempotency — RV-B-01 (P1): reject unconditional or noncanonical resource_version inputs before dispatch so the selected Deployment PATCH actually enforces its promised exact version guard. — docs/adapters/kubernetes.md:61

This independent recheck covers restart-idempotency F11/E08 (initial MP-B-03/04/05) and mutation-visibility E12 (MP-B-06), against the working packet based on 3fc56bddb3a53a3e1320ecc2bfdf8a2a45952e23. **One remaining P1 finding; zero other findings. Mutation-visibility is approved within this review; restart-idempotency needs the correction below.** Classification remains separately closed.

## RV-B-01 — Conditional version input can disable the provider guard

Severity: **P1**. Sole owner: **story:contracts-restart-idempotency**. Origin: introduced by the revised closed-input/precondition claim.

Kubernetes §4.1 admits every nonempty exact resource_version string (`docs/adapters/kubernetes.md:61`), copies it into a strategic merge patch and promises exact equality (`:63–67`). Therefore `"0"` and `"00"` are admitted by the text, while `"00042"` is not excluded as a spelling distinct from `"42"`. The supplied ObjectMeta evidence checks nonempty/immutable fields but does not establish this promised version predicate.

At the **same pinned Kubernetes commit**, the official Deployment strategy enables unconditional updates ([strategy.go](https://github.com/kubernetes/kubernetes/blob/66452049f3d692768c39c797b21b793dce80314e/pkg/registry/apps/deployment/strategy.go#L148)). The [API object versioner](https://github.com/kubernetes/kubernetes/blob/66452049f3d692768c39c797b21b793dce80314e/staging/src/k8s.io/apiserver/pkg/storage/api_object_versioner.go#L73) parses resourceVersion as an unsigned decimal number. The [generic update store](https://github.com/kubernetes/kubernetes/blob/66452049f3d692768c39c797b21b793dce80314e/staging/src/k8s.io/apiserver/pkg/registry/generic/registry/store.go#L666) treats parsed zero as unconditional and substitutes the latest stored version; otherwise it compares parsed values, not arbitrary string spellings. The [strategic PATCH handler](https://github.com/kubernetes/kubernetes/blob/66452049f3d692768c39c797b21b793dce80314e/staging/src/k8s.io/apiserver/pkg/endpoints/handlers/patch.go#L700) delegates the patched object to that update path. This source-derived analysis is not a live provider test.

Thus a caller supplying the correct existing UID and resource_version `"0"` can request a patch that bypasses the selected optimistic-concurrency constraint. UID still pins the object, but cannot prove its version is the one approved. F02's host key only suppresses reuse of that host attempt; it does not repair the weakened provider predicate.

**Correction:** select a binding-specific conditional-version admission rule that excludes zero and all provider-equivalent zero spellings and makes the claimed equality meaningful. For the pinned implementation, a canonical strictly positive decimal representation with its supported bound is one option; a separately specified atomic equality mechanism is another. Keep accepted bytes unchanged, never refetch/rebase a replacement version, and do not import this provider-specific lexical choice as a universal rule for Kubernetes versions. Add textual negative cases for zero/all-zero aliases and a disposition for noncanonical spellings; retain the exact UID, fixed-marker, F02 and lost-response rules. The semantic gate must refuse an invalid conditional version before business dispatch. Generic schema acceptance alone remains explicitly insufficient.

Four supplementary official files are frozen under `supplemental-provider/` with SHA256/size/URL provenance in `supplemental-provider-hashes.json`. `rv-counterexamples.json` records shape acceptance of zero/alias examples; no provider effect or race was executed. This is a bounded input/guard correction, not a requirement to implement a backend or switch the entire mutation protocol.

## Initial finding disposition

- **MP-B-03 closed:** operations §5.2 now distinguishes no repeat guarantee, qualified natural desired-state fulfillment and receiver-keyed observation. No-op fulfillment does not claim a physical transition; current admission, approval, ledger/gate and no automatic resend apply independently.
- **MP-B-04 closed:** Docker selects natural start/stop versus none restart consistently. Exact source-qualified expected full ID, separately admitted bounded inspect, deliberate name ambiguity handling, fixed signal/wait configuration and original remaining budgets prevent silent retargeting. Snapshot allowlist membership is honestly distinct from an atomic daemon predicate. The pinned endpoint acknowledgements concern selected intent and do not promise future state, health, historical replay or resolution of an earlier lost response.
- **MP-B-05 substantially corrected, with RV-B-01 remaining:** Kubernetes selects host-keyed replay plus original UID/version, generates one fixed candidate marker, discards concurrent losers, never generates a new marker during replay, and treats accepted PATCH separately from rollout completion or marker uniqueness. Later conflict/readback cannot settle the original lost answer. The zero/unconditional version edge still defeats the advertised provider guard.
- **MP-B-06 closed:** the v1alpha2 matrix covers all three admission profiles, current metadata/lookup/result authority, stale revision precedence, admitted private disabled lookup, input/enablement/key ordering, candidate-only preflight, readiness-independent visibility and current federation intersection. Static bearer does not invent individual identity. Current legacy descriptor-vector behavior, hidden mutation refusal and the direct Kubernetes Forbidden guard are accurately distinguished. No hidden implementation or cached descriptor grants execution authority.

## Evidence verified

What I read: 57 frozen current source/evidence files; the relevant actual core/host/client/Kubernetes legacy paths; the supplied four pinned provider files; and four additional official Kubernetes source files for the concrete guard concern. Read-only Git/source inspection, bounded text/JSON/YAML/gzip inspection, Python jsonschema and exact hashes were used. Supplemental primary-source retrieval reused the already reported Connectors capability gap; no provider business operation or runtime suite was invoked. The findings serialization follows the installed planning skill's critic-rubric reference; no planning store was mutated.

`evidence-audit.json` records:

- Four vendor gzip payloads match declared hashes/sizes; all nine Docker path/method IDs, responses and parameters match the parsed v1.56 Swagger.
- **32/32** expected shape decisions match actual results, including **9 negatives and 7 accepted semantic counterexamples**.
- Two existing projections match all **222** ordered artifacts; all **7** selected schema copies are exact. Revised IR adds exactly the seven named values; old types/entities/commands/lifecycles remain unchanged (only domains/types differ).
- Three compiled gzip payloads match exact hashes/sizes. Separate sessions contain **13 authored / 201 synthesized** cases, matching provenance and unchanged authored entries.
- Supplied gate log records success with **50 existing Rust tests**, Rust **1.88.0**, **13 ESS files / 215 declarations**, and **222 compiled scenarios including 34 authored**. This reviewer did not rerun the gate.
- All **42 RI/VI rows** are textual expectations, not executed provider/policy/race conformance. They are accurately labeled but do not yet cover RV-B-01.
- All **57** frozen source/evidence files match their recorded hashes and the live files at audit completion. Peer reports/archives and mutable AEP/ledger/dispositions/checkpoints were excluded.

What I could not establish: provider-runtime conformance, a live conditional PATCH race, daemon state timing, actual policy evaluation, durable replay/fencing or new public codec execution. The specification accurately leaves these as implementation/advertisement prerequisites; this review does not add them as blocking implementation work. The single remaining defect is the concrete selected version-input guard above.

```findings
- file: docs/adapters/kubernetes.md
  line: 61
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "RV-B-01 (P1): reject unconditional or noncanonical resource_version inputs before dispatch so the selected Deployment PATCH actually enforces its promised exact version guard."
```
