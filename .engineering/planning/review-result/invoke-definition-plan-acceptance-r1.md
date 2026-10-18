---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-acceptance-r1
kind: review-result
status: active
title: Plan critic acceptance, round 1, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
needs-revision

story:invoke-definition-file — the acceptance says the first run "answers the one token prompt", but the Outcome's Authentication bullet has the command ask a connect yes/no question first and then show the token prompt, so the acceptance must say that the question is answered "yes" and then the token is entered, or the two do not describe the same observable run — .engineering/planning/story/invoke-definition-file.md:99 (against :86-87)

**What I read:** 3 of 3 ids (story:connection-binding-excludes-operations, story:invoke-definition-file, story:invoke-definition-reuse-credential). I ran `aep plan artifact show <id>` for each and `aep plan artifact lifecycle story`, using the 0.71.3 binary. I also read both rubric files, and used `git grep` and `sed` on the tree. That showed `ready` is a real connection state (`adapters/catalog/tests/local_runtime/revision_recovery.rs:67,277`). `revision_recovery.rs`, `cli_journey.rs` and `definition_auth.rs` exist.

**What I could not establish:**
- Whether the connect question and the token prompt count as one prompt or two in the intended test harness.
- Unease, not findings: the acceptance of story:connection-binding-excludes-operations checks only the selection-change and CA-change rows. The executable-digest-only row and the pre-change-connection row are in its Outcome but not in the acceptance. Under my lane that is ambition, not a defect, so it does not set the verdict.
- Out of my lane (scope, design): story:invoke-definition-reuse-credential has every scope entry marked "inferred", and the story itself says the scoper should confirm them.

```findings
- file: .engineering/planning/story/invoke-definition-file.md
  line: 99
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says the first run 'answers the one token prompt', but the Outcome's Authentication bullet (line 86-87) has the command ask a connect yes/no question before the token prompt, so the acceptance must name the question and its yes answer, or the observed run and the stated outcome disagree"
```
