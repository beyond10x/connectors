---
format: aep.planning-md/3
id: review-result:wave-20261106-u2-adversary
kind: review-result
status: active
title: Wave 2026-11-06 U2 adversary pass 1
relations:
- reviews: story:owner-bounded-read-realization-matches-projection
revision: 1
---
unit: U2 adversary
verdict: green
cases: executed 13→16, red 0
origin: introduced 1, pre-existing 1, undecided 1
wrote-outside-worktree: ~/.cache/connectors-wave-20261106/u2/adv/{case.log,suite.log,lint.log,commit-msg.EuA4Pd}
needs-coordinator: no

I found no defect in the change. The realization guard behaves as the acceptance says, and I have no red case to report.

Commit 9a73f986e "core: adversary cases for every inline_schema keyword and array nesting" (b10x-bot[bot]), one test file: crates/connectors-core/tests/inline_schema_adversary.rs (+55).

Cases added, all green (they cover mutants, they show no defect):
- every_listed_keyword_alone_makes_a_schema_relocated: each of the 7 keywords, alone, at the root and one object deep, is not inline.
- a_reference_inside_an_array_makes_a_schema_relocated: a reference inside allOf, prefixItems or anyOf/oneOf is not inline.
- keyword_names_as_values_and_plain_schemas_stay_inline: an enum of "$ref", plain schemas, true and {} stay inline.
- cargo.sh <tree> test --locked -p connectors-core --test inline_schema_adversary: test result: ok. 3 passed; 0 failed, EXIT=0.

Suite: cargo.sh <tree> test --locked -p connectors-host --lib -- owner::governed::tests::bounded: test result: ok. 13 passed; 0 failed; 0 ignored; 705 filtered out, EXIT=0. fmt check and clippy -D warnings on -p connectors-core passed.

Findings:
1. Mutant coverage (warning, pre-existing). crates/connectors-core/src/lib.rs:105-120: inline_schema now decides what the owner serves as well as what the projection advertises. No test used $recursiveRef or $recursiveAnchor, or a reference inside an array; $dynamicAnchor appeared only next to $dynamicRef (bounded_reads.rs:263, projection.rs:961). Four mutants would have left the suite green; the new cases cover them (shown by reading, not on a mutated copy).
2. Doc comment overstates (note, introduced). crates/connectors-core/src/lib.rs:103-104 says "the owner serves one, only over inline schemas"; the generic arm at bounded.rs:61 serves non-inline schemas (the known out-of-scope item). Fix: limit the sentence to an implemented read.
3. Projected read without a budget (note, undecided, out of scope). bounded.rs:12-26: a GovernedRead frame with a projection revision and no budget is checked only for effects, not realization. Reach: seems not to widen access, since a plain GovernedRead without a projection revision already reads any granted Read operation (governed.rs:140). Not tested.

Attacked and not broken: the profile guard; the input and output inline guards (input validation at governed.rs:148 runs before the guard, so the old code reached Ok); worker admission and final fence through resolve (supervisor.rs:1064, :1699); the binding the guard checks is the one the read runs under (governed.rs:126); existing implemented resource cases still green.

target/ 2.5G; about 0.5 cargo minutes, 12 wall minutes.


```findings
[
 {"file":"crates/connectors-core/src/lib.rs","line":105,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"U2-ADV-1: No test covered $recursiveRef, $recursiveAnchor, $dynamicAnchor alone, or a reference inside an array in inline_schema, which now gates the owner too; 9a73f986e adds green cases that catch those mutants (not shown on a mutated copy)"},
 {"file":"crates/connectors-core/src/lib.rs","line":103,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"U2-ADV-2: Doc comment says the owner serves a bounded read only over inline schemas; the kept generic arm (bounded.rs:61) serves non-inline ones"},
 {"file":"crates/connectors-host/src/local/owner/governed/bounded.rs","line":12,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"undecided","message":"U2-ADV-3: Out of scope and not tested: a projected GovernedRead without a budget is checked only for effects, not realization; nothing found reaches a wider read, since a plain GovernedRead already reads the operation"}
]
```

(Coordinator: home-directory paths in the header shortened to ~ for the repository's personal-path gate; nothing else changed.)
