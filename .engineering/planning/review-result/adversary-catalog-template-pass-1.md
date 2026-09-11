---
format: aep.planning-md/1
id: review-result:adversary-catalog-template-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the catalog operation template
relations:
- reviews: story:catalog-operation-template
revision: 1
---
# Adversary pass 1 — story:catalog-operation-template

Worktree `wt-35ad348b4293`, detached HEAD `b447f4b` plus the unit's uncommitted work.

```
unit: 1
verdict: red
cases: executed 30→37, red 7
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: no
```

The pass added one new file, `crates/connectors-catalog/tests/template_adversarial.rs`, seven
cases, all red. It edited no implementation file and changed no existing case. Clippy stays clean
with the cases in the tree, so the suite is red on behaviour only. Four of the seven build their
operation from `inventory::extract` over a real `openapi: 3.1.0` document that produces no
`Unsupported` gap, so the state under test is one the crate's own ingest path emits.

## What it attacked and could not break

Query encoding (`&`, `=`, `#`, `+`, `%` escaped; `/:@` kept), path encoding of non-dot specials
including UTF-8, a placeholder twice in one path, a parameter named with regex metacharacters, an
operation with no parameters, and first-media-type selection. It also observed that the existing
byte-identical case cannot fail, because `BTreeMap` has one iteration order and the output order
comes from the operation — the property is real, the test does not test it.

## Findings

```findings
- file: crates/connectors-catalog/src/template.rs
  line: 221
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a path value of ".." passes the encoder untouched because "." is unreserved, so the bound path "/projects/../issues" resolves under RFC 3986 to "/issues" and the value moves the path the operation identifies.
- file: crates/connectors-catalog/src/template.rs
  line: 240
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a header value containing CRLF is carried verbatim into Binding.headers with no refusal, letting a caller value inject an additional header.
- file: crates/connectors-catalog/src/template.rs
  line: 230
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: an operation-level parameter that overrides a path-level one of the same name and location is emitted alongside it, producing the duplicated query pair "page=2&page=2".
- file: crates/connectors-catalog/src/template.rs
  line: 261
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: an operation-level override relaxing required from true to false is ignored, so a valid document refuses ValueAbsent for a parameter its own operation declares optional.
- file: crates/connectors-catalog/src/template.rs
  line: 261
  category: property
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the bind map is keyed by parameter name alone while OpenAPI identity is name plus location, so a path "id" and a query "id" collide and the path value populates a query parameter the caller never supplied.
- file: crates/connectors-catalog/src/template.rs
  line: 242
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a declared path parameter with no matching placeholder is accepted and its supplied value reaches no part of the binding, the silent drop the Scope's cookie sentence rules out.
- file: crates/connectors-catalog/src/template.rs
  line: 120
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an unterminated "{" refuses as PlaceholderUndeclared and names a parameter the operation does in fact declare, so the refusal's own reason is false.
```

Named fixes the pass proposed and did not apply: refuse or encode a path value that is `.` or
`..`; refuse a header value carrying `\r`, `\n` or a control byte; have `inventory::extract`
replace rather than append a path-level parameter an operation-level one overrides on
(name, location), or dedupe at template construction; key the bind map on (name, location), or
refuse when one name spans two locations; add the symmetric unplaced-path-parameter check with its
own refusal; and give the path scanner its own malformed-path refusal carrying the path.
