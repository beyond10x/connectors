# F03 delegated approval verification — 2026-09-08

This checkpoint selects the proposed one-hop approval subject, preparation read, trusted delivery and issuer proof framing, timing, nonce and sole-leaf redemption rules. It closes the semantic scope of `story:contracts-federated-approval`; receiver/issuer/policy/persistence integration remains an explicit advertisement gate. No runtime or adapter implementation changes are included.

The source baseline was `b5ace3f29d6deb1dbc9e5ddd50a8a8b02152cb2f`. The [normative protocol](../../../contracts/service/delegation.md) pins current Atlas and predecessor Connectors/Platform evidence separately. Pinned ESS is **0.20.0**, not the older PATH installation.

| Verification | Observed result and limit |
|---|---|
| Full repository gate | Exit 0; 50 Rust tests pass, formatting/lint/boundary checks pass, Rust 1.88 workspace/all-target check passes. These protect existing implementation; they do not implement or execute this proposed protocol. |
| ESS validate/compile | 10 files valid; 153 declarations. Subject/preparation/proof/context values and immutable nonce/redemption receipts are typed, with known ServiceConfiguration/AttemptRecord references. |
| ESS schema generation | 163 artifacts in each of two independent output directories, all bytes identical; 21 relevant generated copies retained with [hash manifest](projection-manifest.json). No generated schema was hand-edited. |
| Existing authored operations | 15 files/scenarios, zero refusals; all six idempotency scenario files now show explicit direct-origin route:null where applicable; the namespace fixture's federated candidate has a matching non-null gateway route. |
| Combined gate conformance synthesis | 222 scenarios, including 34 authored operations/auth-acquisition/auth-evidence scenarios, zero refusals. This compiles obligations; it is not sequential runtime execution. |
| Explicit sessions author/synthesis | 13 authored scenarios, zero refusals; 201 synthesized scenarios including those 13, zero refusals. The gate does not collect this directory, so it was checked separately. |
| [Cryptographic vectors](cryptographic-vectors.json) and [results](cryptographic-results.json) | 20/20 expected Ed25519 signature/raw-body digest observations match using public deterministic test keys. Several malformed claims intentionally have valid signatures and must still be refused by a future receiver. No live authority was issued or used. |
| [Type values](subject-and-type-vectors.json) and [results](type-projection-results.json) | 12/12 expected generated-schema results match: nine positive semantic values and three structural negatives. All nine required-null wire values fail the generic ESS optional projection, which uses omission. This is a documented projection limit, not a claimed wire codec. |
| [Subject equality](subject-equality-audit.json) | 15 canonical-coordinate mutations differ; five base subject examples cover configured/managed, direct/federated and executor presence. Coordinate perturbations are byte-equality checks, not necessarily independently admissible alternate contexts. |
| [Time predicates](time-admission-audit.json) | 23/23 arithmetic/truth-table expectations match, including exact 30/300-second profiles, malformed encodings, expiry equality and declared post-ack current-fact failures. No real clock, nonce suspension, policy revocation or storage operation was executed. |
| [Textual scenario audit](scenario-audit.md) | 26 reviewed expected traces cover preparation, routing/authority changes, proof separation, nonce/spend/replay ownership, ambiguity, audit, bounded execution and cutover. |

The exact one-off fixture/arithmetic inspection source is preserved as a [transcript](fixture-harness.md), not installed as executable project tooling. ESS output is generated only through ESS. Compiler validation does not enforce signatures, issuer configuration, bounded clock observations, atomic uniqueness, Prepared-only spending, live revocation, fencing, durability or mandatory-null wire encoding; those remain named UNMAPPED binding obligations in the model.

Full gate command (exit 0):

```sh
env TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked --offline -p connectors-build -- --ess "$PWD/.local/toolchains/ess/0.20.0/bin/ess" gate --msrv
```

Additional commands (all exit 0):

```sh
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --format json
.local/toolchains/ess/0.20.0/bin/ess generate --path ess --kind schema --out .local/federated-approval-20260908/schema-first
.local/toolchains/ess/0.20.0/bin/ess generate --path ess --kind schema --out .local/federated-approval-20260908/schema-second
.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/federated-approval-20260908/operations-authored.json
.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/federated-approval-20260908/sessions-authored.json
.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/federated-approval-20260908/sessions-suite.json
```

Independent review records and final source hashes accompany the planning checkpoint. First reviews identified eight foundation requirements each; rechecks required receiver-enforced exact time windows and a final validity/key/projection/deadline check after nonce acknowledgement. Both changes are explicit in the protocol and evidence above. Final reviewer verdicts are recorded separately, without rewriting the original reports.

## Recorded finding dispositions

Each listed finding is fixed in the normative protocol/model or fixture; AEP records one review_outcome per ID against its immutable review. These are reviewer finding IDs, separate from the source-ledger F03 being closed.

| Finding | Resolution |
|---|---|
| <a id="F03-A01"></a>F03-A01 | §2 complete canonical executing-leaf subject. |
| <a id="F03-A02"></a>F03-A02 | §3 admitted preparation for configured and managed selection. |
| <a id="F03-A03"></a>F03-A03 | §2 separates routing/presentation freshness and signed identity. |
| <a id="F03-A04"></a>F03-A04 | §§1–2 trusted qualified caller/optional executor and origin. |
| <a id="F03-A05"></a>F03-A05 | §§5–6 sole executing-leaf approval redemption. |
| <a id="F03-A06"></a>F03-A06 | §§4–6 transport nonce is separate from keyed business replay. |
| <a id="F03-A07"></a>F03-A07 | §7 preserves original outcome/identity and conservative gateway loss. |
| <a id="F03-A08"></a>F03-A08 | §8 value/receipt model and explicit known references, unresolved owners UNMAPPED. |
| <a id="FB-01"></a>FB-01 | §§2–3 canonical subject and trusted preparation visibility. |
| <a id="FB-02"></a>FB-02 | §2 presentation versus signed target/route changes. |
| <a id="FB-03"></a>FB-03 | §§1–2 receiver-owned scoped issuer trust and identity mapping. |
| <a id="FB-04"></a>FB-04 | §4 exact compact proofs, raw request binding and private GET correlation. |
| <a id="FB-05"></a>FB-05 | §§5–6 separate nonce receipt, approval spend and dispatch fence. |
| <a id="FB-06"></a>FB-06 | §5 exact clock intervals, stable key-independent uniqueness and safe retention. |
| <a id="FB-07"></a>FB-07 | §6 preserves the leaf attempt/gate ordering and ambiguous-ack refusal. |
| <a id="FB-08"></a>FB-08 | §7 precise failure correlation, outcome and disclosure behavior. |
| <a id="F03-A-R2-01"></a>F03-A-R2-01 | §5 receivers enforce exact time profiles; issuer explicitly sets iat=t. |
| <a id="FBR-01"></a>FBR-01 | §§4–5 bounded integer encoding and checked receiver lifetime predicates. |
| <a id="FBR-02"></a>FBR-02 | §4.1 final current validity/key/projection/deadline check after nonce acknowledgement. |
| <a id="F03-A-R3-01"></a>F03-A-R3-01 | Federated namespace scenario now has a matching non-null gateway route; direct cases retain route:null. |
