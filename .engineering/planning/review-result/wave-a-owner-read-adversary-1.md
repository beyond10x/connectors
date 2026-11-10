---
format: aep.planning-md/3
id: review-result:wave-a-owner-read-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on owner-read-answer-size-envelope (wave A)
relations:
- reviews: story:owner-read-answer-size-envelope
revision: 1
---
```
unit: story:owner-read-answer-size-envelope (2e0c234b2, worktree cz-owner-read plus 1 uncommitted test)
verdict: red
cases: executed 30→31, red 1
origin: introduced 0, pre-existing 1, undecided 0
wrote-outside-worktree: <scratch>/units/owner-read-answer-size-envelope/scratch/{case.rs,run-case.sh,run-suite.sh,run-a_read_answer_re_encoding_a_child_admitted_result_stays_within_the_read_bound.log,suite.log,docs.log}
needs-coordinator: yes
```

1. Diff: `crates/connectors-host/src/local/owner/supervisor/adversary_tests.rs | 38 +++` (test only, uncommitted).
2. Case `a_read_answer_re_encoding_a_child_admitted_result_stays_within_the_read_bound`: a result of exactly `RESULT_LIMIT` bytes in the child's spelling (300 × `1.0e+10` plus string padding), which the child admits byte for byte. RED alone: `a result the child admitted is refused at the owner: answer 8390481 bytes, bound 8389632`, `0 passed; 1 failed`. Cause: `read_answer` (`supervisor.rs:2125`) parses the result with `connectors_core::read_json`, which rewrites every non-integer number in shortest `f64` form (`1.0e+10`, 7 bytes, becomes `10000000000.0`, 13 bytes); growth depends on content, so a fixed 1024-byte allowance cannot cover it. Reached by a provider writing exponent-form numbers between 1e7 and 1e16 (Java `Double.toString`, Jackson). Pre-existing: the base refuses the same answer at `RESULT_LIMIT`. Acceptance ("A read result the child admits reaches the CLI") does not hold read literally. Fix named: put the child's bytes into the envelope unchanged (`RawValue`, or prefix + document + `}`), as the governed path keeps its bytes.
3. Suite: `connectors-host --lib -- local::owner::transport local::runtime::channel <case>`: 30 passed, 1 failed (exit 101). `connectors-docs generate --check`: current.
5. Attacked, not broken: envelope ids (128-byte, no escaping) top out near 437 bytes; client and owner both use `answer_limit(&Request::Invoke)`; governed and bounded reads keep their documented Capacity refusal (`governed.rs:362`); `private-adapter.md:54` and `semantics.md:1447` still hold; nothing else calls `Client::invoke`.

```findings
[{"file":"crates/connectors-host/src/local/owner/supervisor.rs","line":2125,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"read_answer rewrites a child-admitted result's non-integer numbers in shortest f64 form (1.0e+10 to 10000000000.0), so the answer grows with the result's contents past the fixed 1024-byte allowance and the owner refuses a RESULT_LIMIT result as Capacity (8390481 > 8389632)"}]
```
