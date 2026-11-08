---
format: aep.planning-md/3
id: review-result:wave-20261105b-u2-adversary
kind: review-result
status: active
title: Adversary pass on the owner's bounded-read effects recheck
relations:
- reviews: story:owner-bounded-read-rechecks-semantic-effects
revision: 1
---
**Verdict: red. 1 warning confirmed by a red case and fixed in the unit before the wave closes, 2 notes. Executed 5 to 8, red 1.**

The adversary ran on `7ba0e97cc`, wave 2026-11-05b unit U2. It added three cases to `crates/connectors-host/src/local/owner/governed/tests/bounded_reads.rs`. It ran its mutations on a scratch copy, not in the tree; `bounded.rs` in the tree stayed unchanged.

- `adversary_projected_governed_read_refuses_semantic_and_non_network_effects` was red: "semantic_effects: projected GovernedRead dispatched {...\"status\":\"success\"...} left: 1 right: 0".
- `adversary_bounded_owner_refuses_every_parseable_semantic_and_non_network_effect` is green. It covers billable, irreversible, `process` and `local_system`, and kills two mutants that survive the unit's own case.
- `adversary_effect_shapes_outside_the_typed_metadata_never_parse` is green. `Metadata::parse` refuses null, non-arrays, `""`, `NETWORK`, unknown strings, duplicates and missing fields.

Findings:
- F1 (warning, pre-existing, verified at runtime): a `GovernedRead` frame with a projection selected (`transport.rs:2826-2843`) carries budget `None`, so `bounded::resolve` returns at `bounded.rs:12-14` before the effects rule, and the supervisor checks (`supervisor.rs:1064`, `:1699`) get the same `None`. A same-uid caller of the owner socket, the story's threat, could swap frame kinds. `projected_read` promises the projection constraint (`transport.rs:675-676`). The coordinator decided to fix it in this unit; the fix and its outcome are recorded on this artifact.
- F2 (note, pre-existing, inferred from code): the plain `Invoke` frame runs any granted read without consulting metadata (`transport.rs:2666-2697`, `owner.rs:965-972`). The host configuration's grant is the boundary there. Filed as a draft.
- F3 (note, introduced, verified on the scratch copy): the unit's case tested one value per field, so mutants refusing only `process` or only `human_visible` survived it. The second adversary case kills both.

Not broken:
- `BoundedRead` without a budget: impossible, because the frame requires the budget (`owner.rs:1307`, `transport.rs:2861`).
- The field shapes: the owner and the projection agree on every shape, since both parse the typed metadata first.
- TOCTOU: every `resolve` re-reads the metadata and checks the projection revision, and the fingerprint is compared at `governed.rs:276`, `supervisor.rs:1064` and `:1699`.
