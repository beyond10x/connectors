# Idempotency scope and replay admission — verification

Owner: `story:contracts-idempotency-scope`; source **F02** in `specification:contract-review-intake-20260908`. The source review remains immutable at `db1c329`. The prerequisite mutation-outcome hardening was committed as `34f298a` before this work began.

## Contract and model coverage

[Mutation §5.1](semantics.md#51-key-namespace-fingerprint-and-replay-admission) defines namespace, fingerprint, current replay admission, reservation ownership and retention. The [Atlassian adapter plan](../../../docs/adapters/atlassian.md) applies these rules to its keyed writes. These remain proposed semantics; no mutation runtime, public wire schema or adapter has been added.

[ESS idempotency](../../../ess/domains/idempotency.yaml) gives AuthorityScope, Origin, KeyNamespace and RequestFingerprint typed homes. KeyReservation references exactly one immutable [AttemptRecord](../../../ess/domains/mutations.yaml), and its lifecycle separates Pending, Replayable, Quarantined and Expired. The attempt records the durable terminal settlement instant used by replay retention. Audit evidence outlives eviction of replay data.

ESS checks identities, field types, relation endpoints, lifecycle causation and authored scenario references. It does not execute namespace derivation/equality, current policy, atomic unique indexes, cross-entity transactions, state coupling, clock assignment/arithmetic, result projection or retention eviction. Those gaps are marked `UNMAPPED`. Tenant/principal/origin and Connection entity relationships await their owning models; typed scope values do not invent those entity cardinalities. F03 owns the still-missing trusted federation delegation binding.

## F02 boundary matrix

Every row is a normative observation audited against the contract. The final column distinguishes existing model checks from required future Rust binding tests.

| Input/evidence difference | Required decision | Evidence and remaining test |
|---|---|---|
| Same namespace/key/full fingerprint, durable known result still retained | Original result/classification with replay delivery metadata; no send or approval spend | §4/§5.1; run all Completed/Failed/Aborted projections through a host fixture |
| Same body/key, different caller or executor | Distinct namespace; separately admit, never disclose/suppress using the other record | Typed AuthorityScope; authored namespace-value trace includes Alice/Bob; actual isolation/admission remains unexecuted |
| Different tenant, absent realm versus `default` | Distinct namespace, preserving null rather than normalizing it | Typed optional fields and namespace-value trace; repeat tenant/realm boundaries in the host index and result gate |
| Different receiving instance or authenticated federation origin | Distinct namespace; refuse an unverified origin | Typed Origin and namespace-value trace; actual authenticated provenance is F03/host binding work |
| Same namespace/key, different operation or connection | Axis-free conflict while live, no extra dispatch | Typed RequestFingerprint; compare operation and connection coordinates in host tests, including Jira create versus comment and Jira versus Confluence |
| Same namespace/key, different canonical input | Conflict while live | Input digest is one fingerprint coordinate; vary only this coordinate in a host test |
| Descriptor/configuration/connection revision, contract/profile or canonicalization changes | Stale client refused before lookup; a refreshed different fingerprint conflicts on the still-live key | Typed fingerprint; no semantic-equivalence inference or silent namespace migration |
| Credential bytes rotate with the same bound identity and unchanged semantic revisions | Same namespace/fingerprint; admitted replay remains the original result | Credential bytes are excluded; provider/host fixture must prove revision behavior |
| Connection is reassigned or destination/cloud changes | Change metadata/config revision, then conflict on the old live key | §5.1 and Atlassian plan; a binding that cannot detect the change must refuse keyed dispatch |
| Key spelling changes, or tuple contains delimiters / empty optional values | Exact opaque key and injective tagged tuple encoding; no accidental alias | Normative `mutation-key/v1`; boundary fixtures for case, normalization, delimiters and null are still required |
| Missing key on a keyed operation | Input refusal before reservation/dispatch | §3/§5; no silent conversion to `none` |
| Two concurrent misses for the same namespace/key | One atomic reservation/Prepared attempt; loser observes winner before spending | Explicit reservation→attempt ESS reference; atomic unique index and cross-entity transaction remain host fault tests |
| B misses; A reserves and spends their shared event claim before B verifies it | B's candidate refusal rechecks the authoritative index under current admission, then observes exact A or safe conflict | §4 steps 5–6 and §5.1 miss/refusal rule; IS1 review counterexample. Repeat for preflight failure, revoked access, unreadable/stale index, and a winner after the confirmed no-winner serialization point; runtime interleaving remains a host test |
| Current access revoked before replay or while waiting | Refuse the observation without revealing original result/existence or calling it not-attempted | §4/§5.1; revoke project/space/result visibility before final admitted response decision |
| Original approval spent/expired, exact admitted replay | No fresh execution approval or redemption required | Lookup precedes new-attempt approval; test cached refusal/aborted/unknown as well as success |
| Fresh attempt after known-result expiry | New independent admission and current execution approval; original spent evidence cannot authorize it | §5.1; host fixture checks no implicit authority from the old result/key |
| Pending reservation passes 86,400 s | Still reserved; no second attempt | Compiled `idempotency-pending-no-expiry` rejects ExpireResult; elapsed clock and real send counts remain future tests |
| Indeterminate attempt passes 86,400 s | Quarantined/unknown; no expiry or implicit reconciliation | Compiled `idempotency-unknown-no-expiry` rejects expiry and RetainResult |
| Durable known terminal outcome | Replayable with deadline at original durable settlement plus 86,400 s | Compiled success/failed/aborted traces; timestamp assignment and arithmetic remain host checks |
| Replay before expiry, at equality, or after a clock discontinuity | No sliding window; expiry allowed at equality only with trustworthy elapsed-time evidence; uncertainty keeps reservation live | Compiled known-result expiry trace names boundary times; ESS `at` orders the file, it does not advance a runtime clock |
| Expiry races a new generation or old waiter | CAS old reservation id; old waiter remains bound to original attempt, never replacement | Compiled Expired terminal-state refusal; generation CAS and waiter pinning remain host checks |
| Terminal result durable but reservation settlement write lost | Recover known settlement/result from immutable attempt; no premature eviction/reuse or reset of settlement time | Typed attempt/reservation timestamps; multi-port lost-write and restart test remains required |
| Full store, unreadable ledger or missing acknowledged record after restart | Refuse unsafe new reservation/reuse; never evict Pending/Quarantined to free space | Normative fail-closed rule; bound-store and restart fault tests remain required |

## Reproduction and observed checks

Pinned tool: `.local/toolchains/ess/0.20.0/bin/ess` (`ess 0.20.0`). Run from the repository root:

```console
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/review/idempotency-ir.json
.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/review/idempotency-authored.json
.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/operations/v1alpha1/scenarios --out .local/review/idempotency-suite.json
cargo run -p connectors-build --locked --offline -- --ess .local/toolchains/ess/0.20.0/bin/ess gate
```

Initial authoring checks: four ESS files valid, 43 declarations compiled; 15 authored scenarios (nine prerequisite traces plus six F02 traces) compiled with zero refusals. Generated-only synthesis produced 52 scenarios with zero refusals. These are compiled specifications, not 67 passing runtime tests. The existing Rust gate already compiles the shared scenario directory, so no new test runner or dependency was introduced.

The full gate exited **0** on 2026-09-08 using this exact command:

```console
env TMPDIR=/home/timo/beyond10x/connectors_v2/.local/tmp CARGO_BUILD_JOBS=2 cargo run -p connectors-build --locked --offline -- --ess .local/toolchains/ess/0.20.0/bin/ess gate
```

It passed formatting, descriptor drift, workspace build/tests (**39 passed, 0 failed, 0 ignored**), Clippy, provider/library and generic-CLI boundaries, ESS validation/compilation and planning validation. Combined synthesis reported `67 scenario(s) (15 authored), 0 refusal(s)`. Transcript: `.local/review/idempotency-gate-local-tmp.log`. The earlier plain `cargo run` exited 101 before the gate started because the C compiler exceeded `/tmp`'s quota; setting the compiler scratch directory to the repository's `.local/tmp` resolved it without deleting unrelated files or changing source. The optional MSRV pass was not run.

Negative control: a temporary copy of `idempotency-pending-no-expiry.yaml` replaced `receiver_instance` with undeclared `caller_selected_receiver` in the input and view assertion. ESS refused both missing-field references with `ESS-AUTHOR-014` and both unknown-field references with `ESS-AUTHOR-013`, exit **1**. Transcript: `.local/review/idempotency-negative.log`. This proves schema-reference refusal, not trusted namespace derivation. Local link checks passed; authored known-result settlement timestamps match their terminal acts.

Two independent final reviews approved: [semantics](../../../.engineering/planning/review-result/idempotency-semantics-r2-20260908.md) and [model](../../../.engineering/planning/review-result/idempotency-model-r2-20260908.md). The [first semantic review](../../../.engineering/planning/review-result/idempotency-semantics-r1-20260908.md) found IS1: a cache miss could later refuse a shared event claim spent by the winning request without observing its replay. §4 steps 5–6 and §5.1 now require the authoritative admitted winner recheck and define its decision point; the matrix records the interleaving and failure variants. Both final reviewers accepted the correction. Only prose changed after the full-gate snapshot; the ESS model and compiled scenarios are unchanged. AEP records the review outcome and final planning validation separately; known diagnostics about prose/empty findings records remain visible.

No Connectors mutation target for `ess verify conform run` exists in the pinned CLI; future Rust bindings must execute the physical and authorization checks in the matrix before runtime conformance can be claimed. The concurrently authored versioning documents are outside this F02 review and change set.
