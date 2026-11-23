---
format: aep.planning-md/3
id: review-result:pattern-grant-design-round-4
kind: review-result
status: active
title: Design critic, round 4, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: approve


My round-3 finding is resolved.
- Design point 7 and the acceptance preamble (story lines 139 and 259-261) now name 0.77.0's in-process `grant` with a test signer and a temporary state directory. "Request check" no longer appears in the story.
- Statement 9 now runs against a file and covers both sides of the cap. A file at 63 pattern members accepts a grant with one and refuses a grant with two. A file at 64 with 4 in an expired grant accepts a grant with 4, and the expired grant is gone.
- The catalog test change is sound. `crates/connectors-catalog/tests/main.rs` exists as that crate's one test binary. `inventory` is a public module (`lib.rs:14`).

I have no new findings.

```findings
[]
```
