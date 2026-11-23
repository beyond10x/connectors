---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
verdict: needs-revision

1. Unnamed collisions on the same files. The checkout's modified story file is the only body that mentions this story. No other body or edge does, and it has no `depends_on`. Bodies that cite a shared surface:
   - story:missing-state-database-says-initialize cites `apps/connectors/src/local/invoke.rs#L1036`, the function holding `READ_DEADLINE` (now line 1029). It also cites `semantics.md#L362`. Cited.
   - story:setup-check-names-missing-provider-with-instances cites `apps/connectors/src/local.rs#L743` and `semantics.md#L397`. Cited.
   - story:definitions-install-requires-a-valid-configuration cites `apps/connectors/src/local.rs#L649` and `semantics.md#L1112`. Cited.
   - story:docs-generated-reference-readable cites `apps/connectors/src/help.rs` and `website/docs/reference/cli.md`. Cited.
   - Scoped stories (the `aep waves` output lists these as inferred) sharing `ess/domains/cli.yaml`, `semantics.md` or the generated files:
     - story:definition-files-in-user-config.
     - story:guarded-write-credential-refusal-says-repair (also `crates/connectors-host/src/local/owner.rs` and `website/docs/reference/contracts/cli.md`).
     - story:standing-write-approval-for-unattended-agents (`apps/connectors/spec/cli.yaml`).
     - story:openrouter-inference-key-profile and story:mcp-cli-journey-discovery-contract (`semantics.md`).
   - Fix: add a "Shares files with" section naming these. Then either add a `depends_on` per pair with the shared file as the reason, or split the surface. I choose neither.

2. Unassessed, not safe. These carry no scope:
   - story:invoke-exits-promptly-on-sigterm. Its acceptance (SIGTERM during admission) is the same invoke path and the same cutoff in `invoke.rs`, `session.rs` and `operations.rs` (inferred).
   - story:tool-contract-for-any-instance. A new CLI command regenerates every whole file listed in the scoping facts (inferred).
   - story:cli-option-value-hint regenerates `completions/connectors.bash` and `spec/cli.yaml` (inferred).
   - Fix: give each a scope before this story is worked beside it, or keep it out of that set.

3. Two scopes are too wide to assess:
   - story:mcp-outbound-stdio-runtime claims `apps/connectors/src` and `crates/connectors-host/src/local`.
   - story:admission-checks-token-scopes claims `apps/connectors/src/local` and all of `ess`.
   - Both collide with every path here. Fix: narrow them before either runs beside this story.

4. The story's own scope misses the line-number citations. Adding lines to `contracts/cli/v1alpha1/semantics.md` above line 1491 breaks the test `the_corrected_semantics_citations_land_on_their_rules` (`crates/connectors-build/tests/cli_spec_mapping_adversary.rs:201`). Design says to edit near line 819 (the refusal beside `--idempotency-key`) and near line 1493, and both shift the citations. Fix: add to Design or Acceptance "the semantics.md line citations in `cli.yaml` (:1946, :2106) are renumbered to land on their rules".

5. Hand-written invoke flag lists that will go stale (grep for `--input-json` and `idempotency-key`, outside the 17 paths):
   - `adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json:259-274`, which lists the invoke flags and constraints, and its `discovery-contract.md`.
   - `docs/local-mcp-cli.md:147`, whose invoke row lists the flags.
   - Nothing generates or tests these: no Rust file reads them.
   - story:mcp-cli-journey-discovery-contract owns all three (inferred). Fix: add an Out of scope line saying so, or add the paths to this scope.

6. Generated files are covered. `apps/connectors-cli-contract` holds the regenerated `binding.json`, `completions/connectors.bash`, `help.txt` and `README.md`. The `waited_ms` member is declared in `ess/domains/cli.yaml` next to `approval_steps` (:899) and generated from there. `crates/connectors-build/generated/metadata-emissions/inputs.json` is listed. No Rust struct literal of `Failure` or `InvokeInput` needs the new member (grep). `owner.md` is not mirrored into `website/`. No fix needed.

7. Tests are under-named. Only `owner_slow_start.rs` is scoped, and it carries the 3-parallel-reads test. The refusal cases (0, 121, a non-number, the flag beside `--idempotency-key`) and the "reads 30 s" test have no file. Existing tests for that refusal sit in `apps/connectors/tests/invoke_short.rs`. Fix: name one existing file for them, or add `invoke_short.rs` to the scope.

8. The scope is only in the working tree (`.engineering/planning/story/invoke-admission-queues-under-load.md` shows ` M`). Other worktrees cannot see it. Fix: commit the story file before the parallel work starts.

Must not run beside this story:
- story:definition-files-in-user-config and story:tool-contract-for-any-instance (same generated files).
- story:standing-write-approval-for-unattended-agents and story:cli-option-value-hint (same `spec/cli.yaml`, same generated files).
- story:docs-generated-reference-readable (`help.rs`, `reference/cli.md`).
- story:missing-state-database-says-initialize, story:setup-check-names-missing-provider-with-instances, story:definitions-install-requires-a-valid-configuration and story:invoke-exits-promptly-on-sigterm (same source and the `semantics.md` line shift).
- story:guarded-write-credential-refusal-says-repair, story:openrouter-inference-key-profile, story:mcp-cli-journey-discovery-contract (`semantics.md` numbering).
- story:mcp-outbound-stdio-runtime and story:admission-checks-token-scopes (too wide).
- The Entity Runtime pin change: `inputs.json` lists `entity-*` and `ess-*` crate versions. It touches `Cargo.toml`, `Cargo.lock` and `metadata*` as well, so run it before or after, and regenerate `inputs.json` after the merge.

May run:
- story:joined-refresh-honours-the-caller-deadline (`owner/refresh.rs`, different file).
- story:cli-timing-cases-under-load.
- The slack stories (story:slack-sdk-upstream-import, story:slack-method-token-scope-table, story:slack-four-profile-definition-generated). They share only `inputs.json`, `website/docs/reference` and `crates/connectors-build` with this story. Regenerate `inputs.json` after the merge, and the other two paths are listed to check.
- story:spec-models-operation-admission shares `apps/connectors-cli-contract` and `inputs.json`, so regenerate both after whichever lands second.

Out of lane: whether a non-number can reach `deadline_seconds` as `invalid_input`, if the option is typed as an integer, is a design and acceptance question.
