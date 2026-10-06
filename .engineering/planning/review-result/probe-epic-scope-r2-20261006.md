---
format: aep.planning-md/3
id: review-result:probe-epic-scope-r2-20261006
kind: review-result
status: active
title: Scope critic, round 2, epic:connector-probe-20261006
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
- reviews: epic:connector-probe-20261006
revision: 1
---
needs-revision

story:catalog-honours-retry-after — the epic promises "Rate-limited reads (`429`) honouring `Retry-After`" for its five live connections, but this story's title, Defect and acceptance cover only the catalog engine, and nothing says the native Tavily adapter is left out, though it maps 429 to `RateLimited` at `adapters/tavily/src/lib.rs:148` with no `Retry-After` handling — .engineering/planning/epic/connector-probe-20261006.md:30

**Round 1 finding.** The "examples triage demo, the MCP adapter" acceptance item is gone from `story:cli-json-answers-as-json`. The story now says "Not in this story: the beyond10x/examples triage demo" at `.engineering/planning/story/cli-json-answers-as-json.md:63`. That finding no longer holds.

**What you read:**
- 11 artifacts: the epic and its 10 `decomposes` stories.
- Commands: `aep plan artifact show` on the epic and on each story (read the epic first), `aep plan artifact graph`, `aep plan artifact relations`, and `aep plan artifact show review-result:probe-epic-scope-r1-20261006`.
- I also read `gh issue view 105`, `grep` of 429 handling in `adapters/`, and the shipped catalog providers.
- I extracted 13 promises from the epic. 12 trace to exactly one story; the thirteenth is the Tavily half of the `Retry-After` promise, which is the finding above.
- Promise to story mapping:

| Epic promise | Story |
|---|---|
| Registry clock floor (#101) | `story:registry-clock-floor-growth` |
| Owner memory (#103) | `story:owner-memory-bounded` |
| Expired evidence | `story:expired-evidence-invoke-advises-revalidate` |
| Identity mismatch during an upgrade | `story:upgrade-adapter-identity-mismatch-names-new-connection` |
| Dispatch `unauthorized` reason | `story:service-failure-carries-upstream-reason` |
| `429` and `Retry-After` | `story:catalog-honours-retry-after` (catalog engine only) |
| JSON as JSON (#105) | `story:cli-json-answers-as-json` |
| GitLab issue creation (#81) | `story:forge-issue-create` |
| crates.io updates, ESS 0.53.0, AEP 0.68.0, toolchain | `story:dependency-refresh-20261006` |
| Per-command metadata cost stops growing | `story:metadata-invoke-cost-flat-in-store-size`, which the epic excludes at `:37-38` |

- `story:metadata-invoke-cost-flat-in-store-size` is the tenth story, and the epic names it as blocked, so it is not reach.

**What you could not establish:**
- `story:cli-json-answers-as-json` also commits `adapters describe` and the legacy describe to JSON (acceptance, second bullet), which the epic does not name. Its own Safety fact says this follows from the shared `OperationDescription` type, so I did not file it. If the types are split, that clause is reach.
- `story:upgrade-adapter-identity-mismatch-names-new-connection` adds only a test, because the behaviour shipped in 0.30.0. Its Defect says so, so the narrowing is recorded.
- Whether the clock-floor halving and the 2x memory bound meet the epic's "stop growing" wording is the acceptance lane. Whether the six stories sharing `local.rs`, `er.rs` or `Cargo.lock` can run together is the parallel-safety lane. Neither set my verdict.

```findings
[
  {"file": ".engineering/planning/epic/connector-probe-20261006.md", "line": 30, "category": "scope", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the epic promises \"Rate-limited reads (`429`) honouring `Retry-After`\" for its five live connections, but story:catalog-honours-retry-after covers only the catalog engine and nothing says the native Tavily adapter is left out, though it maps 429 to RateLimited at adapters/tavily/src/lib.rs:148 with no Retry-After handling"}
]
```
