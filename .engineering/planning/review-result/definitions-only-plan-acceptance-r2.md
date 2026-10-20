---
format: aep.planning-md/3
id: review-result:definitions-only-plan-acceptance-r2
kind: review-result
status: active
title: Plan critic acceptance, round 2, on the definitions-only drafts
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
needs-revision
story:instances-resolve-through-locked-index — the acceptance joins two independent outcomes in one sentence with "and" (the instance runs on the declaration, selection and bundle lowered from the locked release; after installing another tag the same file runs on the new selection), so one can pass while the other fails, and the second names only the selection, not the declaration or bundle — .engineering/planning/story/instances-resolve-through-locked-index.md:90

Round-1 findings, all six fixed:
- **`definition-tests-read-pinned-release`:** Outcome and Acceptance now exempt `definition_import.rs` by name, and "Not in this story" lists its comparison.
- **`definitions-release-index`:** a second Acceptance bullet names the `jira.yaml` checks at the tagged commit. The `issue.create` and `issue.comment.add` text matches the fixture copy, and origin/main `connectors/jira.yaml:77` lacks the `after:` line, so it is checkable.
- **`catalog-tests-read-pinned-release-gitlab-engine` and `-providers`:** each Acceptance now says to build the test binaries first and then rename the three paths away. The gitlab-engine story also requires a `## Removed tests` section and says the 18 engine and security files must not call the pinned-release test support.
- **`definitions-install`:** one bullet per case (success, digest mismatch, unknown format version, unmet feature). The "tag moved" refusal is gone. Each refusal states the lock and cache are as they were before the command.
- **`definitions-pin-covers-catalog`:** a bullet now covers a pinned definition that does not lower.

Judged fresh, `definitions-fixture-copy-removed` is fine: both paths gone, `git grep` finds neither, gate passes.

What I read: 9 artifacts with `aep plan artifact show` — the 8 drafts, `review-result:definitions-only-plan-acceptance-r1` and `decision-blocker:catalog-first-run-source` for context. I also ran `aep plan artifact kinds`, and compared origin/main `connectors/jira.yaml` with the in-tree fixture copy.

What I could not establish:
- **Other CI workflows:** `definitions-pin-covers-catalog` lists `pages.yml`, `macos-host.yml` and `windows-host.yml` as installing the pinned release. Its Acceptance observes only the gate check, so nothing shows those workflows install it. I did not count that as a finding.
- **Out of my lane, ordering:** the first tag still depends on `connectors-definitions` pull request 4 merging first. This is for the design critic.
- **Open question:** the release-asset redirect, which the stories mark as not verified.

```findings
- file: .engineering/planning/story/instances-resolve-through-locked-index.md
  line: 90
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes in one sentence with 'and' (the instance runs on the declaration, selection and bundle lowered from the locked release; after installing another tag the same file runs on the new selection), so one can pass while the other fails, and the second names only the selection, not the declaration or bundle"
```
