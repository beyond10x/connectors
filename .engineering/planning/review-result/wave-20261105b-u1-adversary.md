---
format: aep.planning-md/3
id: review-result:wave-20261105b-u1-adversary
kind: review-result
status: active
title: Adversary pass on the mutating tool call revision rule
relations:
- reviews: story:mcp-mutating-tool-call-revision-rule
revision: 1
---
**Verdict: red. 1 blocker, 1 warning, 3 notes. All are taken into one correction in the unit, or decided, before the merge. Executed 6 to 8, red 1.**

The adversary ran on `be6f693e4`, wave 2026-11-05b unit U1, and added two cases to `apps/connectors/tests/local_mcp_server_write_refusal.rs`:
- `adv_u1_revision_shapes_on_an_unlisted_read_and_write` drives each revision shape through `tools/call` on the V2 binding, for both an unlisted read and an unlisted write.
- `adv_u1_legacy_call_without_meta_on_an_unlisted_write_is_invalid_params` covers the 2025-11-25 lifecycle with no `_meta`.

Its mutations ran on a scratch copy, whose target was cleaned with `cargo clean`.

Findings:
- F1 (blocker, introduced, verified at runtime). An unlisted Write called by name without a revision gets `-32602` "Missing projection revision" instead of `not_found`. That contradicts the delivered story:mcp-write-operation-call-by-name-is-refused, whose test at `local_mcp_server_write_refusal.rs:642` went red: "FAILED. 7 passed; 1 failed". The directing session decided that visibility comes before the revision check; it is recorded on the story.
- F2 (warning, introduced, verified on the copy). Replacing the `reads_only` argument at `projection.rs:349` with `true` left every test that existed before the review green. Only the adversary's two cases catch it, and they stay in the unit.
- F3 (note, introduced). The `-32602` told a granted caller that an unlisted operation is mutating. Closed by the same decision.
- F4 (note, introduced, inferred). `Bootstrap::validate_for` (`runtime.rs:715-722`) refuses Unknown everywhere and Write off V2, so the Unknown arm is unreachable today. The coordinator keeps it, because it refuses rather than binds if a later bootstrap admits an unknown effect.
- F5 (note, pre-existing text). `docs/local-mcp-stdio.md:87` still binds an absent revision without qualification, and `projection-cases.json` has no mutating field. Taken into the correction.

Not broken:
- The classification: each operation has one requirement (`runtime.rs:803`, `:836-843`), and curation enforces `external_write` if and only if the requirement is `Write` (`operation_curation.rs:103`). So `reads_only`, the projection filter and `session.rs:162` agree on every bootstrap that can occur.
- No policy or scope refusal is answered `-32602`.
- The contract text, the case files and the runtime agree on every revision shape.
- Dropping the new row is caught by the required-id list (`mcp_inbound_mutation_replay.rs:531`).
- The connectors-build checkers cannot catch a runtime revert, because they check documents only.
