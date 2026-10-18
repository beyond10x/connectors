---
format: aep.planning-md/3
id: review-result:invoke-definition-reuse-adversary-round-2
kind: review-result
status: active
title: Adversarial review, round 2, on reusing a stored credential
summary: 'approve: no defect found in the round-1 fixes'
relations:
- reviews: story:invoke-definition-reuse-credential
prose_only: the adversary found nothing in round 2; there are no findings to list
revision: 1
---
# Adversarial review, round 2, on reusing a stored credential

Adversarial review, round 2, of the fixes for story:invoke-definition-reuse-credential at
9cc91a24f (4a9d3cb01c..9cc91a24f), run 2026-10-18. Verdict: nothing found; no case added.

Runs: the two reuse journeys and the round-1 race journey all pass (the race case is green); host
`reuse` and `definitions` cases 19 passed.

Attacked and held: the guard covers every write to a connection row that could matter (reconnect
and publish move fence, generation and material; revalidation moves the fence; revoke sets the
state and a new fence; a Stale mark sets the kept flag) and runs inside the publishing commit; a
guard refusal after the custody write fails the claim and retires the written candidate;
`dispatch_reuse` cancels idempotently; `receives_reuse` and its origin parser fail closed on port,
case, userinfo and extra keys; the listing's byte budget always lists at least one candidate and
keeps the question under 4096 bytes; the listing fallback only asks the plain question.
