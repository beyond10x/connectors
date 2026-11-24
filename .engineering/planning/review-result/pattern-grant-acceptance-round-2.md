---
format: aep.planning-md/3
id: review-result:pattern-grant-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


Round-1 findings, in the order I returned them:

1. Resolved. `result.grant` is gone, so the old blocker no longer applies. That left list-standing unchecked, which is new finding 2 below.
2. Resolved. Acceptances 6 to 8 are one family per statement. Design point 7 and acceptance 6 put the document check before the key, and the keyless CLI test covers it. Acceptance 11 states the served half as members refused in 6 to 10 making the file unavailable. Acceptance 8 includes `"value": null` beside a `pattern`.
3. Resolved on your reading. The witness pair is named by the committed test, so it is checkable afterwards. Acceptance 6 does not state the 1 MiB figure, which Design point 2 calls a proposal. That is accepted.
4. Resolved. Acceptances 1 to 3 give named witnesses: `DEV-12x`, `xDEV-12`, a trailing newline, and `^a$|^b$`.
5. Resolved by acceptance 25 (16 grants, 64 patterns, under 1 s).
6. Resolved. The adversary test is named `a_served_file_with_a_bad_member_is_unavailable` and tied to acceptance 11.
7. Resolved. Acceptance 13 holds the replay alone. Acceptance 12 compares `retry_after_seconds` with the exact-value grant's value in the same test and clock. That is checkable.
8. Resolved. Acceptances 15 and 16 are split, and the fixture is committed after 0.77.0 merges.
9. Resolved. Acceptance 5 lists the refused inputs.
10. Resolved. Acceptances 18 and 19 add the admitted consumer inputs, and `/body/update` is named.
11. Resolved. Acceptance 10 lists each overlap case. Its refusals fall under the `invalid_input` heading, and acceptance 11 covers the served side.
12. Resolved. Acceptances 17 and 20 to 24 cover the record, texts, generated files, test commands with the 120 s and 60 s bounds, and no `.py` file.
13. Resolved. There is no cache. The 64-pattern bound is checked by acceptances 9 and 25.

New findings:
- The matching preamble (lines 226-228) says each refusal has no provider request, no attempt recorded and no use counted, "in `covering`". `covering` is a pure function returning an `Uncovered` variant. It cannot show a request, an attempt or a use count, and `approval_required` is mapped above it. The fix is to name the variant for each case (`OtherTarget`, or `MissingMember` for an absent member). The no-request and no-use claim then moves to one control through `admit` and the served test.
- `list-standing` printing the three member forms (the old acceptance 9 half) now has no acceptance. Design point 6 still promises it, and `cli.yaml` is declared unchanged. Add a statement that `list-standing` prints a pattern member and a `one_of` member with their keys. Add a `git diff --name-only` check that `ess/domains/cli.yaml` and `apps/connectors/spec/cli.yaml` are unchanged.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 226
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the matching preamble says each refusal is approval_required with no provider request, no attempt and no use 'in covering', but covering is a pure function that returns an Uncovered variant and cannot show a request, an attempt or a use count; name the variant per statement (OtherTarget, or MissingMember for an absent member) and move the approval_required, no-request and no-use claim to one admit control and the served test"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 131
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design point 6 promises that list-standing shows what a grant binds and that cli.yaml stays unchanged, but no acceptance statement checks either since result.grant and the old acceptance 9 were dropped; add one statement that list-standing prints a pattern member and a one_of member with their keys (in-process), and one that git diff --name-only lists neither ess/domains/cli.yaml nor apps/connectors/spec/cli.yaml"
```
