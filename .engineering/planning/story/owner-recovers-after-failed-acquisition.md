---
format: aep.planning-md/3
id: story:owner-recovers-after-failed-acquisition
kind: story
status: draft
title: An owner that saw an acquisition fail still admits work
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Report (knowledge-ingest consumer, 2026-09-29, v0.15.1, not reproduced here)

1. The consumer stopped the running owner because the configuration revision had changed.
2. First `connect` of a new GitLab instance: `outcome_unknown` at stage `publication`; the
   acquisition stayed `pending` for about 4 min, then `failed`.
3. A retried `connect` succeeded (`ready`).
4. The owner started by the failed attempt then answered every `operations invoke` with `timeout`
   at stage `admission`, and `revalidate` with `outcome_unknown` at `publication`, while
   `connections list` said `ready`.
5. Stopping that one owner process cleared it: the next invoke spawned a fresh owner that worked.

Hypothesis (unverified): an owner whose acquisition fails at publication keeps state that blocks
later admission. Nothing here yet says which state, or whether step 1 matters.

## Acceptance

- A test reproduces steps 2–4 against a fixture provider that fails publication once, and is red on
  the current code, or the report is closed as not reproduced with the attempts recorded.
- After a failed acquisition, a later `connect` success and `operations invoke` on the same owner
  succeed without stopping the owner.
- `connections list` does not report `ready` while that owner refuses every invoke at admission.
