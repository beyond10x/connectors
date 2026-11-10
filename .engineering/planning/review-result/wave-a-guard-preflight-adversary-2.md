---
format: aep.planning-md/3
id: review-result:wave-a-guard-preflight-adversary-2
kind: review-result
status: active
title: Adversary pass 2 on catalog-guard-optional-preflight-values (wave A)
relations:
- reviews: story:catalog-guard-optional-preflight-values
revision: 1
---
```
unit: story:catalog-guard-optional-preflight-values (a64f6396b..8f42d7ae5; this pass HEAD~1..HEAD), plus one uncommitted test file
verdict: red
cases: executed 40→43, red 2
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/units/catalog-guard-optional-preflight-values/scratch/{adv2-run.sh,adv2-red.log,adv2-suite.log,adv2-done.log}
needs-coordinator: yes
```

1. Changed files: `?? adapters/catalog/tests/guard_optional_values_adversary_pass2.rs` (untracked test file only); `git diff --stat` empty.
2. Cases, run alone first (exit 101, `test result: FAILED. 1 passed; 2 failed`):
   - `optional_value_reading_a_body_key_the_closed_body_does_not_admit_is_refused`: RED; `body_keys: ["name"]` with `optional_values: {"mode": "body.mode"}` loads.
   - `optional_value_reading_a_string_list_body_key_is_refused`: RED; `body_types: {"mode": "string_list"}` with an optional `body.mode` loads.
   - `required_value_controls_are_refused`: green; the same selections with `values` are refused at load.
3. Suite after the cases existed: `cargo test --locked -p connectors-catalog-provider --test runpod_adversary --test guard_optional_values_adversary --test google_drive --test google_drive_writes_adversary --test shipped`, exit 0: google_drive 21, google_drive_writes_adversary 4, guard_optional_values_adversary 6, runpod_adversary 7 (incl. `every_shipped_selection_set_still_loads`), shipped 2. The new load-time refusals reject no shipped selection.
4. Finding: `adapters/catalog/src/lib.rs:880` `guarded_body_paths`. The round-1 correction removed `.chain(guard.optional_references())` to stop closed body keys becoming required; the same iterator feeds the load checks at `closed.rs:137` and `closed.rs:181`, so optional references lost body_keys admission, the string_list scalar conflict, the typed-key check, the scalar leaf type and the body_fixed conflict. No shipped selection reaches it; a hand-written local catalog provider does (`docs/local-catalog-provider.md:1075`). NEEDS-CHANGE. Fix named: keep optional refs in the check paths; build `closed.guarded` from `references()` only.
5. Attacked, not broken: the pass-1 corrections (not-required key, non-scalar refusal, undeclared-parameter refusal, both-maps refusal, importer identity path); `reads` zipped against `preflights()` in order; credential-stripped probe parameters refused by the new name check.

```findings
[{"file":"adapters/catalog/src/lib.rs","line":880,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"dropping optional_references from guarded_body_paths also dropped closed-body admission, string_list-scalar, typed-key and body_fixed load checks for optional_values body references, so a local catalog with body_keys [name] and optional body.mode now loads (refused at HEAD~1); red in tests/guard_optional_values_adversary_pass2.rs"}]
```
