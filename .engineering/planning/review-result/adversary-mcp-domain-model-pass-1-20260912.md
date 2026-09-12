---
format: aep.planning-md/1
id: review-result:adversary-mcp-domain-model-pass-1-20260912
kind: review-result
status: active
title: Adversary pass 1 — MCP domain model, wave 2
relations:
- reviews: story:mcp-domain-model
revision: 1
---
```
unit: mcp-w2 — story:mcp-domain-model at d5270ca, worktree cv2-mcp-w1-20260912
verdict: red
cases: executed 33→36, red 3
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 9 paths under ~/.cache/cv2-mcp-waves-20260912/adversary-w2/
needs-coordinator: yes
```

## 1. Diff

One path, `crates/connectors-build/tests/mcp_domain_model_adversary.rs`, 304 lines. No implementation file touched.

## 2. Cases added — all three red

`a_binding_can_select_every_version_a_server_can_report_as_supported`, `the_adapter_owner_document_does_not_deny_the_model_this_unit_added`, and `the_one_stated_census_edge_agrees_with_the_field_that_realises_it`.

Each honours fix-survival and it was measured rather than reasoned: `MCP_ADVERSARY_ROOT` points the same three cases at copies carrying candidate corrections, and all three go green under two independent correction variants.

## 3. Suite

Before is 33, from a run with the three deselected by name. After is 36, red 3, exit 101. The unit's own named checks stay green and are not disputed: `ess specify validate` → `connectors_mcp v1 — 3 file(s), valid`; `ess specify compile` → `22 declaration(s)`; `ess-boundary` → 7 adapter models, exit 0.

## 4. Findings

1. `adapters/mcp/design.md:3,:55,:89,:92` — the adapter owner document states in four places that this directory has no ESS model: the Status line, "No entity or relation is declared by this directory", "`spec/ess/` … do not exist here and are not implied", and "the authored native model" listed under "Remaining obligations, all unstarted". The commit created that root with 4 entities and 20 declarations and left all four sentences untouched; `git diff d5270ca^ d5270ca -- adapters/mcp/design.md` is empty. Reaches: `docs/design.md:279` gives this document its role, the wave brief calls it "the owner document, which records what you owe", and the ten remaining stories of the epic read it to learn what exists. A reader who believes it authors the native model a second time. CONFIRMED, introduced.

2. `state.yaml:254` and `:371` — the one census edge the model states rather than marks is stated `MANY`, and the only field realising it is the scalar `operation_ref: String`. This repository binds the two without exception: `many` carries a `List<…>` via field, `one` a scalar, across four cited sibling documents. The source the story cites for it declares `AdapterSpecification owns many OperationDeclaration` — a **different pair** — so it answers this edge neither way, and the cardinality is chosen rather than read. The census case cannot see it: its `Stated` arm is empty. Reaches: the story lists this as the one relation the model may state, and later contracts read this file row by row; a downstream story reading `MANY` models a list carrier and gets the projection backwards. NEEDS-CHANGE, introduced.

3. `state.yaml:243` — `McpServerBinding.selected_revision` is the closed two-variant `Optional<ProtocolRevision>` while the list it is selected from, `McpCapabilitySnapshot.supported_versions`, is `List<String>` for the reason the model itself states: a server may name versions neither pinned revision covers. `protocol.yaml:36-39` declares the revision set open and `:18-20` says no selection is made in this file. So a version can be recorded as supported and not as selected — `2025-06-18` and `2024-11-05` are reportable and unselectable, and `http-sse-2024-11-05` is a declared transport variant whose revision is one of them. Typing the selection to a closed set **is** the selection this file says it does not make. Reaches: the next story owns a row per transport binding; no code constructs this entity today, so the reader is that story. CONFIRMED, introduced.

4. `mcp_domain_model_census.rs:115-139` — `declared_relations()` walks only `entities[].relations[]`, so the flat-carrier realisation the model itself names as the risk at `state.yaml:316-317` — "a `binding_ref: String` would read as a settled one-to-one" — is invisible to it. Adding exactly that field on a scratch copy validates at exit 0 and produces no `relations:` entry for the census to find. The `UNMAPPED:` assertion at `:194` is file-global rather than bound to the edge under test, so it holds while any one marker survives anywhere. Reaches: a dead guard on the test the whole unit rests on. Nothing trips it today; the exposure is the next editor. CONFIRMED, introduced.

5. `mcp_domain_model_census.rs:86-98` — the `TRAP_STRINGS` doc comment says the pin record "names all three" and the list carries two. Omitting `2024-11-05` is correct, because the legitimate `http-sse-2024-11-05` transport variant carries it and including it would make the case unfixable — but the comment claims a coverage the list does not have. NEEDS-CHANGE, introduced.

## 5. Attacked and could not break

- All 16 cited archives exist; all 40 fully-qualified file-and-line references fall inside the uncompressed line count; about 15 read verbatim, every one says what the model says.
- All three ESS refusals behind the no-lifecycle and no-relations decisions reproduced against scratch copies. The decisions follow from real refusals, not convenience.
- The three credential kinds: three distinct declared types, no alias, no shared struct, no field through which two collapse.
- The four transport bindings match the design document and the archives exactly, and no enum in the model amounts to a selection.
- The version strings: the revision enum carries the two the **primary** revision states, and none of the three documented traps appears as a variant.
- The capability snapshot as a value: the cited lines do say cacheable-with-a-TTL rather than durable.
- The boundary: 7 adapter roots validate and compile independently, no shared file touched, `mcp` appears nowhere in shared ESS, no exception added.
- The declaration count is ESS's own.
- The new root's absence from the website selection matches `gitlab`, which also has a root and is also absent.

```findings
- file: adapters/mcp/design.md
  line: 3
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The adapter owner document denies the model this unit added in four places while
    the root declares four entities and twenty declarations, and the commit does not
    touch the document at all.
- file: adapters/mcp/spec/ess/domains/state.yaml
  line: 254
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The one census edge the model states rather than marks is stated many and realised
    by a scalar carrier, which is cardinality one everywhere else in this repository;
    the cited source declares a different pair and does not answer this edge, so the
    cardinality is chosen rather than read, and the census case asserts nothing about it.
- file: adapters/mcp/spec/ess/domains/state.yaml
  line: 243
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The selected revision is a closed two-variant enum while the list it is selected
    from is a list of strings, precisely because a server may name versions neither
    pinned revision covers, so two reportable versions are unrecordable as selected -
    and typing the selection to a closed set is itself the selection this file says it
    does not make.
- file: crates/connectors-build/tests/mcp_domain_model_census.rs
  line: 115
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The census walks only declared relations, so the flat-carrier realisation the model
    itself names as the risk is invisible to it and validates at exit 0, as measured on
    a scratch copy; the marker assertion is file-global rather than bound to the edge
    under test.
- file: crates/connectors-build/tests/mcp_domain_model_census.rs
  line: 87
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The trap-string doc comment claims the pin record names all three and the list
    carries two; the omission is correct but the comment claims a coverage the list does
    not have.
```
