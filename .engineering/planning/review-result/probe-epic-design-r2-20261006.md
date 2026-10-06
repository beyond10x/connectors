---
format: aep.planning-md/3
id: review-result:probe-epic-design-r2-20261006
kind: review-result
status: active
title: Design critic, round 2, epic:connector-probe-20261006
relations:
- reviews: story:dependency-refresh-20261006
- reviews: story:registry-clock-floor-growth
- reviews: story:owner-memory-bounded
- reviews: story:cli-json-answers-as-json
- reviews: story:upgrade-adapter-identity-mismatch-names-new-connection
- reviews: story:forge-issue-create
- reviews: story:expired-evidence-invoke-advises-revalidate
- reviews: story:catalog-honours-retry-after
- reviews: story:service-failure-carries-upstream-reason
- reviews: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
approve

I read all 10 stories, the epic, the round-1 record and `upstream-blocker:entity-runtime-open-verifies-whole-store`. Nothing I found would change the drafted shape.

- **Cycles:** none. The `depends_on` edges are catalog-honours-retry-after to service-failure-carries-upstream-reason, expired-evidence-invoke-advises-revalidate to service-failure-carries-upstream-reason and dependency-refresh-20261006, and cli-json-answers-as-json and registry-clock-floor-growth to dependency-refresh-20261006. All of them point at two stories that have no `depends_on` edges of their own, so there is no cycle. The set is two fan-ins, not a queue. forge-issue-create, owner-memory-bounded, upgrade-adapter-identity-mismatch-names-new-connection and metadata-invoke-cost-flat-in-store-size have none, and the longest chain is two items.
- **Split abstraction:** none. The nearest case is service-failure-carries-upstream-reason against catalog-honours-retry-after, which both edit the status-to-`ErrorCode` match in `read_body`. They are two behaviours (carry the reason, retry on 429), and the `depends_on` edge records the shared file. A shared file under an ordering edge is a trade-off, not a chain. The alternative is splitting `read_body` so the two stop colliding.
- **Hidden dependency:** none found. Round-1 gaps are closed. The retry-after story's ordering sentence now names the story it waits on and has its edge. The metadata story is blocked on an upstream-blocker that cites #55, and the cleared blocker covers #51.
- **Horizontal slice:** none. Each story is cut by defect, with its own acceptance.

**What I read:** 12 artifacts, plus the superseded dependency story and the upstream-blocker. I ran `aep plan artifact show` on each, then `relations`, `graph` and `validate`. I walked about 30 edges, including edges to artifacts outside the set (the blockers, `informed_by` sources, the superseded story and the vision). `validate` printed `valid`, with only stale review-result notices unrelated to this set.

**What I could not establish:**
- Whether story:owner-memory-bounded can meet its acceptance (peak RSS at 1,200 events at most twice that at 600) without the Entity Runtime change in #55. Its own scope says the fix site depends on a heap profile that does not exist yet. If the profile points at the whole-store verification inside `er.start`, that story would need a `depends_on` edge to the #55 blocker. This is a hypothesis, so it is not a finding.

**Out of my lane, not setting the verdict:**
- **Parallel safety:** story:upgrade-adapter-identity-mismatch-names-new-connection and story:cli-json-answers-as-json share `apps/connectors/src/local.rs`, `apps/connectors-cli-contract` and `ess/domains/cli.yaml` with the two edge-ordered CLI stories, and have no ordering edge; story:service-failure-carries-upstream-reason edits `ess/domains/cli.yaml` and the generated CLI contract that story:dependency-refresh-20261006 regenerates, and has no edge to it; story:owner-memory-bounded and story:metadata-invoke-cost-flat-in-store-size list `Cargo.toml` and `Cargo.lock` with no edge to story:dependency-refresh-20261006; story:owner-memory-bounded, story:registry-clock-floor-growth and story:metadata-invoke-cost-flat-in-store-size share `metadata/er.rs` and `store_cost_tests.rs`.
- **Scope or wording:** story:catalog-honours-retry-after carries a duplicated `## Ordering` heading; story:metadata-invoke-cost-flat-in-store-size is `active` while its upstream-blocker is `open`.

```findings
[]
```
