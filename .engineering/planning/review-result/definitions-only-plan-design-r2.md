---
format: aep.planning-md/3
id: review-result:definitions-only-plan-design-r2
kind: review-result
status: active
title: Plan critic design, round 2, on the definitions-only drafts
relations:
- reviews: story:definitions-release-index
- reviews: story:definitions-install
- reviews: story:definitions-pin-covers-catalog
- reviews: story:definition-tests-read-pinned-release
- reviews: story:catalog-tests-read-pinned-release-gitlab-engine
- reviews: story:catalog-tests-read-pinned-release-providers
- reviews: story:definitions-fixture-copy-removed
- reviews: story:instances-resolve-through-locked-index
revision: 1
---
approve

1. Round-1 finding on `story:definitions-release-index` (transport seam) is fixed. Its Outcome now names the tag, the release asset and `index.json`, and says it "owns what is published". `story:definitions-install` consumes "exactly these three" (`.engineering/planning/story/definitions-release-index.md:13-24`, `.engineering/planning/story/definitions-install.md:16-19`).
2. Round-1 finding on `story:definitions-pin-covers-catalog` (shared test-support function) is fixed. The function is now in its Outcome. It adds the dev-dependency to every consuming manifest and to `Cargo.lock`, and the Acceptance checks that three crates list it (`.engineering/planning/story/definitions-pin-covers-catalog.md:14-27,76-77`).
3. Cycles: none.
   - The `depends_on` graph is release-index ← install ← {pin, instances}, pin ← {definition-tests, gitlab-engine, providers} ← fixture-copy-removed.
   - `decision-blocker:catalog-first-run-source` blocks only the epic and `story:instances-resolve-through-locked-index`.
4. Chain: the serial prefix of three stories and the 3-wide fan-out each have a written reason (the first tag, the install, the shared test support). Instances branches off install and is not queued behind the pin.
5. Slices: each story leaves the tree green and can be shown working. The batches use the three-way mechanical-migration shape, so I do not call them a horizontal slice. The new instances story is a vertical change from lock to running instance.

What I read: 8 drafts, the blocker and round-1 record `review-result:definitions-only-plan-design-r1`. I used `aep plan artifact show/relations/graph/waves/validate` and read `adapters/catalog/src/local.rs` and tests with `git grep` at 9201b1177. I walked the 27 relation lines of the 9 ids, 9 of them `depends_on`, and also read the graph edges to the epic, vision and ADR. Cycle check: no cycle among the set and its neighbours. I traced 11 acceptance reads to a producer in the set, and an edge records 11 of them. Three go through a transitive edge: pin to release-index via install, and both catalog batches to install via pin. `validate` printed nothing on the set.

What I could not establish:
- Possible seam, not a finding. `adapters/catalog/tests/local_runtime.rs:45-46` includes `local_runtime/derived_instance.rs` as a module. The providers batch scopes `local_runtime.rs` and the instances story scopes `derived_instance.rs`. They are two files in one test binary, and the instances story is blocked. Whether they can be worked at once is parallel-safety's lane. The providers batch's claim that "many files here do both" is only partly confirmed: `git grep` finds derived-instance use in `derived_instance.rs`, `invoke_definition.rs` and `recorded_state.rs`.
- Out of lane (acceptance, from round 1, now fixed): `story:definition-tests-read-pinned-release` Acceptance now exempts `definition_import.rs` by name, which matches its Context.
- Outside the set, so not a design defect: the first tag must follow connectors-definitions pull request 4. `story:definitions-release-index` states it in Context and Acceptance.
- Outside the set: where the shipped binary gets its connectors before any install is the open blocker, and the instances story waits on it.

```findings
[]
```
