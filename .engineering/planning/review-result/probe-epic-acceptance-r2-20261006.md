---
format: aep.planning-md/3
id: review-result:probe-epic-acceptance-r2-20261006
kind: review-result
status: active
title: Acceptance critic, round 2, epic:connector-probe-20261006
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
needs-revision

- story:metadata-invoke-cost-flat-in-store-size — the third acceptance item, "a workload of 700 consecutive read invokes against one store completes with no admission `timeout` and no `outcome_unknown`", names no command, test or harness that runs it. Every other item names one, and the first-round bound on the second item is now fixed — .engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md:55
- story:metadata-invoke-cost-flat-in-store-size — the body carries two `## Acceptance` headings and the first is empty, so a reader or verb that takes the first section finds no acceptance — .engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md:44
- story:owner-memory-bounded — "Peak RSS at 1,200 events is at most twice the peak at 600 events" is stated with no baseline at 1,200 events. Observed gives only 601 events (1,991 MB and 1,012 MB), so the item may already hold on the base commit and the work would not change its answer — .engineering/planning/story/owner-memory-bounded.md:38
- story:owner-memory-bounded — the body carries two `## Acceptance` headings and the first is empty — .engineering/planning/story/owner-memory-bounded.md:30
- story:cli-json-answers-as-json — the body carries two `## Acceptance` headings and the first is empty — .engineering/planning/story/cli-json-answers-as-json.md:45
- story:expired-evidence-invoke-advises-revalidate — the body carries two `## Acceptance` headings and the first is empty — .engineering/planning/story/expired-evidence-invoke-advises-revalidate.md:41
- story:registry-clock-floor-growth — the body carries two `## Acceptance` headings and the first is empty — .engineering/planning/story/registry-clock-floor-growth.md:46

What I read: all 10 stories with a `decomposes` edge to the epic, in full with `aep plan artifact show` and by grepping each file for its Acceptance section. I also read the r1 record and ran `aep plan artifact body --help` to see how `--section` behaves.
- Round-1 findings that landed:
  - cli-json no longer requires the demo or MCP.
  - expired-evidence is now one decided outcome.
  - metadata-invoke has a 2x bound at 600 and 6,000 events, and its stale item is replaced.
  - owner-memory drops the "or file upstream" branch.
  - registry-clock-floor uses one unit, a count of `LocalClockFloor` events against a baseline.
- Unchanged and acceptable: catalog-honours-retry-after, dependency-refresh-20261006, forge-issue-create, service-failure-carries-upstream-reason, upgrade-adapter-identity-mismatch-names-new-connection.

Could not establish:
- Whether any consumer, such as a gate or evidence check, reads the first `## Acceptance` section mechanically. `aep plan validate` doesn't exist, so I ran no validator. I rated the duplicate headings a warning on that basis.
- Whether `read_invoke_cost_by_store_size` can build a 6,000-event store on 0.26.0. The Source section records that the base could not, and I did not run it.
- Out of my lane, not counted toward the verdict: several stories share files (parallel-safety). Whether `story:metadata-invoke-cost-flat-in-store-size`, blocked on the upstream change, belongs in this epic's closing set (scope).

```findings
[
  {"file": ".engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md", "line": 55, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the third acceptance item, a workload of 700 consecutive read invokes against one store completing with no admission timeout and no outcome_unknown, names no command, test or harness that runs it, unlike every other item"},
  {"file": ".engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md", "line": 44, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the body carries two `## Acceptance` headings and the first is empty, so a reader or verb that takes the first section finds no acceptance"},
  {"file": ".engineering/planning/story/owner-memory-bounded.md", "line": 38, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the item 'Peak RSS at 1,200 events is at most twice the peak at 600 events' records no baseline at 1,200 events (Observed gives only 601 events), so it may already hold on the base commit and the work would not change its answer"},
  {"file": ".engineering/planning/story/owner-memory-bounded.md", "line": 30, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the body carries two `## Acceptance` headings and the first is empty"},
  {"file": ".engineering/planning/story/cli-json-answers-as-json.md", "line": 45, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the body carries two `## Acceptance` headings and the first is empty"},
  {"file": ".engineering/planning/story/expired-evidence-invoke-advises-revalidate.md", "line": 41, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the body carries two `## Acceptance` headings and the first is empty"},
  {"file": ".engineering/planning/story/registry-clock-floor-growth.md", "line": 46, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the body carries two `## Acceptance` headings and the first is empty"}
]
```
