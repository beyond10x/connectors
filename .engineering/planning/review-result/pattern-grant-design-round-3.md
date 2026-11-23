---
format: aep.planning-md/3
id: review-result:pattern-grant-design-round-3
kind: review-result
status: active
title: Design critic, round 3, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


**Round-2 findings**
1. Worst case: resolved. The host sets `size_limit` and `dfa_size_limit` to 64 KiB, and the ceiling is stated as 64 times the two limits (Design point 3, line 120). Acceptance 25 now runs 64 patterns that compile to just under the limit. One thing I could not confirm is how the `regex` crate behaves with a 64 KiB lazy-DFA limit. I expect it to fall back to a slower engine rather than refuse to build. The story already leaves the figure to be measured before the contract text is written.
2. Catalog: resolved by the alternative I offered. The host keeps a stated copy, the catalog source is untouched, and one vector file is read by a test in each crate (acceptance 26, `whole_value_patterns.rs`).
3. Contract texts and the cap: resolved. Acceptance 21 puts the three bounds in `C`, `K` and the model, and Design point 3 counts the 64 after expired grants are dropped.
4. Both halves are resolved. The 0.77.0 order of checks stays (Design point 7), and acceptance 10 now says "on a grant with `input_keys`".

**New finding**
Design point 7 and the acceptance preamble put statements 6 to 10 on "the request check that 0.77.0 splits from `grant`". The 0.77.0 story describes no such split. Its `grant` takes a path, the state directory, a signer, the instance and a clock, and it is tested in-process with no keyring (`local-invoke…md:371-372`). Its own validation test, `a_grant_input_is_validated`, runs on `grant` (`:452`). Statement 9 also needs the file, to count 64 or 65 pattern members. A document-only check cannot count that, and the 256-grant bound it is modelled on is a `grant` refusal that reads the file (`:238`). The fix is to name 0.77.0's `grant` and its test as the surface for 6 to 10, as the 0.77.0 story does, and to drop "request check" from lines 138-139 and from the acceptance preamble. Statement 9 then runs against a file in a temporary state directory.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 137
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design point 7 and the acceptance preamble put statements 6 to 10 on 'the request check that 0.77.0 splits from grant', which the 0.77.0 story does not describe: its grant takes the file, a signer, the instance and a clock and is tested in-process with no keyring (.engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md:371-372), and its validation test runs on grant (:452); statement 9 counts members of the file, which a document-only check cannot, as the 256-grant bound is a grant refusal that reads the file (:238); name 0.77.0's grant and its test as the surface of 6 to 10, run 9 against a file in a temporary state directory, and drop 'request check'"
```
