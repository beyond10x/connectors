# Connection viability and management verification — 2026-09-08

This specification checkpoint addresses F07/E19/E21 (`story:contracts-connection-readiness`) and E04 (`story:contracts-management-boundary`). Baseline: `12c11f4b43cd6cf5ff67d3243017887b28788344`. No runtime, adapter schema or implementation changes are included. Root edited primary main alone under the repository override; two independent reviewers read the shared sources and retained immutable snapshots.

The normative owners are [connection §4.1](../../../contracts/auth/connection/v1alpha1/semantics.md#41-connection-viability-and-operation-eligibility), [profile §4.1](../../../contracts/auth/profile/v1alpha1/semantics.md#41-baseline-and-operation-requirements), [acquisition](../../../contracts/auth/acquisition/v1alpha1/semantics.md), [evidence](../../../contracts/auth/evidence/v1alpha1/semantics.md) and the shared [management boundary](../../../contracts/auth/management.md). Service compatibility keeps these selected payload/owner changes distinct from current wire/runtime support.

| Check | Observed result and scope |
|---|---|
| Full repository gate | Exit 0, 50 existing Rust tests passed, formatting/lint/boundary checks and Rust 1.88 all-target workspace check passed. This protects existing code, not an implementation of the new reduction or management flow. |
| ESS 0.20.0 validation/compile | 11 files valid; 167 declarations. The new domain contains typed status/facts/decisions, evidence-requirement and management-target values, with no fake lifecycle or authority commands. |
| Deterministic schema projection | Two directories each contain 176 byte-identical generated artifacts. Thirteen relevant copies and their hashes are retained in [projection-manifest.json](projection-manifest.json); no generated schema was hand-edited. |
| [Typed decision vectors](decision-vectors.json) | Sixteen viability cases, eighteen operation-eligibility cases and seven management targets are declared textual expectations over trusted facts. They are not observations of a running reducer, policy engine or store. |
| [Type checks](type-results.json) | 80/80 expected schema decisions match, including rejection of the retired global insufficient_scope state, scope_insufficient error typo, missing fact and extra target member. The generic schema also accepts a deliberately contradictory eligible/error pair: cross-value semantics are explicitly not verified by schema shape. |
| Gate conformance synthesis | 222 compiled scenarios, 34 authored operations/acquisition/evidence cases, zero refusals. No sequential execution is claimed. |
| Separate session check | 13 authored cases accepted and 201 synthesized scenarios including them, zero refusals; checked separately because the gate does not collect sessions. |
| [Textual management and publication traces](traces.md) | Sixteen cases cover the remaining owner/publication/repair/federation/action-disclosure conditions, including no-browser client credentials and exact wire selector absence. Independent reviewers assess these expected consequences against the final contracts. |

The [one-off inspection transcript](fixture-audit.md) constructs declared values and checks only actual ESS-generated type schemas. It implements no current-state reducer, callback protocol, provider integration, database, clock or runtime admission checker. Optional semantic fields use omission in the generated schema; status timestamps use ESS Timestamp values. That projection is not the required public Unix-millisecond spelling or a wire-null codec.

Persistent Connection, Acquisition and AuthProfile owners are still proposed/unmodeled; their previous tables no longer claim implemented ESS entities or lifecycles. Existing generation/evidence/refresh/attempt ownership and fences remain intact. Ordered reduction, exact alternative/subject/generation checks, current authority and time, publication/revocation atomicity, management approval/idempotency and protected codec/UI delivery remain explicit UNMAPPED or binding obligations. Their absence blocks advertisement; a value or generic envelope cannot supply missing authority.

Commands (all exit 0):

```sh
env TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked --offline -p connectors-build -- --ess "$PWD/.local/toolchains/ess/0.20.0/bin/ess" gate --msrv
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --format json
.local/toolchains/ess/0.20.0/bin/ess generate --path ess --kind schema --out .local/connection-semantics-20260908/schema-a
.local/toolchains/ess/0.20.0/bin/ess generate --path ess --kind schema --out .local/connection-semantics-20260908/schema-b
.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/connection-semantics-20260908/sessions-authored.json
.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/connection-semantics-20260908/sessions-suite.json
python3 .local/connection-semantics-20260908/value-audit.py
```

The semantic recheck corrected surviving blanket stale-evidence language, inactive repair-candidate poisoning, exact not_granted/forbidden mappings, absent requires_auth configured binding, the UI requirement for noninteractive flows, omitted versus null selectors, and repair wording for terminal revoked records. Initial and recheck reports remain immutable; final provenance is recorded separately at closure.
