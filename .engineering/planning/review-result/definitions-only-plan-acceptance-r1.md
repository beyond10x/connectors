---
format: aep.planning-md/3
id: review-result:definitions-only-plan-acceptance-r1
kind: review-result
status: active
title: Plan critic acceptance, round 1, on the definitions-only drafts
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
story:definition-tests-read-pinned-release — the acceptance says no Scope file reads a path under `adapters/catalog/providers` or `generated/bundles`, but Context line 44 says `definition_import.rs`, which is in Scope, keeps comparing the release with those in-tree bundles and selections, and the file reads them at lines 122, 162, 175, 216, 332 and 495; the acceptance can never pass as written, so exempt that file or add it to "Not in this story" — .engineering/planning/story/definition-tests-read-pinned-release.md:77
story:definitions-release-index — the acceptance ("entries match the SHA-256 of every file ... in a checkout of that tag") is true for a first tag that still lacks the 0.55.0 Jira postflight check, although Context says that tag would turn the byte-equality test red, so it names no observable condition on `connectors/jira.yaml` at the tagged commit — .engineering/planning/story/definitions-release-index.md:74
story:catalog-tests-read-pinned-release-gitlab-engine — "None of the 45 files ... reads a path under providers/ ... and each one still in the tree passes" holds just as well if every file is deleted or keeps using compiled-in data, because the acceptance neither requires the promised list of removed tests nor says how reading the release is observed — .engineering/planning/story/catalog-tests-read-pinned-release-gitlab-engine.md:75
story:catalog-tests-read-pinned-release-providers — the same defect: "each one still in the tree passes in the gate reading the pinned release" is vacuous for deleted files and does not require the list of removed tests that Context line 32 promises — .engineering/planning/story/catalog-tests-read-pinned-release-providers.md:67
story:definitions-install — one sentence joins the install success to four independent refusals, and the "tag moved to another commit" refusal names no prior lock to have moved from, so "writes neither lock nor cache" reads differently when a lock already exists — .engineering/planning/story/definitions-install.md:80
story:definitions-pin-covers-catalog — the Outcome makes the check red when a definition "does not lower", but the acceptance only tests a selection the release lacks, so that branch has no observable check — .engineering/planning/story/definitions-pin-covers-catalog.md:64

What I read: all 7 drafts, `decision-blocker:catalog-first-run-source` and the epic, with `aep plan artifact show` on each. I checked the Scope lists with `git grep` at 9201b1177: the 80 listed catalog test files match the grep set exactly, there are 15 selections and 15 definitions in `connectors-definitions` origin/main, the fixture counts are 15, 3 and 15, and the fixture path is read by the 5 named files. The aep `kinds` command ran, but I did not run `lifecycle`.
Could not establish: whether the release asset redirect matters for the install test (the story marks it not verified). Out of my lane: the first tag depends on the open PR 4 in `connectors-definitions` (GitHub shows it OPEN), which is an ordering and coupling matter for the design critic. `definition_import.rs` is also kept for byte-equality while the fixture copy is deleted by `story:definitions-fixture-copy-removed`, and I have not checked whether that is consistent. `story:definitions-fixture-copy-removed` is not a finding: its acceptance is observable and checks a before/after state.

```findings
- file: .engineering/planning/story/definition-tests-read-pinned-release.md
  line: 77
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says no Scope file reads a path under adapters/catalog/providers or generated/bundles, but Context line 44 says definition_import.rs, which is in Scope, keeps comparing the release with those in-tree bundles and selections (it reads them at lines 122, 162, 175, 216, 332, 495), so the acceptance cannot pass as written"
- file: .engineering/planning/story/definitions-release-index.md
  line: 74
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is true for a first tag that still lacks the 0.55.0 Jira postflight check, although Context says that tag turns the byte-equality test red, so it names no observable condition on connectors/jira.yaml at the tagged commit"
- file: .engineering/planning/story/catalog-tests-read-pinned-release-gitlab-engine.md
  line: 75
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance holds if every one of the 45 files is deleted or keeps using compiled-in data, because it neither requires the promised list of removed tests nor says how reading the release is observed"
- file: .engineering/planning/story/catalog-tests-read-pinned-release-providers.md
  line: 67
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance holds if every one of the 35 files is deleted or keeps using compiled-in data, because it does not require the list of removed tests that Context line 32 promises or say how reading the release is observed"
- file: .engineering/planning/story/definitions-install.md
  line: 80
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "one sentence joins the install success to four independent refusals, and the 'tag moved to another commit' refusal names no prior lock to have moved from, so 'writes neither lock nor cache' reads differently when a lock already exists"
- file: .engineering/planning/story/definitions-pin-covers-catalog.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome makes the check red when a definition does not lower, but the acceptance only tests a selection the pinned release lacks, so that branch has no observable check"
```
