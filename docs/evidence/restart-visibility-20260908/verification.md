# Restart and visibility verification — 2026-09-08

Owners: `contracts-restart-idempotency` (F11/E08) and `contracts-mutation-visibility` (E12). Baseline: 3fc56bddb3a53a3e1320ecc2bfdf8a2a45952e23. Root is the sole tracked editor. Initial independent findings are preserved in the preceding mutation-profile packet: A03–06/B03–05 for restart, A07/B06 for visibility. Classification was closed separately and is not re-claimed here.

## Normative textual traces

These are declared expectations audited against [operations](../../../contracts/operations/v1alpha1/semantics.md#52-exact-lifecycle-intent-and-repeat-guarantees-f11e08), [Docker](../../adapters/docker.md#41-target-intent-and-lifecycle-results), [Kubernetes](../../adapters/kubernetes.md#41-rollout-restart-intent-and-replay), [extended visibility](../../../contracts/service/v1alpha2/semantics.md#321-visibility-lookup-and-refusal-precedence-e12), and [legacy projection](../../../contracts/service/compatibility.md#3-legacy-projection-and-federation-intersection). They are not executed provider, policy or race tests.

| ID | Trace | Expected observation |
|---|---|---|
| RI01 | Docker start on exact stopped ID, then start without intervening change | acknowledged, then already_satisfied; natural desired-state fulfillment/applied, no historical replay guarantee |
| RI02 | Docker stop, then stop without intervening change | acknowledged, then already_satisfied; applied intent, no invented physical change count |
| RI03 | Docker restart then deliberate restart again | none; another independently admitted restart can occur |
| RI04 | Process exits, external actor or restart policy intervenes between start/stop calls | another state change is possible; natural promise is qualified by exact target/intent and provider state |
| RI05 | A later 304, inspect or successful lifecycle call follows an earlier lost reply | observation of the later request/state does not resolve the earlier effect |
| RI06 | Name resolves to expected full ID and configured membership permits it | one bounded exact-ID preflight; business request sent to fixed full ID |
| RI07 | Name recreated, expected ID absent or name/ID mismatch | no retarget/fallback; pre-gate refusal, no business dispatch |
| RI08 | Name/label changes after preflight but same ID remains | provider-atomic membership is not claimed; approved stable identity cannot change |
| RI09 | Signal/wait/configuration changes after approval or before gate | changed semantic binding must be re-admitted; old preparation cannot gain dispatch |
| RI10 | Inspect consumes remaining stop-wait budget, or preflight cannot prove authority/target | refuse before gate; no extended deadline or business attempt misclassification |
| RI11 | Post-gate timeout, malformed answer or insufficient effect evidence | unknown/outcome_unknown; no automatic resend |
| RI12 | None/natural invocation supplies a host key | invalid_input; it cannot opt into keyed replay |
| RI13 | Kubernetes same full live host key/fingerprint | observe original; no fresh marker/PATCH/approval spend |
| RI14 | Same live key with changed UID/version/target/binding | idempotency_conflict, no axis or stored-input disclosure |
| RI15 | New candidate with original UID/resourceVersion | exact fields stay in strategic merge PATCH; marker fixed once before anchored dispatch |
| RI16 | Concurrent candidate loses reservation | discard its preparation; observe winner, no second marker used for a business send |
| RI17 | Successful patch advances version, later new key retains original version | later precondition may refuse; that refusal cannot prove which actor changed the version |
| RI18 | First reply lost, current version still equals original | another independently admitted new-key request might apply; not every repeat necessarily conflicts or rolls again |
| RI19 | Same name now has another UID | no replacement restart; retain immutable UID guard |
| RI20 | Deliberate new read/version/input/key/approval | new intent; no refetch/rebase/retry performed by the original mutation |
| RI21 | Successful exact acknowledgement with equal already-present marker | patch_accepted/applied intent; no unique timestamp or fresh rollout/healthy Pods claim |
| RI22 | Expired known-result key, quarantined unknown, or later marker readback | F02 distinct retention rules; no unspending, automatic reconciliation or unknown-key reuse |
| RI23 | Correct UID with version 0, 00 or another all-zero spelling | admitted input validation refuses invalid_input before key/preflight/PATCH; zero cannot activate unconditional update |
| RI24 | Positive version with leading zeros, sign, whitespace, radix prefix, non-ASCII digits or unsigned 64-bit overflow | refuse instead of normalizing or refetching; this selected binding accepts only canonical positive ASCII decimal within its bound |
| RI25 | Versions 1 and 18446744073709551615, or another admitted canonical positive version | valid lexical/range input is copied unchanged; provider still decides conditional equality, so shape or syntax acceptance does not prove the object/version exists |
| RI26 | Process exits after start action, or restart policy changes state after stop action, before HTTP acknowledgement | applied acknowledgement retains desired_state/disposition; no synchronized current-state or health claim and no extra inspect |
| VI01 | Unknown/unimplemented/unbound/unsupported binding with admitted lookup and current revision | omitted; not_found, no provider/handler/key lookup |
| VI02 | Implemented bound disabled operation, admitted lookup/current revision | omitted; forbidden, no mutation/original-result disclosure |
| VI03 | Current host lookup/operation/target observation policy denies or executor mismatches | not_granted/forbidden as applicable before stale/existence/key/approval disclosure |
| VI04 | Current required policy unavailable | unavailable; no cached-authority/static-policy fallback or successful admitted projection claim |
| VI05 | Metadata scope admits operation but selected connection observation scope denies | metadata may remain visible; invocation forbidden, no target/result disclosure |
| VI06 | Enabled/admitted operation lacks new-attempt readiness | visible; candidate preflight uses exact eligibility-specific refusal, no provider describe probe |
| VI07 | Enabled/admitted operation lacks fresh one-shot approval | visible with requirement; candidate approval_required after any exact-key observation |
| VI08 | Exact retained key, current result authority, unavailable new-attempt dependencies | observe original without provider preflight/marker generation/fresh approval |
| VI09 | Current authorized/ready new attempt | normal profile/audit/approval/gate sequence, not dispatch authority from describe alone |
| VI10 | Old descriptor after disable, current lookup authority retained | stale_description before private disabled lookup |
| VI11 | Current descriptor omits disabled ID; caller guesses it under admitted scope | forbidden via future private registry; no direct-handler bypass |
| VI12 | Old descriptor and lost current lookup/result authority | denial/unavailable wins without stale/original disclosure |
| VI13 | Credential bytes rotate for same static service identity | no fabricated per-person identity or automatic semantic revision change |
| VI14 | Two people share static bearer | same configured principal/policy scope; no individual isolation claim |
| VI15 | Gateway alias/broad token would expose denied or unbound leaf operation | intersect current delegated metadata/bindings; no widening/fallback; unavailable if required current leaf admission cannot be established |
| VI16 | Guessed hidden legacy mutation or disabled legacy hosts.discover | current legacy eligible vector lookup is not_found; no extended private-registry bypass |
| VI17 | Legacy request revision is old | existing server returns stale_description before descriptor-vector lookup |
| VI18 | Call direct Kubernetes adapter guard with disabled hosts.discover | internal forbidden remains distinct from the public not_found path |
| VI19 | Pre-result-admission refusal before a possibly existing key lookup | omit mutation metadata; absence is not not_attempted |
| VI20 | State/profile/visibility claims supplied only as ESS values | type acceptance does not establish authoritative facts, predicate results or actual dispatch behavior |
| VI21 | Unauthorized caller supplies malformed operation-specific input for a hidden operation | current denial precedes operation-schema or private lookup error; only bounded envelope/authentication/version/syntactic checks and nondisclosing policy resolution may precede admission |
| VI22 | Admitted current caller targets enabled/bound operation with invalid conditional version and a possibly existing key | operation-input refusal before key observation; no mutation/original-result disclosure, approval spend or provider preflight |

## Model and executable-check boundary

Seven private values are added: IdempotencyKind, ContainerLifecycleAction, ContainerLifecycleIntent, DeploymentRestartTarget, PreparedDeploymentRestart, OperationAvailability and AvailabilityDecision. No new entity, lifecycle, declaration entity field, public descriptor member or persistent ownership relation is asserted. Provider-resource identities remain qualified opaque coordinates; unsupported target/marker/parameter/availability predicates are explicitly UNMAPPED. Generic Optional values mean omission, not a public required-null encoding. A decision with no lookup_error allows later checks only, never dispatch.

Pinned ESS 0.20.0 baseline validates 13 files / 208 declarations; revised validates 13 / 215. Two projections contain **222 identical ordered artifacts**; seven copied schema files are unmodified output. [46 shape expectations](typed-values.json) include **9 rejected shapes** and **19 accepted semantic counterexamples**; [actual results](shape-results.json), [projection hashes](projection-hashes.json) and [one-off transcript](check-transcript.md) record what ran. The counterexamples demonstrate that generic schemas do not enforce bounds, signal/action coupling, full target identity, marker provenance, valid conditional resourceVersion or visibility/admission logic.

[Provider evidence](provider-evidence.md) preserves eight exact source files and a nine-endpoint Docker declaration audit. Old implementation/code inspection and current v1.35 source/documentation support bounded source claims; no provider effect/race was executed. Existing core/server/Kubernetes adapter code and adapter schemas remain unchanged.

The existing full gate/MSRV command and its result are recorded in [gate.log](gate.log). It tests the existing implementation and compiles ESS/scenario references; it does not implement or execute these new provider/policy/visibility rules. Final observed counts and independent recheck records are added to the checkpoint after completion; a pending process or a textual expectation is never a passing runtime conformance result.

Observed gate: **exit 0**, one run, **50 existing Rust tests**, MSRV **1.88.0**, ESS **13 files / 215 declarations**, **222 compiled scenarios including 34 authored**, no refusals. Command:

```console
env TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked --offline -p connectors-build -- --ess "$PWD/.local/toolchains/ess/0.20.0/bin/ess" gate --msrv
```

The gate does not collect sessions, so separate ESS author/synthesize commands checked that directory: **13 authored / 201 total compiled cases**, no refusals. [Compiled evidence](compiled-evidence.json) hashes the exact IR and scenario JSON preserved as gzip. Compilation does not execute sequential scenarios. The 48 RI/VI rows above are textual expectations, not additional passing runtime tests.

The narrow correction recheck validates ESS again and compiles byte-identical IR before/after the added UNMAPPED comment and against the gated IR ([exact result](recheck2-validation.json)). Runtime/scenario sources and generated shapes are unchanged; the existing full gate was not rerun. The evidence checker was rerun after adding the four pinned provider files and fourteen version-input shape cases; all 46 expectations match. The twelve new schema-accepted invalid-version cases document a normative predicate that the generic String does not execute.
