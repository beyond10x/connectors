---
format: aep.planning-md/1
id: review-result:adversary-mcp-domain-model-pass-2-20260912
kind: review-result
status: active
title: Adversary pass 2 — MCP domain model, wave 2
relations:
- reviews: story:mcp-domain-model
revision: 1
---
```
unit: mcp-w2 — story:mcp-domain-model at ff8ec40 (d5270ca + ff8ec40), worktree cv2-mcp-w1-20260912
verdict: red
cases: executed 36→39, red 3
origin: introduced 9, pre-existing 0, undecided 0
wrote-outside-worktree: 14 paths under ~/.cache/cv2-mcp-waves-20260912/adversary-w2-pass2/
needs-coordinator: yes
```

## 1. Diff

One untracked path, `crates/connectors-build/tests/mcp_domain_model_adversary_pass2.rs`, 433 lines. No implementation file touched, no existing case deleted, skipped, weakened or rewritten. The index was left alone because another agent holds this tree.

## 2. Cases added — three, all red

- `no_state_carrier_closes_a_set_the_pinned_schema_leaves_open`
- `a_census_rows_verdict_cannot_be_supplied_by_the_row_beside_it`
- `every_relation_no_source_answers_carries_the_marker_the_acceptance_names`

The marker-window case reads `MARKER_WINDOW` out of the census source rather than hard-coding it, so narrowing the constant is what turns it green, and if the constant is removed the case stands down.

Fix-survival measured, not asserted: a candidate correction in scratch turns all three green with their assertions untouched and validates at exit 0 under the pinned ESS.

## 3. Suite

Before 36, after 39, red 3, exit 101. The unit's three census cases and pass 1's three cases are green and undisputed. `cargo clippy -p connectors-build --all-targets -- -D warnings` exits 0, and the adversary proved the lint reaches its own target by appending a `needless_range_loop`, watching clippy reject it, and restoring before every recorded run.

## 4. Findings

**1. `state.yaml:191` and `:327` — the class was swept across documents and not across declarations.** `McpCapabilitySnapshot.server_capabilities` and `McpAdvertisedCapability.capability_kind` are typed to a seven-variant enum that this root's own `protocol.yaml:81-85` marks open, quoting the pinned schema verbatim: "Known capabilities are defined here, in this schema, but this is not a closed set: any server can define its own, additional capabilities." The struct is the discover response, whose capabilities object carries whatever the server chose, and the file admits the gap one line up — "an unknown advertised key has no carrier here". This is the identical defect the correction fixed for `selected_revision` one line above. Reaches: no code constructs it; `story:mcp-inbound-capability-projection` and `story:mcp-profile-selection-matrix` read exactly these two carriers. NEEDS-CHANGE, blocker, introduced.

**2. `mcp_domain_model_census.rs:275` — two lines is one too many.** Census row 4 is named at two sites and both sit inside a neighbouring row's marker, so its verdict can be struck at every site and the guard still passes, while the mutant validates at exit 0 because the strike is inside a YAML comment. Six sites have their verdict supplied by a neighbour. The guard's own doc comment says a marker in a heading above a list is not a marker for any of them; at two lines the footer list, whose rows are two lines apart, is exactly that list. A window of 1 catches it and leaves the current model passing. CONFIRMED, warning, introduced.

**3. `state.yaml:459` and `:461` — the acceptance sentence.** The acceptance requires every unreadable relation to be carried as an explicit `UNMAPPED:` comment. Eight of ten census rows are unreadable; six carry it, two carry `FILED:` plus a blocker id and carry `UNMAPPED` on no line that names them. The census guard encodes that second vocabulary, so the test was written to the implementation rather than to the sentence the story is accepted against, and nothing compares them. Reaches: the next story says it cites the `UNMAPPED:` marker this model carries, and grepping it returns six of eight open questions — the two omitted are both rows that matrix has. NEEDS-CHANGE, warning, introduced.

**4. `mcp_domain_model_census.rs:217` — a realisation form still invisible.** The check reads `entities[]` only and only the census row's source side. A carrier on the *value* side — a field on a `types[]` struct named for the source entity's own identity — realises an unmapped edge, validates and compiles at exit 0, and is invisible. Plural carrier names escape both arms too. The model's stated convention is itself one-directional, so this is a gap in the rule rather than a slip in the code. CONFIRMED, warning, introduced.

**5. `story:mcp-domain-model` revision 6 — the invariant the corrected cardinality rests on is stated too strongly.** Enumerating all 40 `via` relations in shared ESS: two carry `cardinality: many` through a scalar — `declarations.yaml:119-123` and `discovery_state.yaml:85-89` — both `kind: owns`, where `via` names the child's back-reference to the owner's identity. The model's own `state.yaml:289-291` scopes the rule to `references` relations, which makes it true; the story dropped that scope, and `state.yaml:302-304` repeats the unscoped form inside the model. CONFIRMED, note, introduced.

**6. `state.yaml:295` — a cited block does not say what the citation says.** "five `cardinality: one` relations, each `via` a scalar `String` field": the five are scalar, but three of the five `via` types are not `String` — one is `Optional<connectors.credentials.CredentialGenerationId>`. The rule the citation supports says scalar, so the conclusion survives; the evidence sentence does not. CONFIRMED, note, introduced.

**7. `design.md:62` — "holds the root to all of the above".** Of the three load-bearing properties, single-state lifecycles are enforced by ESS rather than by the census case, and "it selects nothing" is enforced by nothing: a new selected-transport enum validates and compiles at exit 0 without touching an assertion. CONFIRMED, note, introduced.

**8. `mcp_specification_pin_adversary_pass2.rs:174` — citation drift this unit created.** The correction inserted a 29-line section at `design.md:37`, moving the four-transport table from `:36-42` to `:68-76` and leaving a sibling unit's citation pointing into the new section. The case still passes because it matches text rather than lines. The third instance of the half-life this story documents. CONFIRMED, note, introduced.

**9. `design.md:88` — a directional reference that points the wrong way.** "see the authored native model below"; the section is above the bullet. The anchor resolves; the word does not. CONFIRMED, note, introduced.

## 5. Attacked and could not break

- **The class sweep re-derived with different terms.** Exactly three non-test documents outside the planning store and the evidence directories name this directory. Two are fixed; the third's sentence is genuinely scoped to itself, so the non-member classification is honest. No fourth member.
- **The withdrawn citation is genuinely withdrawn** — it appears only for ownership, and neither the footer's citation list nor the census assertion carries it.
- **The four carrier citations exist and bind as claimed**, modulo finding 6's wording.
- **Loosening the selected revision broke nothing.** No declaration, relation or field still references the enum; it stands as a protocol value with no carrier, which is what the change intended.
- **The rest of the owner document's new section is accurate**: the root name, its position among seven, four entities and five types exactly as tabulated, and the census arithmetic summing to ten.
- **The trap-string comment** closes pass 1's finding.
- **Boundary**: no shared file touched; every probe ran in scratch.

```findings
- file: adapters/mcp/spec/ess/domains/state.yaml
  line: 191
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The discovered capability set is typed to a closed seven-variant enum that this
    root's own protocol file marks as not a closed set, quoting the pinned schema's
    "any server can define its own, additional capabilities", so a discover snapshot
    cannot record a capability the specification permits - the same defect the
    correction fixed for the selected revision and did not sweep to the sibling
    carrier one line above it, or to the advertised capability kind.
- file: crates/connectors-build/tests/mcp_domain_model_census.rs
  line: 275
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    With a marker window of two the census footer's rows sit inside each other's
    markers, so a row can lose its verdict at every site in the file and the guard
    stays green while the mutant validates at exit 0; a window of one catches it and
    leaves the current model passing.
- file: adapters/mcp/spec/ess/domains/state.yaml
  line: 459
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The acceptance requires every relation no source answers to be carried as an
    explicit UNMAPPED comment, and the two filed rows carry only a blocker id on every
    line that names them, while the census guard encodes that second vocabulary rather
    than the sentence the story is accepted against - and the next story says it cites
    the UNMAPPED marker this model carries.
- file: crates/connectors-build/tests/mcp_domain_model_census.rs
  line: 217
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The repaired realisation check reads entities only and only the census row's source
    side, so a carrier on the value side named for the source entity's own identity
    realises an unmapped edge, validates and compiles at exit 0 and is invisible, as is
    any plural carrier name.
- file: .engineering/planning/story/mcp-domain-model.md
  line: 75
  category: property
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The corrected cardinality rationale states the carrier invariant without exception
    and without the references scope that makes it true; two owns relations in shared
    ESS carry cardinality many through a scalar via, one of them the very block the
    same paragraph withdraws.
- file: adapters/mcp/spec/ess/domains/state.yaml
  line: 295
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The claim that a cited block is five cardinality-one relations each via a scalar
    String field is wrong for three of the five, one of which is via an optional
    credential-generation id and is not a string at all.
- file: adapters/mcp/design.md
  line: 62
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The owner document says the census case holds the root to all three load-bearing
    properties; single-state lifecycles are enforced by ESS rather than by that case,
    and nothing at all enforces that the root selects nothing.
- file: crates/connectors-build/tests/mcp_specification_pin_adversary_pass2.rs
  line: 174
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Inserting a 29-line section into the owner document moved the four-transport table
    and left a sibling unit's citation pointing into the new section, the third
    instance of the citation half-life this story documents.
- file: adapters/mcp/design.md
  line: 88
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The bullet points a reader to the authored native model section below when that
    section is above it.
```
