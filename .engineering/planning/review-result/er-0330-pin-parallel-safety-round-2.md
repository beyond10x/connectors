---
format: aep.planning-md/3
id: review-result:er-0330-pin-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: needs-revision

1. story:entity-runtime-0330-records-name-definition-by-digest — the scope list still holds two files the body says this story does not edit, and each puts a collision in the scope that the text denies.
   - `crates/connectors-host/src/local/metadata/er.rs` is listed as inferred. "Shares files with" says "`er.rs`, which this story does not edit", and `LEVEL` stays 9. `story:spec-models-operation-admission` lists the same file in its scope (spec-models-operation-admission.md:19), so `aep plan artifact waves` still reads this as an `er.rs` collision.
   - `crates/connectors-host/tests/metadata_store_previous_pin.rs` is listed as cited. The body says the story "reads that file", and `story:test-binary-per-package` moves it. A cited entry says the story edits it.
   Accepted change: remove both from `scope`. Acceptance 8 already covers `metadata_store_previous_pin.rs` as a test that must pass, so nothing is lost. If either may change, keep it as inferred and say "may change" in "Shares files with", with its owner story.
   Citation: `aep plan artifact show story:entity-runtime-0330-records-name-definition-by-digest`, scope lines 13 and 19. "Shares files with" says "`er.rs`, which this story does not edit" and "this story reads that file and adds its new test in the lib".

Round 1 findings that now hold:
- **Round 1 finding 1:** `story:spec-models-operation-admission` is named per file, with its digest inputs.
- **Finding 2:** `ess/**`, `Cargo.lock`, `docs/development.md` and `CHANGELOG.md` are listed per story id.
- **Finding 3:** the `depends_on` edges exist in `aep plan artifact graph`. They run read-cost to this story, settled-read to this story, and settled-read to read-cost.
- **Finding 4:** every file of acceptance 8 is in scope.
- **Finding 5:** the unassessed stories are listed. `story:invoke-admission-queues-under-load` is correctly absent, because 0.75.0 carries it.

Unease, no finding:
- Acceptance 5 cites `metadata_store_previous_pin.rs:103-111`. If `story:test-binary-per-package` lands first, those line numbers shift.
- `story:test-binary-per-package` is active, still has no scope, and is listed as unassessed.

```findings
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 13
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope still lists er.rs (inferred) and metadata_store_previous_pin.rs (cited) although the body says neither is edited; er.rs collides in waves with spec-models-operation-admission and the test file with test-binary-per-package; remove both or mark them may-change"
```
