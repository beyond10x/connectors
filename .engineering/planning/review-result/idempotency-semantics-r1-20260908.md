---
format: aep.planning-md/1
id: review-result:idempotency-semantics-r1-20260908
kind: review-result
status: active
title: Idempotency independent semantics review, round 1
relations:
- reviews: story:contracts-idempotency-scope
revision: 1
---
needs-revision

**IS1 · P2 — A concurrent cache miss can reject a replay because its claim was spent by the winner.** Location: [semantics.md:101](/home/timo/beyond10x/connectors_v2/contracts/operations/v1alpha1/semantics.md:101), with the winner recheck currently only at line 103.

Counterexample: A and B have identical admitted namespace/key/fingerprint and the same valid event claim. B observes no reservation at step 4. A then reserves the key, spends the claim and completes. B resumes step 5, sees the spent claim and returns `approval_refused`; it never reaches step 7’s winner recheck. Before A reserved, the claim was unspent; afterwards, B should observe A without requiring another claim. The refusal conflicts with §5.1’s admitted replay rule.

Before returning a new-attempt approval/preflight refusal following a miss, recheck the authoritative reservation under current disclosure admission. Observe an exact winner or return a safe conflict; return the candidate’s refusal only when no winner governs that decision. Add this interleaving to the normative verification matrix; runtime execution remains future binding work.

Reviewed the F02 contract changes, six scenarios, ESS additions, Atlassian changes, owning story, original F02 and design §7. No mutations, integrations, full-gate rerun or other reviewer feedback. F03, wire/versioning work and unrelated concurrent files were excluded.

```findings
- file: contracts/operations/v1alpha1/semantics.md
  line: 101
  category: behavior
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "IS1/P2: After B misses the key at step 4, A can reserve the identical namespace/key/fingerprint, spend their shared event claim and complete before B verifies that claim. B then returns approval_refused without reaching step 7's winner recheck, although an exact admitted replay requires no claim redemption. Recheck the authoritative reservation under current disclosure admission before returning a new-attempt approval/preflight refusal after a miss; observe an exact winner or return a safe conflict. Add this interleaving to the normative verification matrix."
```
