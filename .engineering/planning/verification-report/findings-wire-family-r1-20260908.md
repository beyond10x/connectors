---
format: aep.planning-md/1
id: verification-report:findings-wire-family-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:wire-family-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: c17683275e5a5ea9e285d786fe0a36044b857e7d2e332d4d8e5dbb5eb04eef2a
relations:
- verifies: review-result:wire-family-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:wire-family-r1-20260908

This supplements [the immutable original](../review-result/wire-family-r1-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

8 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "1. **P1 — No single complete disposition for closed additions.** Core optional descriptor/envelope/error extensions require an explicit new selected binding. Current capability and connection no-change claims, auth evidence's additive claim, and records/resource additive shorthand omit the strict-reader distinction. Sources: capability :104; connection :115; evidence :147; records :89; resource discovery :102; core :31/:61/:72/:93/:113."
  },
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "2. **P1 — Mutation response still lacks a representable full classification/replay contract.** `effect: replayed` is a delivery marker, cannot replace original applied/refused/not_attempted/unknown, and must not imply success. New public errors, cause versus classification and observation-subject identity must be preserved for duplicate/refused observation cases. Source: operations :73, :91, :129, :163. The parent owns final envelope encoding."
  },
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "3. **P1 — Descriptor profile/auth composition is inconsistent.** Catalog `profiles[]` + generic-http/mutation composition conflicts with required singular core `profile`; its `requires_auth` string conflicts with auth.profile's array. Select a normative representation and update examples/declarations. Sources: catalog :73/:97/:121; auth.profile :72; core :61."
  },
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "4. **P1 — Session/media binding version and extension refusal are missing from service-only compatibility.** Establishment payload, public session errors, duplex controls, authenticated live authority and binary frames must each have an explicit selected binding disposition. Old unary success with a session-looking result is insufficient. Sources: sessions :53/:67/:71/:105; media :45/:51."
  },
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "5. **P2 — Mediated route public/private boundary is contradictory.** Resolve parent-operation-surface wording against same-composition-only forward; choose propagation mapping for route_refused/route_unavailable without exposing a generic forward route. Sources: mediated_route :43/:55/:67."
  },
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "6. **P2 — Generic HTTP limits cannot fit unchanged v1 semantics.** 256 KiB and 30 s must not be advertised over a fixed 64 KiB/20 s binding absent explicit profile/binding limits and refusal. Sources: catalog :145/:147; service :36; operations :147."
  },
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "7. **P2 — Artifact/schema compatibility needs its own accounting.** Adapter-kind additions, catalog manifest/index formats and receiver configuration (`http.extra_headers`) are not solved by service wire v1alpha2. Strict readers need named unsupported-format/configuration refusal; do not rename adapter/v1 or /v2 to match service wire. Sources: auth.profile :109; catalog :174/:176; logs :83."
  },
  {
    "file": ".engineering/planning/review-result/wire-family-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "8. **P2 — Existing profile preservation versus proposed private hardening must be explicit.** The old configured/read_only/http-bearer paths can retain wire bytes while proposed generation validation changes admission behavior. New readers must retain the selected old semantics, or refuse an unsupported projection, rather than silently impose new validation obligations while calling it unchanged. Sources: connection :82/:115; evidence :93; custody :91; service :63."
  }
]
```

