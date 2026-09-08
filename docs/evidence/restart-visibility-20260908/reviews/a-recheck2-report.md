approve

Independent reviewer A approves **story:contracts-restart-idempotency (F11/E08)** and **story:contracts-mutation-visibility (E12)** for this normative specification packet. **Zero residual/new findings**. RV-A-01 and RV-A-02 are resolved, and the corresponding initial MP-A-03–07 requirements are addressed. Classification remains its separately approved checkpoint; this verdict does not claim completion of the broader specification goal or authorize runtime implementation.

## Reviewed immutable inputs

Before analysis, **66 current normative/evidence files** were frozen under `sources/` with exact hashes/sizes in `source-hashes.json`. `comparison.json` identifies changes from recheck1. Review archives, peer outputs and mutable AEP/ledger/dispositions/checkpoint files are excluded. No peer report/archive was opened or peer contacted.

The two previously frozen 222-artifact projections were copied into this immutable lane and checked again against the current manifest and seven selected schema files; `projection-source-hashes.json` covers all **444 files**. This is reuse and verification of prior generated evidence, not a new reviewer compiler run. Three exact before/after/gated IR inputs are preserved under `compiled-sources/` with `compiled-source-hashes.json`. Common provider archives were hash-verified before their source contents were inspected.

## Findings resolved and semantic conclusions

**RV-A-01 — mutation-visibility, resolved.** Operations §4 steps 1–3 now limit initial validation to bounded transport/authentication/version/envelope syntax and syntactic identifiers. Any early target resolution is internal and nondisclosing. Current policy precedes revision/existence information, and admitted fresh bound/enabled lookup precedes operation-input validation and then key inspection. This agrees with service v1alpha2 §3.2.1, including denial before hidden schema errors and no original-outcome disclosure on pre-key input refusal. The corrected sequence preserves candidate-only approval/preflight, current F02 replay admission and the unchanged legacy descriptor-vector behavior.

**RV-A-02 — restart-idempotency, resolved.** Docker §4.1 now returns `desired_state` with `acknowledged` or `already_satisfied`, explicitly identifying requested intent rather than measured/current state. Operations §5.2 uses the same acknowledgement meaning. Process exit or restart-policy interference before HTTP response is expressly included, without an added inspect/poll or a health/liveness guarantee. Applied still describes the selected endpoint's accepted intent, including an already-satisfied check, not invented physical change or historical resolution of a lost request. The existing stable full-ID, exact name/expected-ID, one bounded admitted inspect, fixed configuration options and no-resend constraints remain coherent.

**Additional Kubernetes conditional-version correction, accepted.** Kubernetes §4.1 now selects canonical positive ASCII decimal resource_version in `1..18446744073709551615`, rejects zero/zero aliases, noncanonical spellings and overflow at admitted input validation, and copies accepted bytes unchanged. This restriction is specific to the selected built-in Deployment binding. It neither chooses a newer version nor supplies an implicit refetch/rebase.

The newly pinned common source path supports that selection: Deployment strategy permits unconditional update; APIObjectVersioner parses resourceVersion as uint64; the generic store substitutes current version for parsed zero under that strategy and otherwise compares parsed values; strategic PATCH reaches that update path. Canonical positive admission avoids the zero branch and alternate lexical representations. The source evidence is not generalized to every API/binding or all admission-plugin behavior. Existing exact UID, fixed prepared marker/body, keyed replay, accepted-patch result, uncertain lost outcome and no-reconciliation/no-resend rules remain intact. No new concrete defect was found in these bounded corrections.

## Independent evidence checks

`independent-checks.json` records the following read-only checks:

- **46/46 schema expectations** match recorded results: **9 shape negatives**, **19 intentionally shape-accepted semantic counterexamples**. All **14 added version examples** were independently compared with the selected canonical positive decimal/range rule: two valid boundaries and twelve invalid forms. This is an offline normative-example audit, not execution of a receiver/provider validator.
- Both frozen **222-artifact projections**, the manifest and **seven unchanged selected schemas** match exactly.
- All **eight provider archives** match their uncompressed SHA-256 values and sizes. The nine-endpoint Docker declaration audit remains exact. Kubernetes strategy/versioner/store/PATCH sources were read directly from this common frozen packet; no peer report was used as evidence.
- All compressed compilation artifacts match their manifests. The before-comment, after-comment and gated IR are byte-identical with SHA-256 `32e87a8fe96a9aa984f205bc847db54510b5596143a7a3ccbce99526fac1d2bb`.
- All ESS YAML semantic contents are unchanged from recheck1; the sole ESS text change is the explicit UNMAPPED comment. Existing entity fields/relations/lifecycles and seven private value shapes remain unchanged. Reviewed legacy runtime bytes are unchanged.
- The **48 textual RI/VI cases** are unique and consistent with the corrected owners. They are explicitly not additional runtime tests.
- The unchanged recorded gate reports **50 passing existing Rust tests**, MSRV 1.88, **13 ESS files / 215 declarations**, and **222 compiled scenarios including 34 authored**, with no refusals. Separate exact session artifacts retain **13 authored / 201 total compiled cases**. This review did not rerun the gate or scenario compiler.

## Limits

No provider operation, runtime suite, real policy/clock/storage race, state transition or network call was run. Source interpretation supports the selected version mechanism, not live provider conformance, arbitrary server/plugin behavior or physical daemon/cluster continuity. Schema checks and byte-identical IR do not execute canonical version admission, identity/permission checks, fixed-marker generation, atomic reservation or visibility/refusal predicates. Those implementation/binding obligations remain explicit and must be established before advertisement. The review made no tracked source or planning edits.

```findings
[]
```
