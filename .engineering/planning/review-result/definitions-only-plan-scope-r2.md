---
format: aep.planning-md/3
id: review-result:definitions-only-plan-scope-r2
kind: review-result
status: active
title: Plan critic scope, round 2, on the definitions-only drafts
relations:
- reviews: story:definitions-release-index
- reviews: story:definitions-install
- reviews: story:definitions-pin-covers-catalog
- reviews: story:definition-tests-read-pinned-release
- reviews: story:catalog-tests-read-pinned-release-gitlab-engine
- reviews: story:catalog-tests-read-pinned-release-providers
- reviews: story:definitions-fixture-copy-removed
- reviews: story:instances-resolve-through-locked-index
- reviews: epic:catalog-connectors-only-in-definitions
revision: 1
---
approve

Round-1 findings, each checked against the revised set:

| Round-1 finding | Status | Evidence |
|---|---|---|
| D10 pages promised under "publishes" but dropped without a reason | Fixed | The epic (revision 2) now lists D10 under "Not in scope" and says it is planned on its own. `story:definitions-release-index` cites that line under "Not in this story". |
| "instance configuration names the locked index" claimed by no item | Fixed | New `story:instances-resolve-through-locked-index`, blocked by `decision-blocker:catalog-first-run-source`. `story:definitions-install` points to it under "Not in this story". |
| Engine tests had no synthetic-definition route | Fixed | `story:catalog-tests-read-pinned-release-gitlab-engine` splits the 45 files into 27 pinned-release and 18 synthetic. Its acceptance says none of the 18 calls the pinned-release test support. |
| Install refusal for "a tag moved" came from the design draft, not the epic or D5 | Fixed | `story:definitions-install` now says it is "not in this story". Its acceptance holds only the three D5 refusals. |

Fresh judgment of the 8 drafts against the epic as written:
- Reach beyond the parent: none found. The pin, the gate install and the test-support function in `story:definitions-pin-covers-catalog` are the machinery for epic steps 2 and 3.
- Two items claiming one outcome: none found. `story:definitions-install` and `story:instances-resolve-through-locked-index` are separated by an explicit "Not in this story". The three test stories are file-disjoint.
- Silent narrowing: none found. Everything left out is named in a story's "Not in this story" or on the epic's open questions.

What I read: 11 artifacts (the epic, the 8 drafts, the blocker and my round-1 record), with `aep plan artifact show` and `graph`. I also ran `git grep` at 9201b1177 for readers of `catalog/providers`, `generated/bundles` and `tests/fixtures/definition`. All 102 reader files under `adapters/catalog/tests/` are accounted for. The 82 `.rs` files are in the two batches or named as excluded. The 20 `.json` files (`hand_written/*`, `connector_declarations/gmail.json`) are tree-path data that `connector_declarations.rs` maps (`connector_declarations.rs:104-135`); that file is in the scope of `story:instances-resolve-through-locked-index`, and the batches name `hand_written/` as untouched. The readers outside that directory are covered by `story:definition-tests-read-pinned-release`, `story:instances-resolve-through-locked-index`, or named as going with the deletion.

Promises: 12 extracted from the epic, 10 traced to an item. The other 2 are the deletion of `adapters/catalog/providers` and of `generated/bundles` with `index.json` (epic lines 38-39 and Order step 4). They are not claimed by any item. The drafter named them in `story:definitions-fixture-copy-removed` Context ("not drafted"), `decision-blocker:catalog-first-run-source` blocks the epic on them, and the epic lists the question as open. I counted them as an honest omission, not a gap.

What I could not establish:
- The deletion has no named owner after the blocker clears. `story:definitions-fixture-copy-removed` calls `story:instances-resolve-through-locked-index` the replacement, but that story's "Not in this story" excludes the deletion. This is not a finding, because the blocker is open and the drafter said "not drafted". Whoever drafts the deletion after the blocker clears should take the pointer from the epic.
- Unease, not a finding: `story:definitions-pin-covers-catalog` reads the epic's "exists in connectors-definitions at the pinned tag" as "exists and lowers without refusal". That is a small strengthening, and the story records it in Context.
- Out of my lane (acceptance): the release index asks for `index.json` to be "written after tagging and published beside the tag". The acceptance does not say how a reader tells that the index's commit equals the tagged commit.

```findings
[]
```
