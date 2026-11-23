---
format: aep.planning-md/3
id: review-result:er-0330-pin-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: needs-revision

1. The story and `story:read-cost-test-reports-bytes-per-read` both claim the "bytes appended per invoke" column.
   - Source: issue #267 wanted item 4, "Extend `read_invoke_cost_by_store_size` with a bytes-appended-per-invoke column and a bound on it."
   - Gap: this story's acceptance 4 (story:102) adds `blob_bytes_per_invoke` to the same test. The sibling's first acceptance bullet (story/read-cost-test-reports-bytes-per-read.md:27) prints the same number. Both will be marked done, and both edit `store_cost_tests.rs`.
   - Change: one story owns the print. Make this story's acceptance 4 a before/after taken with that test's output, add `depends_on` on the sibling, and drop its own edit to the test. Alternatively, the sibling drops its print bullet.

2. The bound in item 4 has no owner after the pin.
   - Source: "and a bound on it". The sibling writes "The bound is the number measured on the commit that adds it" (read-cost story:29), and this story leaves it there (story:105).
   - Gap: if the sibling lands first, the bound sits at about 107 KB per read. The pin then drops the number to about a third of that, and a revert to embedded definitions passes the test. Neither story orders the two or lowers the bound.
   - Change: this story names the order, either "sibling lands after, bound taken at the pin" or "this story lowers the bound to the post-pin number".

3. Item 2's "about 10×" is narrowed to "a third" without saying so.
   - Source: item 2, "it would shrink the store about 10×". The outcome (story:36) says at most a third. `story:settled-read-uses-leave-the-store` says "an estimated fifth". Entity Runtime measured 4.8× (21,075 to 4,373 bytes).
   - Arithmetic from comment 7091114772: 0.6 MB of non-definition blobs plus 673 × 4.4 KB is about 3.5 MB of 11.0 MB, about 32 %. The "third" bound is therefore marginal.
   - Change: Outcome and Source say that item 2 promised about 10× and the pin delivers about 5×, and that the result is posted on #267. The third and fifth figures are reconciled.

4. The consumer's gain is not reachable from this story, and the story does not say so.
   - Source: "Consumer: dev-operator, whose Jira store held 650 MB" (story:43). Issue #267 says "the only remedy operators have is moving the state directory aside".
   - Gap: old records are not rewritten, so the 650 MB store keeps its bytes and its replay cost. Only slower growth is delivered. The shrink comes from `story:settled-read-uses-leave-the-store`, which compacts a store written by the previous release. The 35 s `connections list` comes from `story:connections-list-answers-through-the-owner`. Out of scope still says "a state directory is rotated instead" (story:141), which is the remedy the issue rejects.
   - Change: Source names dev-operator's gain as arriving through B and item 3, not through this pin. The CHANGELOG sentence in acceptance 5 or 8 says that records already written keep their size until compaction ships.

5. Acceptance 6 is not traceable to item 2 or pick A.
   - Source: item 2 and pick A say nothing about RSS. `story:settled-read-uses-leave-the-store` already owns "The owner's RSS after 1,000 reads is reported before and after".
   - Change: drop acceptance 6 (story:109), or mark it as a report with no claim.

What I read: 5 artifacts (`aep plan artifact show` on the story, the epic and the three siblings), `aep plan artifact graph`, issue #267 with its comment, the 0.33.0 release notes, and `Cargo.toml` and the 0.26.0 fixture README in the tree.

Not established:
- Whether tag `v0.74.2` exists, and why the outcome says "0.75.0" while the fixture says `v0.74.2`.
- Whether the "third" bound holds. That belongs to the acceptance critic.
- The "Decided after acceptance 5" clause (raising `LEVEL` to 10) is conditional and named, so it is not a finding. It would widen the story if it fires.
- `epic:review-20261123-findings` never mentions #267, yet 19 stories already decompose it, so I did not count this as reach.

```findings
- {file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md, line: 102, category: scope, severity: blocker, verdict: needs-revision, origin: introduced, message: "acceptance 4 adds blob_bytes_per_invoke to read_invoke_cost_by_store_size, the same column story:read-cost-test-reports-bytes-per-read acceptance 1 claims (issue 267 item 4); one story owns the print and the other depends on it"}
- {file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md, line: 105, category: scope, severity: warning, verdict: needs-revision, origin: introduced, message: "the bound of item 4 is set at the commit that adds it; if the sibling lands before the pin it sits at about 107 KB and does not catch a revert of item 2, and no story orders the two or lowers the bound"}
- {file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md, line: 36, category: scope, severity: warning, verdict: needs-revision, origin: introduced, message: "item 2 promised about 10x; the outcome says a third and settled-read-uses-leave-the-store says a fifth, with 4.8x measured by Entity Runtime; the narrowing is not stated and the two figures disagree"}
- {file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md, line: 43, category: scope, severity: blocker, verdict: needs-revision, origin: introduced, message: "the consumer 650 MB Jira store gains nothing from this pin because old records are not rewritten; the story names the consumer without saying its gain comes from compaction and item 3, and the changelog sentence does not tell operators that old bytes stay"}
- {file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md, line: 109, category: scope, severity: warning, verdict: needs-revision, origin: introduced, message: "acceptance 6 reports peak_rss_mb, which neither item 2 nor pick A asks for and which settled-read-uses-leave-the-store already owns"}
```
