---
format: aep.planning-md/3
id: review-result:probe-epic-acceptance-r1-20261006
kind: review-result
status: active
title: Acceptance critic, round 1, epic:connector-probe-20261006
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

- story:cli-json-answers-as-json — the third acceptance item requires "the examples triage demo, the MCP adapter" to read the new form, but the story's own Scope says the demo is outside this repository and finds no MCP code, so nothing in this tree can show it done — .engineering/planning/story/cli-json-answers-as-json.md:52
- story:expired-evidence-invoke-advises-revalidate — the acceptance offers two different outcomes joined by "Or:" (a revalidate advice versus transparent revalidation "if the contract allows it") and leaves the choice to the implementor, so a check can't know which behaviour to expect — .engineering/planning/story/expired-evidence-invoke-advises-revalidate.md:52
- story:metadata-invoke-cost-flat-in-store-size — "stays within a stated bound" names no bound, so any measured time passes; the same sentence also joins a second independent outcome (the ~700-invoke workload completing with no admission `timeout`) — .engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md:48
- story:metadata-invoke-cost-flat-in-store-size — the first acceptance item requires an Entity Runtime that exposes "subject-scoped reads" flat in unrelated events, but the body records 0.26.0 already adopted, `execute_batch` flat, the scoped `read_history` measured slower and not kept, and `er.start` still growing, so the item no longer describes what is left to verify — .engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md:46
- story:owner-memory-bounded — the last acceptance item passes either on "at most twice that at 600" or on filing an upstream change, so the story closes with no change in peak RSS at 1,200 events — .engineering/planning/story/owner-memory-bounded.md:36
- story:registry-clock-floor-growth — the second acceptance item mixes two units: "the share of clock floor events falls by at least half" (the Observed share is 59%) against "2026-10-06's 7 per invoke" (a count), and the harness reports total `appended_per_invoke`, not clock-floor events, so two readers compute different thresholds — .engineering/planning/story/registry-clock-floor-growth.md:51

Read 10 of 10 ids (all the stories with a `decomposes` edge to epic:connector-probe-20261006), in full with `aep plan artifact show`. I listed the set with `aep plan artifact list --format json | jq` and cross-checked line numbers against the store files. Approvable as drafted: catalog-honours-retry-after, dependency-refresh-20261006, forge-issue-create, service-failure-carries-upstream-reason, upgrade-adapter-identity-mismatch-names-new-connection.

Could not establish:
- I didn't run `aep plan artifact lifecycle` or `kinds` for story-specific terminal statuses.
- I didn't check that the symbols and files the acceptances name exist in the tree, apart from `appended_per_invoke` in `store_cost_tests.rs:229-235`.
- Out of my lane, not counted toward the verdict:
  - `story:cli-json-answers-as-json` carries an unproven safety fact that changing `operation()` also changes `adapters describe` and the legacy describe, and the acceptance doesn't mention it (design).
  - Several stories collide on `adapters/catalog/src/lib.rs`, `local/operations.rs`, `er.rs` and `store_cost_tests.rs` (parallel-safety).

```findings
[
  {
    "file": ".engineering/planning/story/cli-json-answers-as-json.md",
    "line": 52,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "undecided",
    "message": "the third acceptance item requires the examples triage demo and the MCP adapter to read the new form, but the story's Scope says the demo is outside this repository and finds no MCP code, so nothing in this tree can show it done"
  },
  {
    "file": ".engineering/planning/story/expired-evidence-invoke-advises-revalidate.md",
    "line": 52,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "undecided",
    "message": "the acceptance offers two different outcomes joined by \"Or:\" (a revalidate advice versus transparent revalidation \"if the contract allows it\") and leaves the choice to the implementor, so a check cannot know which behaviour to expect"
  },
  {
    "file": ".engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md",
    "line": 48,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "undecided",
    "message": "\"stays within a stated bound\" names no bound, so any measured time passes, and the same sentence joins a second independent outcome (the ~700-invoke workload completing with no admission timeout)"
  },
  {
    "file": ".engineering/planning/story/metadata-invoke-cost-flat-in-store-size.md",
    "line": 46,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "undecided",
    "message": "the first acceptance item requires Entity Runtime subject-scoped reads flat in unrelated events, but the body records 0.26.0 adopted, execute_batch flat, the scoped read_history measured slower and not kept, and er.start still growing, so the item no longer describes what is left to verify"
  },
  {
    "file": ".engineering/planning/story/owner-memory-bounded.md",
    "line": 36,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "undecided",
    "message": "the last acceptance item passes either on peak RSS at 1,200 events being at most twice that at 600 or on filing an upstream change, so the story closes with no change in peak RSS"
  },
  {
    "file": ".engineering/planning/story/registry-clock-floor-growth.md",
    "line": 51,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "undecided",
    "message": "the second acceptance item mixes a share (\"the share of clock floor events falls by at least half\", 59% observed) with a count (\"2026-10-06's 7 per invoke\"), and the harness reports total appended_per_invoke rather than clock-floor events, so two readers compute different thresholds"
  }
]
```
