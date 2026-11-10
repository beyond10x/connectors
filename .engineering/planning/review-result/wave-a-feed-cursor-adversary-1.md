---
format: aep.planning-md/3
id: review-result:wave-a-feed-cursor-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on feed-cursor-unadvanced-is-unavailable (wave A)
relations:
- reviews: story:feed-cursor-unadvanced-is-unavailable
revision: 1
---
```
unit: story:feed-cursor-unadvanced-is-unavailable (HEAD 2d2186cfa plus uncommitted test additions)
verdict: red
cases: executed 11→13, red 1
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/units/feed-cursor-unadvanced-is-unavailable/scratch/{cases.rs,cases.log,suite.sh,suite.log,suite2.sh,suite2.log,suite3.sh,suite3.log}
needs-coordinator: yes
```

1. Diff: `adapters/catalog/tests/feed_engine.rs | 81 +++` (test file only, uncommitted).
2. Cases, run alone first (`cargo test --locked -p connectors-catalog-provider --test feed_engine -- adversary_`: 1 passed; 1 failed, exit 101):
   - `adversary_the_catalog_cursor_row_names_the_refusal_the_engine_performs`: RED. The engine refuses a first read answering `{"changes":[],"has_more":true}` as `unavailable`, but the catalog contract's `| Watermark: \`cursor\`` row (`contracts/catalog/v1alpha1/semantics.md:183`) lists the cursor position's refusals and not this one.
   - `adversary_a_complete_answer_without_a_cursor_keeps_the_one_sent`: green; pins `if next.is_some() { mark.cursor = next }` against the unconditional-assignment mutant.
3. Suite after the cases: 13 feed targets of connectors-catalog-provider, all passed except the red case (feed_engine 12 passed, 1 failed; confluence_v1_feed 14, feed_capabilities 3, feed_extensions_security_review 12, feed_utc_offsets 5, feed_wave1021_adversary 2, _u6 4, gitlab_feed 7, gitlab_u23 2, u23_pass2 3, gitlab_wave1021 1, jira_feed 14, jira_wave1021 1). `connectors-build --bin connectors-build -- feed_conformance`: 41 passed (sccache off). `connectors-docs generate --check`: current.
4. Finding: `contracts/catalog/v1alpha1/semantics.md:183` misses the new refusal. NEEDS-CHANGE, introduced. The story asks for the rule beside the cursor.last one, which lives in the catalog contract (row 180), not only in the family contract. Fix named: add to row 183 that an answer for which `complete` does not hold and whose cursor at `next` is absent, empty or the one sent is `unavailable`; regenerate.
5. Attacked, not broken: no shipped feed uses the cursor position (confluence-v1, jira, gitlab use `time`); first vs resumed read comparison; an absent `complete` pointer is incomplete (looped before, refuses now; no shipped declaration reaches it); mutants dropping each conjunct turn a case red; numeric cursors compare as strings; the conformance model provider always advances.

```findings
[{"file": "contracts/catalog/v1alpha1/semantics.md", "line": 183, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The catalog engine's Watermark: cursor row lists the cursor position's refusals but not the new unavailable for an incomplete answer that does not advance the cursor; red case adversary_the_catalog_cursor_row_names_the_refusal_the_engine_performs (feed_engine.rs:1308)."}]
```
