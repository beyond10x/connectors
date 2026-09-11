---
format: aep.planning-md/1
id: review-result:adversary-catalog-authored-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the locally authored TOML action
relations:
- reviews: story:catalog-local-toml-action
revision: 1
---
# Adversary pass 1 — story:catalog-local-toml-action

Worktree `wt-35ad348b4293` at `2c66010` plus the unit's uncommitted work.

```
unit: 1
verdict: red
cases: executed 74→78, red 4
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: 9 paths under the assigned scratch root
needs-coordinator: no
```

One new file, `crates/connectors-catalog/tests/authored_adversarial.rs`, four cases, all red and all
written so that refusing the document is also a passing outcome. Three further findings carry no
case: they were established by mutating a copy of the crate in scratch and observing the suite stay
green.

## The load-bearing claim held

Every accepted action goes through `Template::from_operation` and nothing else builds a template; a
template refusal is returned unrewritten with its subject, reason and error code delegated. The pass
could not construct a document producing a template the imported path would refuse. Method
normalisation cannot yield a method outside the known set; unknown fields are refused at every table
level including inline spellings; the size bound precedes UTF-8 decoding and parsing; duplicate
action names are refused.

## Findings

```findings
- file: crates/connectors-catalog/src/authored.rs
  line: 281
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a document repeating one parameter under the same name and location is accepted, its explicit `required = true` is discarded by the template's merge, and the Action's `operation` and `template` fields then disagree about whether a value is needed.
- file: crates/connectors-catalog/src/authored.rs
  line: 288
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: authored request media types keep document order while imported ones are sorted by serde_json's map, so an action offering two of them does not bind as the equivalent OpenAPI operation the Acceptance requires it to match.
- file: crates/connectors-catalog/src/authored.rs
  line: 337
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: read_file discards the io::ErrorKind, so a directory or a permission-denied file is reported as ErrorCode::NotFound and no caller can tell it apart from an absent document.
- file: crates/connectors-catalog/src/authored.rs
  line: 229
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the local copy of the four location spellings agrees with inventory::Location::parse today but its `header` arm is covered by no case, and a scratch mutation sending `header` to Location::Query leaves the whole suite green.
- file: crates/connectors-catalog/src/authored.rs
  line: 315
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "an empty `action` array is refused as FieldAbsent, whose reason claims the document must declare a field it did declare."
- file: crates/connectors-catalog/src/authored.rs
  line: 303
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no case reads a document of exactly DOCUMENT_LIMIT bytes, so changing the strict bound to an inclusive one leaves the suite green."
- file: crates/connectors-catalog/src/authored.rs
  line: 145
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Document::find is public and has no case at all; replacing its body with None leaves the suite green.
```
