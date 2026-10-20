---
format: aep.planning-md/3
id: review-result:definitions-only-plan-design-r1
kind: review-result
status: active
title: Plan critic design, round 1, on the definitions-only drafts
relations:
- reviews: story:definitions-release-index
- reviews: story:definitions-install
- reviews: story:definitions-pin-covers-catalog
- reviews: story:definition-tests-read-pinned-release
- reviews: story:catalog-tests-read-pinned-release-gitlab-engine
- reviews: story:catalog-tests-read-pinned-release-providers
- reviews: story:definitions-fixture-copy-removed
revision: 1
---
needs-revision
story:definitions-release-index — the fetch path (archive beside `index.json`, or the tagged tree fetched by ref) is deferred to `story:definitions-install` as "agreed with", while `story:definitions-install` says it is "chosen here together with" this story and depends on this story, so neither owns the transport seam. This story lands first and must publish something, yet its Outcome and Acceptance name only `index.json`. Make this story's Outcome name what is published for the installer to fetch, or have `story:definitions-install` own the choice and say what this story must publish — .engineering/planning/story/definitions-release-index.md:35, .engineering/planning/story/definitions-install.md:40
story:definitions-pin-covers-catalog — the shared test-support function that `story:definition-tests-read-pinned-release`, `story:catalog-tests-read-pinned-release-gitlab-engine` and `story:catalog-tests-read-pinned-release-providers` all read "through the test support of" this story appears only in Context and Scope ("inferred"), not in its Outcome (lines 15-18). The check can pass with its logic inline and the three readers stay blocked, so the producer of a shared surface is left unstated. State the function in the Outcome, or split it into its own story that the three depend on — .engineering/planning/story/definitions-pin-covers-catalog.md:35, .engineering/planning/story/definition-tests-read-pinned-release.md:16

What I read: 8 artifacts (epic, 7 stories) plus `decision-blocker:catalog-first-run-source` and the ADR, via `aep plan artifact show`, `relations`, `graph` and `validate`.
- **Edges:** I walked 24 declared edges touching the set, including the edges to the epic, the vision and the ADR.
- **Cycles:** none. The `depends_on` graph is release-index ← install ← pin ← {definition-tests, gitlab-engine, providers} ← fixture-copy-removed.
- **Reads traced:** 9 acceptance reads traced to a producer in the set. All 9 are recorded by an edge, directly or transitively. The pin story's read of the first tag reaches release-index only through install.
- **Other checks:** the store and the tree (`git grep`) agree that 80 test files are listed in the two catalog stories. The only other tests reading `providers/` or `generated/bundles` are the 3 declared exclusions. `engine.rs` matches on "providers" in test names, which is a false positive.
- **Chain:** the three-story serial prefix and the 3-wide fan-out each have a written reason, so I am not calling it a serialising chain.
- **Validator:** `aep plan artifact validate` reported nothing on the 8 artifacts.

What I could not establish:
- **Out of my lane (acceptance):** `story:definition-tests-read-pinned-release` Acceptance says no Scope file reads `adapters/catalog/providers` or `generated/bundles`. Its Context says `definition_import.rs`, which is in Scope, keeps comparing against the in-tree bundles. `definition_import.rs` reads them at lines 122, 162, 175, 216, 332 and 495, so the acceptance contradicts the Context.
- **External dependency:** the first tag must follow connectors-definitions pull request 4 (Jira postflight), or `definition_import.rs` goes red. That dependency is outside the set, so it is not a design defect. `story:definitions-release-index` Acceptance does not require it.
- **Open blocker:** `decision-blocker:catalog-first-run-source` blocks only the epic. The stories avoid the first-run question, so I found no hidden dependency on it. The deletion of `adapters/catalog/providers` and `generated/bundles` is correctly left undrafted.

```findings
- file: .engineering/planning/story/definitions-release-index.md
  line: 35
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the fetch path (archive beside index.json, or tagged tree by ref) is deferred to story:definitions-install as 'agreed with', while that story says it is 'chosen here together with' this one and depends on this one, so neither owns the transport seam; this story's Outcome should name what is published for the installer to fetch, or story:definitions-install should own the choice and state what this story must publish"
- file: .engineering/planning/story/definitions-pin-covers-catalog.md
  line: 35
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the shared test-support function that three sibling stories read 'through the test support of' this story appears only in Context and Scope, not in its Outcome, so the producer of the shared surface is unstated; state it in the Outcome or split it into its own story the three depend on"
```
