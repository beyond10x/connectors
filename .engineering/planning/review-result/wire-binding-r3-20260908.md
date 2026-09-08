---
format: aep.planning-md/1
id: review-result:wire-binding-r3-20260908
kind: review-result
status: active
title: Final independent service binding approval
relations:
- reviews: story:contracts-wire-compatibility
revision: 1
---
# Independent final E02 review B

Verdict: **approve the E02 specification changes**. Residual findings: **0**. This approves the local semantic/version disposition, not implementation of a v1alpha2 codec, audit store or delegated admission.

## Reviewed source

The final reviewed 31-file set is recorded in `source-hashes-final-reviewed.json`. Its immutable bytes are retained in `final-snapshot/`, `final-snapshot-amendment/` and `final-snapshot-evidence-amendment/`. The initial `source-hashes-final.json` and both amendment manifests remain intact. I read the final corrections, confirmed every consolidated hash against current files, and verified no runtime source paths under crates/adapters/apps have a diff. I changed no source or planning files, delegated no work and read no other reviewer's report. Root's creation of immutable final planning records and final planning validation follows this review; those administrative actions are not claimed already complete.

## Finding closure

| Finding | Final disposition |
|---|---|
| WB-01 version selection | Explicit /v1alpha2 routes select the codec before GET. Extended describe success uses a common Response, null request_id and complete nested Descriptor with matching versions; failure uses the same framing and exact HTTP mapping. Legacy GET remains bare Descriptor. |
| WB-02 strict readers | Shape-compatible version refusal, unknown-field decoding refusal, bare early errors and correlation/HTTP fallback are distinguished. There is no universal invalid_input-to-unsupported remapping or automatic retry. |
| WB-03 mutation representation | Five required observation fields preserve original effect, original identity and separate replay/secondary cause. Success requires applied; unknown requires outcome_unknown. Applied-but-undeliverable result is error/applied. Gateway and source audit facts have separate typed slots. |
| WB-04 projection | Projection defaults off, requires an explicitly verified unchanged static/read subset, and has its own revision and invoke allowlist. Hidden operation names or full/new revisions cannot bypass it. Gateway support is the actual per-hop intersection. |
| WB-05 delegation | The incomplete HMAC header is withdrawn. F03 remains unbound/unadvertisable pending full receiver/request/subject/replay/correlation rules. This is an explicit E02 disposition, not closure of F03. |
| WB-06 audit | Required nullable references/status distinguish acknowledged complete/incomplete from unavailable/not_required. Admission and final observations are separate. Pre-dispatch audit failure refuses; final failure preserves effect knowledge. Early untrusted coordinates remain absent. |
| WB-07 contexts | Static bearer maps to a stable configured principal. Authentication precedes decoding; trusted policy/grant facts establish a supplied executor assertion before admitted context construction. Missing governed policy cannot fall back to local authority. |
| WB-08 version authority | One proposed local binding owner supersedes conflicting defaults while retaining independent family, adapter kind, artifact and CLI versions. Multi-codec clients are allowed with explicit selection and no fallback; rollout is not implied. |

WRB-01 is closed by explicit GET framing and the 30-code HTTP mapping. WRB-02 is closed by realization:generic selecting identical 256 KiB/4 MiB/40 s/30 s/5 s ceilings for reads, mutations and pages across the owners. WRB-03's empty auth alternative now carries scopes:[]. WRB-04's CLI audit row and stack executor/delegation guidance agree with the owner. The final S4 scenario-number residue now names policy, executor/realm, audit-failure and correlation requirements, marking old numbering historical.

The final series clarification places minimum step in input_schema and maximum-window arithmetic in the selected profile; neither extends the five-field Operation.limits object. I read both that sentence and its corresponding matrix entry.

## Evidence audit

- All **27 proposed response vectors** were independently read against the final semantic rules: **20 positive and 7 negative** expectations agree. They cover describe, original/replayed effect classifications, denied disclosure, unknown lookup/waiter, audit and terminal-store failures, undeliverable applied results and source audit provenance.
- `final-vector-audit.json` records an additional offline consistency audit of closed fields, required-null emission, original identity co-presence, audit status/reference consistency, effect/status/code combinations and HTTP mapping. This is **not execution of a v1alpha2 codec** and does not prove authority, ledger truth or provider effects.
- All **68 legacy vector IDs and expected decoding decisions** match the recorded observations. The retained temporary Rust harness calls the current concrete connectors-core::read_json types and asserts only decode acceptance. Later server/client behavior is separately supported by inspected source and the existing wrong-version zero-dispatch test; no new negotiation execution is claimed.
- Gate log results independently sum to **50 Rust tests passed, zero failed/ignored**. The log contains successful MSRV 1.88 checking, **9 valid ESS files**, **135 declarations**, **222 compiled obligations (34 authored)**, zero synthesis refusals and the final all-checks-passed marker. Separate session artifacts contain **13 authored / 201 synthesized** scenarios. Overlap and lack of sequential execution are accurately disclosed.
- ESS service-wire types remain values, reusing the existing effect and attempt identity types without inventing persistence cardinalities or behavioral commands. Required-null, bounds, cross-field, privacy and codec/execution obligations remain explicitly UNMAPPED.

The family matrix assigns public additions to extended envelope fields, selected payload schemas, private ports, independent artifact/configuration readers or explicitly unbound transports. F03, authority/audit persistence, protected completion, session/media bindings and other family behavior retain their named owners and advertisement gates. This approval closes no sibling story and does not complete the broader all-specifications stabilization goal.
