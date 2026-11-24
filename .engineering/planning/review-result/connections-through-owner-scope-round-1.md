---
format: aep.planning-md/3
id: review-result:connections-through-owner-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: needs-revision

Findings:

1. story:connections-list-answers-through-the-owner — Outcome promises process behaviour only ("makes no fresh open and no complete read") and says nowhere that the consumer's 35 s may not fall. The Source gives the 35 s and the consumer's 650 MB store, and "What the tree holds today" admits the owner's `Registry::list` "re-opens SQLite under `metadata.lock`, takes the pooled handle, catches up and builds the projection" with "Its time on a grown store is not measured". Add one sentence to Outcome or Out of scope: this story removes the CLI's own open and read; it does not promise the consumer's time falls, and the owner-side time is unmeasured. — `.engineering/planning/story/connections-list-answers-through-the-owner.md:41-46,86-88,162`

2. story:connections-list-answers-through-the-owner — Acceptance 6 gates closure on speed ("not at most half … the story is not closed on it"), but Out of scope line 162 says "lowering it is not this story". If the owner is slow, the story can never close and no item owns the fix. The measurement is also taken on "at least 10 MB", while the consumer's store is 133 MB to 650 MB, so a pass says nothing about the consumer's case. Resolve one of two ways. Either name the follow-up (a story or a decision for Timo) that takes the owner-side time if the half-bar fails, and measure at a size near the consumer's. Or make line 6 a report with no closing condition, and say so in Outcome. — `.engineering/planning/story/connections-list-answers-through-the-owner.md:131-134,162-163`

3. story:connections-list-answers-through-the-owner — The Source quotes only the measurement of `connections list` and never the wording of wanted item 3 itself. The claim for `describe` and `status` (including `status --acquisition`, and `describe` without `--connection`) therefore cannot be traced to a sentence of the ask. Only the contract line at `semantics.md:1378-1380` is quoted. Quote item 3's sentence in Source, and drop the `describe` and `status` arms, or give them a stated reason, if item 3 asks only for `list`. I could not read the issue body: no network access, and `gh issue view` ran once before I noticed the rule. — `.engineering/planning/story/connections-list-answers-through-the-owner.md:48-57,105`

4. story:connections-list-answers-through-the-owner — The Design's refusal path was not asked for and contradicts the contract paragraph. Design says "A refusal is an answered exchange: the CLI does not fall back". `owner.md` says the opposite for `cached` and `connections`: "an owner's refusal but `description_unavailable` included, is answered by that direct read, so a refusal carries the direct read's own code". Acceptance 3 and the L01 rule add work, and acceptance 7 does not list this change to `owner.md`. The simpler reading of the parent ask is to fall back on a refusal, which costs a direct open on the error path only. Drop the refusal path and acceptance 3, or state the reason it is needed and add the `owner.md` correction to acceptance 7. — `.engineering/planning/story/connections-list-answers-through-the-owner.md:105-109,126-129,135-138`; `contracts/cli/v1alpha1/owner.md:254-257`

5. story:connections-list-answers-through-the-owner — The ESS modelling of the request (Design line 111, acceptance 8) is traceable to no sentence of item 3 or the epic, and nothing makes it a condition of the change. The only stated reason is "so `owner.md` names a type that exists". The precedent c58fdb5f82f was a separate commit that modelled a request `owner.md` already named, referring to story:cold-owner-start-verifies-once. It brings in 8 of the 14 scope entries, three metadata-emissions pool jobs at `slot --timeout 30m`, a regeneration clash with story:entity-runtime-0330-records-name-definition-by-digest on `inputs.json` and `entity-runtime-0330` definitions, and shared edits with "ten other drafts" on `ess/domains/cli.yaml`. Either name the check that fails without the ESS type, or move the modelling to its own follow-up story and have `owner.md` describe the request without naming an ESS type. — `.engineering/planning/story/connections-list-answers-through-the-owner.md:111-113,139-145,146-152`

6. story:connections-list-answers-through-the-owner — The parent never mentions issue #267. The epic's Source is "the connectors review of 2026-11-23", and its Stories section says "One story per finding (11), plus the timing test", but this story and its three siblings take their promise from #267 (2026-11-16, 0.73.0). The decomposition edge does not show that item 3 is one of the 11 findings. Either the epic names #267 as a finding, or the story should cite the review finding it decomposes. — `.engineering/planning/epic/review-20261123-findings.md:17-23`; story Source at `.engineering/planning/story/connections-list-answers-through-the-owner.md:50`

7. story:connections-list-answers-through-the-owner — The other commands that open the store in the CLI process are listed under Out of scope with no reason and no owner. The `connections::select` path that `revalidate`, `revoke`, `launch` and `token` share does an owner-free `Registry::held` open, and `setup check`, the `definition` commands and `endpoint` open the store too. No sibling story claims them. The consumer's issue title says "CLI-side commands replay all of it", and siblings 1 and 2 shrink the store without claiming command routing. A dropped promise is only honest if the story says why. Add the reason per group (for example, the write commands need the CLI-side custody judgement) or name the story or question that carries them. — `.engineering/planning/story/connections-list-answers-through-the-owner.md:164-165,66-68`

What I read: 1 story, its parent epic, the vision (first 60 lines), the 3 sibling stories, the CLI graph filtered for owner and #267, `invoke-admission-queues-under-load.md` (lines 75-100, 222-240), `owner.md:224-262`, `semantics.md:1375-1385` and `git show --stat c58fdb5f82f`. Commands: `aep plan artifact show` ×5, `aep plan artifact graph`, `git` and `grep`. Counted from the epic and the siblings' out-of-scope lines, the parent and its siblings make 4 promises about this story: list through the owner, no store replay in the CLI, the commands beside it, and the time. Traced to the story: list through the owner and no store replay (outcome, acceptance 1-2); the time only in part (findings 1-2); the rest unclaimed (findings 3, 7). The epic itself states 11 findings and I could not trace this story to one (finding 6).

Could not establish:
- The wording of issue #267 wanted item 3, since there was no network. All judgements about the three commands rest on the story's own quotation.
- Whether `describe` without `--connection` or `status --acquisition` is something the consumer uses. Nothing local says it.
- Out of my lane: the checkability of the "at most half" threshold in line 6, and whether the CLI-judged custody sent in the request holds up (design). Neither sets the verdict.

```findings
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 41
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Outcome promises only no fresh open and no complete read; the story never says the consumer's 35 s may not fall, though the owner's own transaction time on a large store is unmeasured (line 88)"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 131
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 6 gates closure on a half-time bar, measured on 10 MB not the consumer's 133 to 650 MB, while Out of scope line 162 says lowering the owner's time is not this story, so a failed bar has no owner"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 48
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Source quotes only the measurement of connections list, not the wording of wanted item 3, so the claims for describe and status (acquisition included) cannot be traced to a sentence of the ask"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 105
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The refusal-as-answered path was not asked for and contradicts owner.md:254-257 (an owner's refusal is answered by the direct read); drop it with acceptance 3, or justify it and add the owner.md correction to acceptance 7"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 111
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "ESS modelling of the request (design line 111, acceptance 8) traces to no sentence of item 3 or the epic and nothing makes it a condition; the precedent c58fdb5f82f was a separate commit; move it to its own story or name the check that fails without it"
- file: .engineering/planning/epic/review-20261123-findings.md
  line: 17
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The epic's Source is the 2026-11-23 review and never names issue #267, so the story's decomposition edge does not show that item 3 is one of the 11 findings"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 164
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Out of scope lists setup check, revalidate, revoke, launch, token, definition commands and endpoint with no reason and no story or question that carries them, and no sibling claims them"
```
