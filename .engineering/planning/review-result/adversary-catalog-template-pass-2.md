---
format: aep.planning-md/1
id: review-result:adversary-catalog-template-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the catalog operation template
relations:
- reviews: story:catalog-operation-template
revision: 1
---
# Adversary pass 2 — story:catalog-operation-template

Worktree `wt-35ad348b4293`, detached HEAD `b447f4b` plus the unit's uncommitted work. Pass 1's
seven cases pass unmodified; this pass attacked the corrections rather than re-verifying them.

```
unit: 1
verdict: red
cases: executed 47→51, red 4
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: no
```

One new file, `crates/connectors-catalog/tests/template_adversarial_two.rs`, four cases, all red.
No implementation file edited, no existing case changed. Clippy and fmt stay green with the cases
in the tree, so the suite is red on behaviour only. Every case builds its operation through
`inventory::extract` over an `openapi: 3.1.0` document that yields no `Unsupported` gap.

## What it attacked and could not break

The dot-segment claim held against 23 hostile path values, including `%2e%2e`, `..%00`, `..\`,
fullwidth `．．` and a BOM prefix: each was refused or held its segment under RFC 3986. Two
placeholders in one segment and a literal dot adjacent to a placeholder assemble into nothing. Three
locations sharing one name behave correctly when fully qualified. Key order does not change output.
Header value boundaries match the stated decision. `MediaTypeUnsafe` is unreachable from `bind` but
correct.

## Findings

```findings
- file: crates/connectors-catalog/src/template.rs
  line: 417
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: when one name is declared in two locations and the higher-ranked twin is required, ValueAbsent names the bare key the caller already supplied, so the refusal is false and no re-supply of the key it names can satisfy it.
- file: crates/connectors-catalog/src/template.rs
  line: 429
  category: property
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the location:name key is an unescaped string prefix, so a parameter literally named query:trace collides with the qualified spelling of a parameter named trace and one supplied value populates both, breaking the invariant bind's own documentation states.
- file: crates/connectors-catalog/src/template.rs
  line: 332
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the undeclared-key guard accepts a key if any parameter answers to it while the precedence rule may make that key unreadable, so a value supplied under an accepted key reaches nothing and nothing is refused.
- file: crates/connectors-catalog/src/template.rs
  line: 151
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: header_safe checks only control bytes and DEL, so document-declared header names that are not RFC 9110 tokens - holding a colon, a space, a comma, an at-sign, or empty - are accepted into the binding.
- file: crates/connectors-catalog/src/template.rs
  line: 290
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a document-declared path containing dot segments binds to a path that resolves elsewhere, and the bundle a document arrives in is third-party data whose digest vouches only for the file, not its author; stated as residue because no case was written against a deliberate decision and no untrusted-bundle caller exists yet.
```

Named fixes the pass proposed and did not apply: carry the key the caller must actually supply into
the absent-value refusal; replace the `location:name` string prefix with a key type that cannot
collide, or refuse a parameter name that begins with a location prefix; make the undeclared-key
guard check reachability rather than declaredness; and test a header name against `tchar` rather
than only control bytes.
