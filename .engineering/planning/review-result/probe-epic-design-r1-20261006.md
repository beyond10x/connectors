---
format: aep.planning-md/3
id: review-result:probe-epic-design-r1-20261006
kind: review-result
status: active
title: Design critic, round 1, epic:connector-probe-20261006
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

I read all 10 stories, the epic and the superseded story:toolchain-pins-newest-release-20261001, and found nothing that would change the drafted shape.

- **Cycles:** none. The edges are `depends_on` (story:cli-json-answers-as-json to story:dependency-refresh-20261006, and story:registry-clock-floor-growth to the same story) and `supersedes`. Both `depends_on` edges point at the same story, so the set is a fan-in, not a queue.
- **Split abstraction:** none. The set is cut by defect, and each story is a complete change with its own acceptance. The nearest case is story:registry-clock-floor-growth against story:metadata-invoke-cost-flat-in-store-size. Both aim at cost per invoke, but one reduces recorded events and the other adopts an upstream fix. The earlier mitigation, story:registry-clock-outside-shared-batches, is implemented and rejected, so it does not overlap.
- **Hidden dependency:** none. The one `depends_on` edge each of the two stories needs is declared.
- **Horizontal slice:** none.

**What I read:** 11 artifacts via `aep plan artifact list --format json` and `show` on each. I also ran `relations`, `graph` (I read about 27 edges, the set's own plus those to artifacts outside it, such as story:registry-clock-outside-shared-batches, upstream-blocker:entity-runtime-batch-closure-cost and epic:tech-debt-review-20260930) and `validate` (it printed `valid`; the only other output was stale review-result notices unrelated to this set).

**Out of my lane, not setting the verdict:**
- **Shared files and no ordering edge (parallel safety):**
  - story:expired-evidence-invoke-advises-revalidate, story:service-failure-carries-upstream-reason, story:upgrade-adapter-identity-mismatch-names-new-connection and story:cli-json-answers-as-json all touch `apps/connectors/src/local.rs`, `apps/connectors-cli-contract/binding.json` and `ess/domains/cli.yaml`. Only story:cli-json-answers-as-json is ordered after story:dependency-refresh-20261006, whose body says to schedule it first or alone.
  - story:owner-memory-bounded, story:registry-clock-floor-growth and story:metadata-invoke-cost-flat-in-store-size share `metadata/er.rs` and `store_cost_tests.rs`.
  - story:owner-memory-bounded and story:metadata-invoke-cost-flat-in-store-size list `Cargo.toml` and `Cargo.lock` with no edge to story:dependency-refresh-20261006.
  - story:service-failure-carries-upstream-reason and story:catalog-honours-retry-after share `adapters/catalog/src/lib.rs`.
- **Stale ordering sentence (scope):** story:catalog-honours-retry-after still says it "lands after wave 20260930c (Google)", but the Google stories in the store are all `implemented`. It also names no artifact id.
- **Missing blocker artifact (scope):** the epic says story:metadata-invoke-cost-flat-in-store-size stays blocked on beyond10x/entity-runtime#55. The store has no artifact for #55. The only upstream-blocker on that story is still `open` and cites #51, which the story's own body records as closed.
- **Acceptance wording (acceptance):** story:expired-evidence-invoke-advises-revalidate leaves the choice between two outcomes to "the story decides". I did not judge it.

```findings
[]
```
