---
format: aep.planning-md/3
id: review-result:definitions-only-plan-scope-r1
kind: review-result
status: active
title: Plan critic scope, round 1, on the definitions-only drafts
relations:
- reviews: story:definitions-release-index
- reviews: story:definitions-install
- reviews: story:definitions-pin-covers-catalog
- reviews: story:definition-tests-read-pinned-release
- reviews: story:catalog-tests-read-pinned-release-gitlab-engine
- reviews: story:catalog-tests-read-pinned-release-providers
- reviews: story:definitions-fixture-copy-removed
- reviews: epic:catalog-connectors-only-in-definitions
revision: 1
---
needs-revision
epic:catalog-connectors-only-in-definitions — "connectors-definitions publishes what D5 installs (a tagged release with `index.json`; D10 pages)" is promised, and the only story that publishes (story:definitions-release-index) drops D10 pages under "Not in this story" with no reason, no other item and no blocker; add it to that story or record it as undecided — .engineering/planning/epic/catalog-connectors-only-in-definitions.md:50 and .engineering/planning/story/definitions-release-index.md:67
epic:catalog-connectors-only-in-definitions — "instance configuration names the locked index" is promised, and the only item that mentions it (story:definitions-install, "Instances resolving a connector through the lock…") defers it to "the first-run question", but decision-blocker:catalog-first-run-source asks only where a binary gets connectors before any install, so nothing claims or blocks the post-install promise; a story or an explicit blocker line should take it — .engineering/planning/epic/catalog-connectors-only-in-definitions.md:47 and .engineering/planning/story/definitions-install.md:73
story:catalog-tests-read-pinned-release-gitlab-engine — the epic says "The engine and lowerer tests keep a small synthetic definition of their own instead of a provider copy", but the 18 "engine and security" files (`engine_adversary.rs`, `parameter_types.rs`, `gateway_prefix.rs`, `selection_bounds_adversary.rs` all read `providers/gitlab/operations.json` and `generated/bundles` today) are routed to the pinned release with no synthetic-definition route, so the promise is narrowed for them without a recorded decision — .engineering/planning/epic/catalog-connectors-only-in-definitions.md:48 and .engineering/planning/story/catalog-tests-read-pinned-release-gitlab-engine.md:58
story:definitions-install — the acceptance and Context add a refusal for "a tag moved to another commit" that comes only from the draft design section 6, not from the epic or ADR D5 (D5 names digest mismatch, unknown format version and unmet feature); drop it or mark it as an addition for the operator — .engineering/planning/story/definitions-install.md:30,82

Checked and not findings. The deletion of `adapters/catalog/providers` and `generated/bundles` is held back by decision-blocker:catalog-first-run-source, and the epic names it as an open question, so I did not count it as a gap. My `git grep` at 9201b1177 found no unclaimed test file: the 80 files in the two catalog batches match the files that read those paths, and what is left out (`bundle_drift.rs`, `connector_declarations.rs`, `guide_instances.rs`, `hand_written/`) is named in the stories. Every reader under `crates/` and `apps/` outside the catalog tests, apart from the excluded `migrate.rs`, `setup_migrate.rs` and `import.rs`, is in story:definition-tests-read-pinned-release.

What I read: 9 artifacts (the epic, the 7 drafts and the blocker), with `aep plan artifact show`, `list` and `graph`. I also ran `git grep` at 9201b1177 to check test coverage. Of 9 promises extracted from the epic, 6 trace to an item without a finding. Those 6 are the 14 providers plus `slack-user` check, the install command, the tests moved to the pinned release, the fixture copy, the first tag with `index.json`, and the native connectors staying. The other 3 are the findings above. The graph shows no older set that claims any of them.

What I could not establish, out of my lane:
- Acceptance: story:definitions-release-index has no acceptance that Jira pull request 4 has merged before the first tag. Its Context says a tag cut earlier turns `definition_import` red.
- Parallel safety: the two catalog batches both edit `adapters/catalog/tests/` but share no files.

```findings
[
  {"file": ".engineering/planning/epic/catalog-connectors-only-in-definitions.md", "line": 50, "category": "scope", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "\"connectors-definitions publishes what D5 installs (a tagged release with `index.json`; D10 pages)\" is promised, and story:definitions-release-index drops D10 pages under Not in this story (definitions-release-index.md:67) with no reason, no other item and no blocker; add it to that story or record it as undecided"},
  {"file": ".engineering/planning/epic/catalog-connectors-only-in-definitions.md", "line": 47, "category": "scope", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "\"instance configuration names the locked index\" is promised, story:definitions-install defers it to the first-run question (definitions-install.md:73), but decision-blocker:catalog-first-run-source covers only the source before any install, so no item claims or blocks the post-install promise; a story or an explicit blocker line should take it"},
  {"file": ".engineering/planning/story/catalog-tests-read-pinned-release-gitlab-engine.md", "line": 58, "category": "scope", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the epic says \"The engine and lowerer tests keep a small synthetic definition of their own instead of a provider copy\" (epic:48), but the 18 engine and security files are routed to the pinned release with no synthetic-definition route, which narrows the promise without a recorded decision"},
  {"file": ".engineering/planning/story/definitions-install.md", "line": 82, "category": "scope", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance adds a refusal for \"a tag moved to another commit\" that comes only from the draft design section 6 (definitions-install.md:30), not from the epic or ADR D5; drop it or mark it as an addition for the operator"}
]
```
