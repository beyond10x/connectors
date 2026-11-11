---
format: aep.planning-md/3
id: story:owner-recovers-after-failed-acquisition
kind: story
status: active
title: An owner that saw an acquisition fail still admits work
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/lifecycle.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:16Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:17Z", actor: "human:operator", revision: 4, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
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

## Further observations (same consumer, 2026-09-29 around 15:10, v0.15.1, not reproduced here)

- After the connection's evidence expired, `operations invoke` answered `not_granted` with next
  action `repair_connection` while `connections list` said `pending`; nothing pointed at revalidate.
- `revalidate` answered `outcome_unknown` at stage `publication`, and `connections list` showed
  `ready` right after.

Both are the same symptom as steps 2–4: the answer to a command and the state `list` reports
disagree after a publication-stage failure.

## Wave status

Status on 2026-11-10: not reproduced. The attempts are recorded below, and the reproduction is kept
as a regression guard (merge d3bd6fcd4, commit 55773eedf).

Setup: a jail with `mounted_secret` custody, the real CLI, an owner and an adapter child. Debug-only
hooks force a publication failure, and the acquisition window is shortened from 4 minutes. Each
attempt ran `connections list` and `operations invoke` on the same owner process.

1. Publication fails without committing. The first `connect` gives `outcome_unknown` at
   `publication`. The acquisition stays in flight until the shortened window expires. A retried
   `connect` gives `ready` on the same owner. `invoke` succeeds and `list` says `ready`, with no
   restart.
2. Publication commits but the reply is lost. The first `connect` gives `outcome_unknown` at
   `publication`. `list` says `ready`, and the connection really is published. `invoke`, then
   `revalidate`, then a second `invoke` all succeed.

`apps/connectors/tests/owner_recovers_after_failed_acquisition.rs` passes 3 of 3. It asserts the
second and third acceptance lines:
- after the failure, a later `connect` and an `invoke` on the same owner succeed;
- `list` never says `ready` while that owner refuses every invoke.

Not exercised:
- the platform Secret Service custody path;
- the report's step 1, the owner stopped after a configuration revision change.
