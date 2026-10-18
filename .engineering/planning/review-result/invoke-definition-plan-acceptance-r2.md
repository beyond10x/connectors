---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-acceptance-r2
kind: review-result
status: active
title: Plan critic acceptance, round 2, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
needs-revision

story:connection-binding-excludes-operations — the acceptance is one sentence that joins three independent outcomes: selection-only change stays `ready`, a CA change stays `pending`/`stale`, and an executable-only replacement stays `ready` after one automatic validation. Any of the three can pass while the others fail, so the story is neither done nor not done. Keep one outcome in the acceptance (the selection-only change) and move the other two to the Outcome table as separately named checks, or split them out. Quoted text: "while the same test with a changed CA file still lists `pending`/`stale = true` … and the same test with only the provider executable replaced … lists `ready`" — .engineering/planning/story/connection-binding-excludes-operations.md:110-116

story:invoke-definition-reuse-credential — the acceptance joins two independent outcomes with a semicolon: the "reuse" success path (result, new connection, source unchanged) and the different-identity path (publishes nothing, offers connect fresh). One can pass while the other fails, so the acceptance should state a single outcome. Quoted text: "the same with a provider that answers a different identity publishes nothing and offers connect fresh" — .engineering/planning/story/invoke-definition-reuse-credential.md:86-90

**What I read:** 3 of 3 ids (story:connection-binding-excludes-operations, story:invoke-definition-file, story:invoke-definition-reuse-credential). I ran `aep plan artifact show <id>` for each with the pinned 0.71.3 binary, and read `critic-rubric.md`, `plan-critic-acceptance.md` and `review-result:invoke-definition-plan-acceptance-r1`. I ran `aep plan artifact kinds` and checked the acceptance line numbers in the store files.

**Round 1 finding:** it is fixed. The acceptance of story:invoke-definition-file now says the test "answers 'yes' to the one connect question and enters the token at the token prompt that follows" (`invoke-definition-file.md:105-110`). That matches the Authentication bullet in its Outcome. Each story also names a before and after state, so I raise no transition defect.

**What I could not establish:**
- Whether the multi-outcome sentences count as one statement because each is a single test scenario. Story:invoke-definition-file has the same shape (three sequential runs plus the byte-identical config check). I did not flag it, because it is one continuous journey in one test. The two findings above are separate scenarios with opposite results.
- Unease only, not findings: the Outcome rows for executable identity mismatch (`identity_mismatch`) and for a connection saved before the change are not in story:connection-binding-excludes-operations's acceptance. Story:invoke-definition-reuse-credential's acceptance also leaves out the authority-change and revoke-old-connection case. That is a question of ambition, not a defect.
- Out of my lane: story:invoke-definition-reuse-credential has every scope entry marked "inferred". The story says a scoper should confirm them.

```findings
- file: .engineering/planning/story/connection-binding-excludes-operations.md
  line: 110
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is one sentence joining three independent outcomes (selection-only change stays ready, a CA change stays pending/stale, an executable-only replacement stays ready after one validation), so one can pass while the others fail; it should state one outcome and move the rest to the Outcome table or a split story"
- file: .engineering/planning/story/invoke-definition-reuse-credential.md
  line: 86
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins the reuse success path and the different-identity path ('the same with a provider that answers a different identity publishes nothing and offers connect fresh') as two independent outcomes, so one can pass while the other fails; it should state a single outcome"
```
