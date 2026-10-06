---
format: aep.planning-md/3
id: review-result:probe-epic-parallel-safety-r1-20261006
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic:connector-probe-20261006
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

- `story:expired-evidence-invoke-advises-revalidate` — both it and `story:service-failure-carries-upstream-reason` land on `apps/connectors/src/local.rs`, `crates/connectors-host/src/local/owner.rs` and `contracts/cli/v1alpha1/semantics.md` (cited in both scope fields), plus `binding.json` and `ess/domains/cli.yaml` (inferred), and neither body says so. Fix with an ordering edge that records the shared file, or split the surface — `.engineering/planning/story/expired-evidence-invoke-advises-revalidate.md:47`
- `story:expired-evidence-invoke-advises-revalidate` — its scope names 13 files across the CLI, the owner and the registry because the acceptance leaves the fix site undecided ("`retry_status` or a dedicated revalidate action … Or: invoke revalidates transparently … the story decides which"). That collides with six of the other nine items (service-failure, upgrade-mismatch, cli-json, retry-after, registry-clock, forge-issue-create). Decide the approach, or narrow the scope, before it is scheduled concurrently — `.engineering/planning/story/expired-evidence-invoke-advises-revalidate.md:47-52`
- `story:expired-evidence-invoke-advises-revalidate` — it shares `crates/connectors-host/src/local/registry/tests.rs` with `story:registry-clock-floor-growth` (cited in the expired scope, and cited at `:395` in the registry-clock body). Registry-clock's "Would collide with" list omits `registry/tests.rs` and neither body names the other — `.engineering/planning/story/registry-clock-floor-growth.md:77`
- `story:catalog-honours-retry-after` — it shares `adapters/catalog/src/lib.rs` with `story:service-failure-carries-upstream-reason`. Both edit the same status-to-`ErrorCode` match in `read_body` (`lib.rs:895-908`, where `401` and `429` are adjacent), and its Ordering section names only wave `20260930c`. Carrying the delay in the refusal would also reach `runtime.rs` and `owner.rs`, which service-failure cites. The scope field omits them, so that overlap is inferred. Name the pair, or order them — `.engineering/planning/story/catalog-honours-retry-after.md:49`
- `story:catalog-honours-retry-after` — it shares `docs/local-catalog-provider.md` with `story:expired-evidence-invoke-advises-revalidate` (cited in both) and `story:forge-issue-create` (inferred on the forge side). The Ordering section names none of them — `.engineering/planning/story/catalog-honours-retry-after.md:37-49`

**What I read:** 10 stories with a `decomposes` edge to the epic, the epic itself, and the derived waves and collision list. Commands: `aep plan artifact list --format json`, `aep plan artifact show` on each, `aep plan artifact waves --kind story --status draft --format json`, and `git grep` in the tree at `80bee2fb58`. Surface placement: all 10 have at least one cited path, none are wholly inferred and none are unplaceable. The code site of `story:catalog-honours-retry-after` is only inferred in its own scope field. I confirmed the `lib.rs` overlap with service-failure from the tree.

**What I could not establish:**
- `story:dependency-refresh-20261006` states that any unit touching `Cargo.*`, the generated CLI contract or `ess/` goes after it, and only cli-json and registry-clock carry a `depends_on` edge to it. The waves tool matches exact paths and missed its directory-prefix overlaps with expired and service-failure (`apps/connectors-cli-contract`, `ess/domains`). I did not raise this because the body states the rule.
- `story:metadata-invoke-cost-flat-in-store-size` is active and blocked upstream, so I treated it as not concurrent. Once unblocked it shares `metadata/er.rs`, `registry.rs`, `registry/tests.rs` and `Cargo.*` with registry-clock, owner-memory and dependency-refresh. Only owner-memory names it.
- Out of my lane: whether `story:cli-json-answers-as-json` should split `OperationDescription` from the legacy describe type (design), and whether the unproven safety facts are checkable (acceptance).

```findings
[
  {"file": ".engineering/planning/story/expired-evidence-invoke-advises-revalidate.md", "line": 47, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "shares apps/connectors/src/local.rs, crates/connectors-host/src/local/owner.rs and contracts/cli/v1alpha1/semantics.md (cited) and binding.json and ess/domains/cli.yaml (inferred) with story:service-failure-carries-upstream-reason, and neither body says so; add an ordering edge recording the shared file or split the surface"},
  {"file": ".engineering/planning/story/expired-evidence-invoke-advises-revalidate.md", "line": 47, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "scope names 13 files across CLI, owner and registry because the acceptance leaves the fix site undecided (retry_status, dedicated action, or transparent revalidate), so it collides with six of nine other items; decide the approach or narrow the scope before concurrent scheduling"},
  {"file": ".engineering/planning/story/registry-clock-floor-growth.md", "line": 77, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "shares crates/connectors-host/src/local/registry/tests.rs with story:expired-evidence-invoke-advises-revalidate (cited in both) and its Would-collide list and Ordering omit it; add an ordering edge or split the surface"},
  {"file": ".engineering/planning/story/catalog-honours-retry-after.md", "line": 49, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "shares adapters/catalog/src/lib.rs (the read_body status match, lib.rs:895-908) with story:service-failure-carries-upstream-reason and the Ordering section names neither it nor the runtime.rs/owner.rs path the delay would travel (that part inferred); add an ordering edge or split the surface"},
  {"file": ".engineering/planning/story/catalog-honours-retry-after.md", "line": 37, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "shares docs/local-catalog-provider.md with story:expired-evidence-invoke-advises-revalidate (cited in both) and story:forge-issue-create (inferred on the forge side) and names neither; record the shared file in an ordering edge or move the edits to separate files"}
]
```
