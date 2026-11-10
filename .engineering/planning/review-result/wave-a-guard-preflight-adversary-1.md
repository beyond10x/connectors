---
format: aep.planning-md/3
id: review-result:wave-a-guard-preflight-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on catalog-guard-optional-preflight-values (wave A)
relations:
- reviews: story:catalog-guard-optional-preflight-values
revision: 1
---
unit: story:catalog-guard-optional-preflight-values (HEAD b18f46282 on impl/catalog-guard-optional-preflight-values, plus one untracked test file)
verdict: green
cases: executed 110→116, red 5
origin: introduced 4, pre-existing 0, undecided 1
wrote-outside-worktree: <scratch>/units/catalog-guard-optional-preflight-values/scratch/{red-case.log,suite.sh,suite-provider.log,suite-catalog.log,docs-check.log,suite-done.log}
needs-coordinator: no

1. Diff: `?? adapters/catalog/tests/guard_optional_values_adversary.rs` (untracked, 228 lines). `git diff --stat` shows nothing. It is a test file only; no implementation file changed. I ran rustfmt on that file only, after the red run.

2. The cases were run alone first (`cargo test --locked -p connectors-catalog-provider --test guard_optional_values_adversary`): `test result: FAILED. 1 passed; 5 failed`, exit 101.
- `closed_body_key_read_only_as_an_optional_value_is_not_required`: red. `...made the closed body key `mode` required: {..."required":["mode"]...}`
- `given_non_scalar_optional_value_is_not_silently_dropped`: red. `...dropped from the read and the write prepared; reads: [(["items", "i-1"], [])]`
- `optional_value_naming_no_parameter_of_the_read_is_refused_when_it_loads`: red. The preflight binding `mdoe` loaded.
- `required_value_naming_no_parameter_of_the_read_is_refused_when_it_loads`: red. Same result with `values`; this case is the comparison for the one above.
- `parameter_bound_both_required_and_optional_is_refused_when_it_loads`: red. `mode` bound in both `values` and `optional_values` loaded.
- `open_body_optional_value_is_forwarded_when_given`: green (control).

3. Suite, run after the cases existed:
- `cargo test --locked -p connectors-catalog-provider --test guard_optional_values_adversary --test google_drive_writes_adversary --test google_drive --test engine`: engine 44 ok, google_drive 21 ok, google_drive_writes_adversary 4 ok, mine 1 passed and 5 failed. Exit 101.
- `cargo test --locked -p connectors-catalog --test definition_import`: `ok. 41 passed`, exit 0.
- `cargo run --locked -p connectors-docs -- generate --check`: `139 generated files: current`, exit 0.
- The before count of 110 is the same targets without my file.

4. Findings. They cover b18f46282. The story's Acceptance holds for shipped Drive without fixing any of them.
- **`adapters/catalog/src/lib.rs:883` (introduced, CONFIRMED, warning).** `guarded_body_paths` now includes optional references, and `closed.rs:481` declares every guarded key required. So a closed body key read only through `optional_values` becomes required, and a body without it is refused at input validation. The guide says an absent value is left out. What reaches it: the documented workflow in `docs/local-catalog-provider.md:1075`, which allows `body.` references "like `values`". Nothing shipped uses it. Fix: keep optional references out of `closed.guarded`, or refuse `body.` references in `optional_values` when the selection loads.
- **`adapters/catalog/src/lib.rs:2016` (introduced, INFEASIBLE, note).** An optional value that is given but is not a scalar is dropped without a refusal. The write sends it and the read omits it; a required value in the same place refuses. Top-level parameters are protected because `parameter_values` refuses non-scalars earlier. What reaches it: only an optional `body.` reference in an open body. Nothing found. Fix: refuse when `reference` is `Some` and `scalar` is `None`.
- **`adapters/catalog/src/lib.rs:1422` (introduced, INFEASIBLE, note).** The loader does not check that an `optional_values` key is a parameter of the read. A typo loads, and a test that omits the value never sees it. The definition compiler does refuse it (`GuardParameter`), so only hand-written `operations.json` reaches it.
- **`adapters/catalog/src/lib.rs:1422` (undecided, INFEASIBLE, note).** The same gap exists for `values`. I did not run it against the base.
- **`adapters/catalog/src/lib.rs:2013` (introduced, INFEASIBLE, note).** One parameter bound in both maps loads. The required binding silently wins, and neither the guide nor `guard.yaml` says so. The importer also drops an identity path-parameter entry from `with_optional` (`import.rs:1222`). Nothing found that reaches it.
- **`contracts/catalog/v1alpha1/semantics.md:278` (introduced, NEEDS-CHANGE, warning; no test, judgement).** §7 Compatibility records `mismatch` and `present` but not `optional_values`. `Preflight` denies unknown fields, so an older reader refuses the shipped google-drive selection, and its configuration revision changes.
- **`adapters/catalog/spec/ess/domains/guard.yaml:83` and `adapters/catalog/tests/google_drive_writes_adversary.rs:117` (introduced, CONFIRMED, note; judgement).** The model comment above `Preflight` does not describe `optional_values`. The doc comment on the flipped test still says the guard maps only `fileId` and `fields`.

5. Attacked and could not break:
- Bundle drift: generated bundles hold only inventory and auth, so no `connectors-build catalog` regeneration was needed.
- Importer round trip for google-drive: green.
- `connectors-docs generate --check`: current.
- No stale shared-drive claim is left in docs or descriptions.
- Credential and withhold checks cover the optional references.
- `declare` handles top-level optional references.
- The postflight read does not use `optional_values`.

6. Outside the worktree I wrote only the six scratch files listed in the header. `.local/tmp` in the worktree holds the implementor's files, which predate this session.

7. Findings block:
```findings
[
  {"file": "adapters/catalog/src/lib.rs", "line": 883, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "a closed body key read only through optional_values is declared required via closed.guarded, so a body without it is refused instead of the value being left out of the preflight"},
  {"file": "adapters/catalog/src/lib.rs", "line": 2016, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "a given non-scalar optional body value is silently dropped from the preflight while the write carries it, where a required value would refuse"},
  {"file": "adapters/catalog/src/lib.rs", "line": 1422, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "an optional_values key naming no parameter of the probe loads and is refused only when a caller gives it, so a typo passes tests that omit the value"},
  {"file": "adapters/catalog/src/lib.rs", "line": 1422, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "undecided", "message": "a values key naming no parameter of the probe also loads in a hand-written selection, unchecked until prepare"},
  {"file": "adapters/catalog/src/lib.rs", "line": 2013, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "a parameter bound in both values and optional_values loads and the required binding silently wins, with no rule in the guide or model and the importer dropping an identity path entry from with_optional"},
  {"file": "contracts/catalog/v1alpha1/semantics.md", "line": 278, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the Compatibility section does not record the new optional preflight optional_values, which an older reader denies as an unknown field and which changes the shipped google-drive configuration revision"},
  {"file": "adapters/catalog/spec/ess/domains/guard.yaml", "line": 83, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the Preflight model comment and the flipped test's doc comment at google_drive_writes_adversary.rs:117 still describe a preflight without optional values"}
]
```
