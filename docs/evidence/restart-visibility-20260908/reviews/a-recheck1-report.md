needs-revision

Independent reviewer A recheck of **restart-idempotency (F11/E08)** and **mutation-visibility (E12)** only. **Two P2 residual/new findings**, zero P0/P1. Classification remains separately approved and is not reopened here.

## Findings

- **RV-A-01 (owner: story:contracts-mutation-visibility; P2): operations §4 steps 1–2 still validate input shape and resolve the operation/connection before current policy, conflicting with E12’s selected policy-before-revision/lookup/operation-schema order.** Source: `contracts/operations/v1alpha1/semantics.md:99–101`; compare `contracts/service/v1alpha2/semantics.md:76–78,94`. Narrow the early operation steps to bounded envelope/syntactic checks and internal nondisclosing resolution needed for policy; explicitly defer operation-schema validation and externally visible lookup/refusal decisions to the selected E12 order. The link in step 3 does not clearly override the preceding numbered steps. This matters for an unauthorized request whose malformed operation input could otherwise disclose a hidden schema/implementation before policy denial.
- **RV-A-02 (owner: story:contracts-restart-idempotency; P2): Docker start/stop results claim running/stopped as an endpoint state observation, but the pinned lifecycle acknowledgement supplies no synchronized state snapshot; process exit or restart-policy interference can occur before the response.** Source: `docs/adapters/docker.md:63–64`; compare `contracts/operations/v1alpha1/semantics.md:195` and the frozen Docker v1.56 Swagger lifecycle responses. Define these as acknowledged desired-state/command-completion facts (for example `desired_state` plus disposition), not measured/current state at acknowledgement. A valid 204 can preserve applied acknowledgement without promising the container is still running/stopped when the response is emitted. A valid 304 reports the endpoint's already-satisfied check, not a synchronized durable snapshot. Preserve the selected no-health/no-future-state/no-historical-replay limits; no extra inspect/poll is needed. The current caveat about changes between invocations does not cover changes between the provider action and its HTTP acknowledgement.

Both corrections are normative wording/output-meaning fixes, not requests for runtime implementation or vendor calls.

## What was reviewed

The packet was frozen before substantive analysis: **63 inputs** in `source-hashes.json`, plus the referenced current legacy Kubernetes adapter in `supplemental-source-hashes.json`. Normative sources, all ESS files, provider archives, schemas/value results, textual cases, gate and compiled archives are under `sources/`. Source hashes/sizes are exact. Two complete projection directories were subsequently frozen before checking their bytes, with **444 entries** in `projection-source-hashes.json`. Root's current edits after that freeze do not alter this report's inputs.

Read-only checks used source reads/diffs, Python JSON Schema validation, YAML parsing, gzip/tar hash inspection, and exact git-show comparisons against baseline `3fc56bddb3a53a3e1320ecc2bfdf8a2a45952e23`. The pinned old Kubernetes implementation matches reviewer A's own initial snapshot at `81459ac42ddd518d3942f4b079841e9e0ed6efc8`. No peer report/archive was opened; the old implementation in the provider packet was verified against that independent prior copy. Mutable AEP/ledger/dispositions/checkpoint files are outside this review.

## Earlier findings and sound selections

MP-A-03/06 are substantially addressed: Docker's natural start/stop versus none restart classification is consistent, none is no guarantee rather than inevitable effect, provider repeat assumptions are explicit, and unsupported keys cannot opt into replay. RV-A-02 remains about precise acknowledgement/result meaning.

MP-A-04 is addressed: Kubernetes retains exact source-qualified UID/resourceVersion, fixes a decimal timestamp/body once per prepared candidate, selects independent receiver-keyed replay, and reports only verified accepted patch intent. Same-key replay, changed preconditions, new intent and unknown original outcome remain separate; a later conflict or matching marker does not settle a lost response. Provider no-op/version behavior and concrete no-effect proof are appropriately qualified rather than inferred universally from a status code.

MP-A-05 is addressed: Docker names require expected full ID, one admitted bounded exact-ID inspect checks selector/membership, and lifecycle dispatch uses that fixed ID. Signal/wait options are receiver-owned and included in semantic configuration meaning. The documented non-atomicity of mutable membership between inspect and dispatch is explicit, and cannot authorize target substitution. F03 preparation does not acquire a hidden provider read.

MP-A-07 is substantially addressed: all admission profiles use the selected metadata projection; disabled, unknown/unbound, denied, unavailable policy, not-ready, missing approval, exact keyed observation and stale revision cases are distinguished. Current denial precedes private/existence/key information; metadata visibility and readiness remain independent; legacy vector lookup and its separate internal guard are correctly recorded. RV-A-01 is the remaining conflicting execution-sequence wording.

## Independently verified evidence and limits

`independent-checks.json` records:

- **32/32 shape expectations**, **9 rejected shapes**, **7 accepted semantic counterexamples** match the recorded results.
- **222 identical projection paths/bytes** in each run, and **7 unmodified selected schemas**, match the manifest.
- **Four provider archives** match their uncompressed hashes/sizes; the **nine Docker endpoint declaration audit** exactly matches the pinned Swagger. Kubernetes conditional update and immutable UID validation evidence supports the bounded mechanism claims, not every possible provider/admission-plugin outcome.
- Exactly **seven private values** are added. Existing ESS entity fields, identities, relationships, lifecycles and commands remain unchanged. Reviewed legacy core/server/Kubernetes runtime and adapter schemas are byte-identical to the baseline.
- Compressed compilation evidence hashes/sizes match; session artifacts contain **13 authored / 201 total** compiled cases. The gate records **50 passing existing Rust tests**, MSRV 1.88, **13 ESS files / 215 declarations**, and **222 compiled scenarios including 34 authored**, with zero refusals.
- All **42 textual cases** are uniquely recorded, and reviewer A's **57 initial archived inputs** match their own hash manifests. The two findings above constrain approval of the normative meanings those textual cases describe; their existence does not mean runtime predicates were tested.

No runtime suite, provider call, real policy, clock, atomicity, identity-attestation, state-transition or fault/race test was rerun. The evidence accurately says schemas and scenario compilation do not execute those predicates. The review found no new implementation or persistent entity graph being smuggled in by the private values. Exact provider/runtime conformance remains a binding prerequisite.

```findings
- file: contracts/operations/v1alpha1/semantics.md
  line: 99
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "RV-A-01 (owner: story:contracts-mutation-visibility; P2): operations §4 steps 1–2 still validate input shape and resolve the operation/connection before current policy, conflicting with E12’s selected policy-before-revision/lookup/operation-schema order."
- file: docs/adapters/docker.md
  line: 63
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "RV-A-02 (owner: story:contracts-restart-idempotency; P2): Docker start/stop results claim running/stopped as an endpoint state observation, but the pinned lifecycle acknowledgement supplies no synchronized state snapshot; process exit or restart-policy interference can occur before the response."
```
