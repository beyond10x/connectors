---
format: aep.planning-md/3
id: review-result:replan-template-store-parallel-safety-round-3
kind: review-result
status: active
title: Parallel-safety critic, round 3, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: **approve**

Everything below was read at commit cb5e856e8. `git rev-parse HEAD` printed it and `git status` was empty. The reads were done without `cargo` or `slot`.

## Round-2 findings

1. Blocking (N7 regenerated `inputs.json` as one pool job): **resolved.** N7 point 4 now names the three `--part` runs and cites `docs/development.md` lines 92 to 97. It runs one part, `--part component`, as one job that names 30 minutes. The `slow` job holds the other two parts with `--check`. Its "Not in this story" says the jobs run on the build host. Source check of "one part is enough":
   - `inputs(root)` (`crates/connectors-build/src/metadata_emissions.rs` lines 91 to 117) reads `ess/`, the three `SOURCES` and `Cargo.lock`. It takes no part argument.
   - `generate` (lines 361 to 410) pushes it last, after the `wanted(...)` blocks, whichever part is selected.
   - So `inputs.json` is byte-identical whichever part writes it.
2. Note (incomplete list of who shares `inputs.json`): **resolved.** N7's scope section now reads as follows.
   - It names the 9 text-only drafts as unassessed.
   - It says 27 files (`inputs.json` has 27 `ess/domains/` paths and 5 others, counted with `grep`; auth_bindings at line 52, cli at line 56).
   - It lists the five other inputs and the four drafts that record `connectors-build` as a directory.
   - It prices a merge at "at least one part's job" for a comment and three for a model change.
3. Note (base commit of a pull-request gate job): **resolved.** S9d acceptance 4 now says base is the first parent of the merge the job tested, read from its checkout step. That is correct for a `pull_request` run.
4. Note (timing runs overlapping on the build host): **resolved.** S9e acceptance 1 says no other timing run of the set starts while its six runs are made. It also records the pool's job count (`slot status`) beside each duration. N6 lines 261 and 262 say the reverse: items 9 and 11 do not start during those runs.
5. Note ("stories of the set that name the file"): **resolved.** S9d now says "write" and names who only names `er.rs`.

## Blocking findings

None.

## Notes

1. N7 point 4 puts `--fetch` after "names 30 minutes" without a full command line. It is a `slot` flag (`slot --timeout 30m --fetch <path> …`) and not an argument of `connectors-build`. The client copies it with `rsync -a`, which also works for a directory. An implementor who knows `slot` reads it correctly, and a full command line would remove the doubt.
2. The precedent for "a comment changes no emitted manifest" is two comment-only edits of `cli.yaml` (`c97cb5660` and `df1f2c5b1`). Each changed `cli.yaml` and `inputs.json` and nothing else under `generated/`. There is no precedent for `auth_bindings.yaml`. By the source, `spec_digest` in each manifest comes from `SuiteProvenance::of(ir)`, which hashes the compiled model and not the file bytes (the call chain in the dependency was read, not the hash itself). The `slow` job's `--check` of all three parts would catch a miss, and N7 already says what to do then.
3. The `slow` job (`slow.yml` line 115) runs `--check` with no `--part`, so it covers all three parts in one step, within a 60 minute job limit. How long it takes on the forge is outside this lens.
4. S9e's job count from `slot status` is a record only and sets no limit. A busy pool can still lengthen runs. S9e's 7.0 s base floor and its rule that a refuted cause means nothing is merged guard that, so this is a note only.
5. N7 counts "the thirteen above" as sharers of `cli.yaml`. That includes `admission-checks-token-scopes`, which records only the directory `ess`, so the count is a ceiling.

## Checked and found correct

- N7's lists against `inputs.json`: 27 under `ess/domains/` plus 5 other paths (`components.yaml`, `system.yaml`, `toolchain.json`, `metadata_emissions.rs`, `metadata_entities.rs`).
- `slow.yml` triggers on `ess/**` and `crates/**` for pull requests. The step name N7 acceptance 5 cites exists at line 114.
- S9d's change since round 2: nine test-module calls of `definition_bundle()`, a thread that enters the namespace, and a `mod tests` limited to one new test. None of them adds a shared file.
- S9d and N7 write different functions of different files, as in round 2. N7's `er.rs` change is none.
- S9e writes only `adapters/sql/tests/mysql/fixture.rs`. N6's items 9 and 11 are ordered against it in the text of both stories.
- N6's items 12 and 13 add no shared-file write. Its timers stay in a worktree made for the measurement.

## Not checked

- No forge run was read, so the checkout of a past gate job was not checked against the "first parent of the merge" sentence. It rests on how `actions/checkout` handles a `pull_request` event.
- The 8 to 27 minutes per part were not run (rules of the brief).
- The body of `ess_gen::Provenance::of` was not read. It is the basis of note 2.

## For another critic

None.
