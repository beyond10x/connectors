# Auth access and permission-budget verification — 2026-09-08

This local specification checkpoint addresses F09/E10 (anonymous/parent-authenticated access), F08 (permission budgets) and E09/E29 (acquisition paths). Baseline: `1d63519d3a98845e31311749ab40cbc4532193f6`. Root is the sole tracked-file editor in primary main under the repository override. Two independent read-only reviewers retain their own source snapshots and reports. No runtime code or adapter-kind schema is changed.

Normative owners: [profile §4.2](../../../contracts/auth/profile/v1alpha1/semantics.md#42-explicit-access-bindings-without-child-credentials), [acquisition §4.0](../../../contracts/auth/acquisition/v1alpha1/semantics.md#40-acquisition-paths-and-required-declarations), [evidence §4.4](../../../contracts/auth/evidence/v1alpha1/semantics.md#44-exact-authorization-targets-and-fan-out-budget-f08), with capability/connection/management, mediated route/discovery, adapter examples and service compatibility aligned to those rules.

| Check | Observed result and limits |
|---|---|
| ESS 0.20.0 validate/compile | 12 files valid, 183 declarations. New auth_access domain contains 15 values, no entity or executable admission/flow/budget lifecycle. |
| Deterministic projection | Two runs produced 191 byte-identical artifacts. Fifteen unmodified type schemas and hashes retained in [projection-manifest.json](projection-manifest.json). |
| Type expectations | [50/50 matched](type-results.json), including seven rejected invalid shapes. Four deliberately shape-valid semantic contradictions are accepted: invalid anonymous/bearer combination, complete denied coverage, over-ceiling budget and missing code endpoints. Those predicates are not schema-verified. |
| Textual scenarios | [44 declared traces](traces.md) cover placement, current authority, no fallback, exact target/cache equality, budget boundaries, coverage/refusal, static activation, OAuth requirements and reserved support. These are expected consequences for independent review, not executed runtime tests. |
| Full existing gate | [Exit 0](gate.log): 50 existing Rust tests, formatting/lint/boundary checks and Rust 1.88 MSRV check passed. This protects existing implementation; it does not prove the new semantics run. |
| Gate conformance authoring/synthesis | 222 compiled scenarios, 34 authored cases, zero refusals. No sequential ESS execution is claimed. |
| Separate session compilation | 13 authored cases accepted; 201 synthesized including them, zero refusals. The gate does not collect sessions, so this was checked separately. |

The [one-off transcript](fixture-audit.md) only constructs declared values, checks the generated schemas and compares projection bytes. It is verification evidence, not installed executable project tooling. Fixtures under vendor.example are invented examples, not provider source evidence. ESS Optional uses omission rather than null; ESS Timestamp is not the service's Unix-millisecond codec. Generic projections are not new accepted configuration or payload readers.

Explicit UNMAPPED obligations include mode/flow field combinations, source adequacy and provider interpretation, current configuration/route/parent generation equality, transport pinning and revocation, exact private cache keys, target normalization, numeric ceilings and call consumption, current clocks/authority, coverage completeness, cursor invalidation and strict public readers. Non-material transport/admission values are not coerced into CredentialGeneration/DispatchAdmission. Persistent Connection, Acquisition, AuthProfile and route/cache owners remain with their dedicated modeling work. No profile may advertise support solely because its vocabulary or shape validates.

Verification commands (exit 0):

```sh
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --format json
.local/toolchains/ess/0.20.0/bin/ess generate --path ess --kind schema --out .local/auth-profile-budget-20260908/schema-a
.local/toolchains/ess/0.20.0/bin/ess generate --path ess --kind schema --out .local/auth-profile-budget-20260908/schema-b
python3 .local/auth-profile-budget-20260908/value-audit.py
env TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked --offline -p connectors-build -- --ess "$PWD/.local/toolchains/ess/0.20.0/bin/ess" gate --msrv
.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/auth-profile-budget-20260908/sessions-authored.json
.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/auth-profile-budget-20260908/sessions-suite.json
```

Initial and subsequent review reports are immutable. Their findings and corrections are recorded in [dispositions](dispositions.md); final input hashes and gate provenance are retained at closure. Later lifecycle/ledger updates do not change normative source approval. The gate ran against the final ESS values and existing code; subsequent textual clarification and AEP evidence updates are separately reviewed/validated.
