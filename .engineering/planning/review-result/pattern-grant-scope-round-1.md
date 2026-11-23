---
format: aep.planning-md/3
id: review-result:pattern-grant-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


The story covers pattern, `one_of`, both placements and the consumer's two grants. It leaves three outcomes unclaimed, drafts one thing nobody asked for, and states one contract claim too narrowly. I extracted 14 promises from the pick, the record and the Consumer section. 11 trace to an Acceptance line. 3 trace only to Design or Files, or to nothing: the record revision, the bound comment prefix, and `not_after`.

**Gaps**
- **Record revision.** It appears only under Files (line 150) and in Decision (line 27). No Acceptance line checks that Decision point 1 now names `pattern` and `one_of`. The record's `decides:` relations (record lines 9-13) also don't list this story.
- **Comment prefix.** The Design example binds `^Release review of ` (line 127). Acceptance 7 (lines 189-191) never says a comment whose text lacks the prefix is refused. It never says an `issueIdOrKey` outside `^DEV-[0-9]+$` is refused. It never says a transition with `target_status` outside the set is refused, or that the consumer's own transition input is admitted.
- **`not_after` and "long-lived".** The pick asks for one long-lived grant with `not_after`. No Acceptance line or Out of scope line claims it. The consumer's example has `not_after_unix_ms: 0`, which 0.77.0 refuses (local story line 235). A bound on how far ahead `not_after` may lie is out of scope in 0.77.0 (local story line 646). A pattern grant makes that gap more costly, and this story says nothing about it.

**Reach beyond the parents**
- **`result.grant`.** Neither the pick, the record nor the Consumer section asks for it. 0.77.0 O6 settled that visibility comes from `list-standing` (local story lines 671-673). Design point 4 (lines 82-84), Acceptance 9 (line 195) and the `cli.yaml` Files line add a change to the result shape.
- **`input_keys` relaxation.** This one traces to the consumer's prefix, which sits in nested arrays, so it stays. It is not limited to pattern members, though. A 0.78.0 file of exact members only, with `input_keys` above a `target` pointer, is `Malformed` on 0.77.0. Lines 104-105 and Acceptance 10 (lines 197-199) say only that a file with pattern or `one_of` members is unreadable to 0.77.0.
- **`regex` in `connectors-host`.** I did not flag it. The pick's example is a regex, and the story cites the version already in the lockfile.

**Out of my lane**
- The placeholder `not_after_unix_ms: 0` in the grant examples (acceptance lane).
- Acceptance 1 and 2 name no placement, and the served side is a plain `covering` test (design lane).

What I read: the story, the record, the closes-the-input story, and the local story's design, Out of scope and O1-O7 and run-1 sections. I also read `standing.rs:175-200` in the tree and ran `rg` over the planning store. The `aep` CLI was not used.

Not established: whether the 0.77.0 contract already states a `not_after` bound. I did not read `contracts/cli/v1alpha1/semantics.md` beyond a `grep`.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 189
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The consumer's bound comment prefix is claimed only by the Design example; Acceptance 7 never says a comment whose text lacks 'Release review of ', an issueIdOrKey outside ^DEV-[0-9]+$, or a target_status outside the set is refused, nor that the consumer's transition input is admitted. Fix: add these cases to Acceptance 7 and to the_jira_grants_cover_the_consumers_inputs."
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 150
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The change of the accepted record is promised by the pick but claimed only under Files; no Acceptance line checks it. Fix: add an acceptance that Decision point 1 of architecture-decision-record:standing-scoped-approval names pattern and one_of and that the record gains a decides relation to this story."
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 132
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The pick's one long-lived grant with not_after is claimed by no Acceptance or Out of scope line; the example's not_after_unix_ms 0 is refused by 0.77.0, and a bound on how far ahead not_after may lie (out of scope in 0.77.0) is not mentioned. Fix: add an acceptance that a pattern grant refuses a matching write after not_after and after revoke-standing, and list the not_after bound in Out of scope."
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 82
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "result.grant (StandingGrantResult gains grant, Acceptance 9 first half, the cli.yaml Files line) is in no parent; 0.77.0 O6 settled that list-standing shows what is granted. Fix: drop it, or put it to the operator as a lettered question with a default and keep it out of the story until answered."
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 104
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The claim that a file of exact members only still reads on 0.77.0 is false when input_keys lies above a target pointer (the relaxation applies to every member form, and 0.77.0 reads that file as Malformed); Acceptance 10 names only pattern and one_of files. Fix: state the relaxation as a second reason a 0.78.0 file is unreadable to 0.77.0 in lines 104-105 and Acceptance 10, or limit the relaxation to pattern and one_of members."
```
