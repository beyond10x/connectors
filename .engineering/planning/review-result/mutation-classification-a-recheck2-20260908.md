---
format: aep.planning-md/1
id: review-result:mutation-classification-a-recheck2-20260908
kind: review-result
status: active
title: Mutation classification reviewer A final recheck
relations:
- reviews: story:contracts-mutation-classification
revision: 1
---
# Independent classification recheck A — final terminal-error refinement

Verdict: **approve story:contracts-mutation-classification only**. **Zero residual/new findings.** Approval does not extend to restart/idempotency or visibility.

The same 51 inputs reviewed in classification-recheck1 were frozen again before analysis. `source-hashes.json` contains their exact current bytes/hashes; `comparison.json` records the comparison to the previously approved snapshot. Exactly two inputs changed, with one semantic table-row refinement in each; the other **49 inputs are byte-identical**.

The SIP §4.1 post-ready/pre-encoding terminal case now distinguishes `revoked`, `lease_expired`, `session_not_ready` for confirmed close/hangup, and `session_lost` for lost continuity. Classification-verification MC18 agrees. These are already selected closed service/session error codes. This avoids conflating a known terminal state with lost continuity, while preserving known `applied` effect knowledge and withholding unusable handles or revoked one-use authority.

The surrounding session first-terminal rule, serialized ready decision, historical ready receipt, current result authority, safe delivery, no redial and existing cutoff/teardown requirements are unchanged. A later terminal does not rewrite the dial's business effect or imply a new dispatch opportunity. The change introduces no new public field, schema, state, provider proof or runtime claim.

Exact changed-input hashes:

| Source | SHA-256 |
|---|---|
| docs/adapters/media-session.md | `61892e6ae282ccb3a620f41577f0f6a08b7debec512d0d89ad34d93faa61855a` |
| docs/evidence/mutation-profiles-20260908/classification-verification.md | `2528da3a97d9be88ef56ab595c149fd26d87f132bbb4d4247bca323a042e49a5` |

The previous independent evidence checks remain applicable: all shape results, selected schemas, compiled archives, projection manifest and gate evidence among the frozen inputs are unchanged. No runtime suite or provider call was rerun; no tracked source or planning was edited, and no peer output was inspected. Existing MP-A-03–06 stay with restart-idempotency and MP-A-07 with mutation-visibility. Concrete SIP commitment/refusal proof and runtime conformance remain future binding prerequisites as the approved packet states.
