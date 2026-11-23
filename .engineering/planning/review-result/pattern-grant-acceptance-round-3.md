---
format: aep.planning-md/3
id: review-result:pattern-grant-acceptance-round-3
kind: review-result
status: active
title: Acceptance critic, round 3, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: approve


Round-2 findings:

1. Resolved. The matching preamble now names `OtherTarget` and `MissingMember`. Statement 5a puts the caller-visible claim in two places. Locally, `admit` returns `approval_required` and the use store holds no row. On the served host, the write gets the code that `a_write_to_another_target_is_refused_with_no_request` asserts, and the provider double sees no request. Tests are named for both halves.
2. Resolved. Statement 10a checks that `list` returns the `pattern` and `one_of` keys as written, with test `list_shows_the_three_member_forms`. Statement 21a checks that neither `cli.yaml` changed.

The reversal of Design point 7, the shared vectors (26), the 64 KiB limit (25) and the three bounds (21) are all checkable. That includes test 24's command for the `whole_value_patterns` binary. The one gap I found is below. It is small and does not stop the story reaching the operator.

New finding:
- Statement 9 (65 pattern members refused, 64 accepted) is listed under `a_grant_document_with_a_bad_member_is_refused` "on the request check". That check sees only the one document. The count is a property of the file after expired grants are dropped (Design point 3), so it can only be checked through `grant` against a file already holding patterns. The expiry-drop case in Design point 3 has no statement either. This is a warning only.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 35
  category: acceptance
  severity: warning
  verdict: approve
  origin: introduced
  message: "statement 9 (65 pattern members refused, 64 accepted) is tested on the request check, which sees one document and not the file, but the count is the file's after expired grants are dropped (Design point 3); move 9 to the in-process grant call against a file already holding 64 patterns, and add the case where expired grants holding patterns are dropped first so the 65th is admitted"
```
