---
format: aep.planning-md/3
id: review-result:adversary-catalog-parameter-shapes-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the engine lane (repeated, required, refusal shapes)
relations:
- reviews: story:catalog-repeated-query-parameters
- reviews: story:catalog-selection-required-parameters
- reviews: story:catalog-engine-provider-refusal-shapes
revision: 1
---
unit: engine lane `impl/catalog-query-parameter-shapes` at 2e14b1e18 (base 81b509745), working tree `~/.local/state/worktree/trees/b10x/connectors/wave0930c-engine`
verdict: NEEDS-CHANGE
cases: executed 236→240, red 4
origin: introduced 4 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 path (`~/.cache/w0930en/adv1/suite.log`)
needs-coordinator: none

**1. Diff of the tree.** `git --no-pager diff --stat` is empty. `git status --short` shows one new file, a test file: `?? adapters/catalog/tests/engine_adversary.rs`. No implementation file was touched.

**2. Cases added** in `adapters/catalog/tests/engine_adversary.rs`. All 4 are red now. First run of the file alone: `test result: FAILED. 0 passed; 4 failed`.

| case | asserts | red output (verbatim, trimmed) |
|---|---|---|
| `adversary_repeated_parameter_declares_type_array` :84 | The story's acceptance: the repeated parameter is declared `type: array` | `left: Array [String("array"), String("string"), String("integer"), String("boolean")]  right: String("array")` |
| `adversary_a_scalar_for_a_repeated_integer_parameter_is_typed_like_its_elements` :93 | `{"ids":"abc"}` is refused when the elements are declared integer | `"abc" was sent as [(["messages"], [("ids", "abc")])]` |
| `adversary_a_required_repeated_parameter_declares_what_the_engine_refuses` :117 | The declared schema and the engine agree on `[]` for a required repeated parameter | `the declared schema accepts {"labelIds":[]} and the engine refuses it: {...,"required":["labelIds"],...}` |
| `adversary_an_instance_without_repeated_parameters_keeps_its_descriptor_revision` :236 | A GitLab instance with project.get, merge_request.get, branch.get and the 3 guarded writes (none has a repeated parameter) has the same descriptor revision over the base bundle and over the new one. The test rebuilds the base bundle from the current one and checks its sha is `da458f3b…`, the base index value; that check passed. | `left: "7611670a…"  right: "1db812d3…"` |

**3. Suite run.** `cargo test -p connectors-catalog -p connectors-catalog-provider --no-fail-fast` exited 101. The only failing target was `engine_adversary` (`0 passed; 4 failed`); every other binary passed, including `bundle_drift` (1 passed) and `engine` (20 passed). 240 cases ran; 236 of them were outside `engine_adversary`, and that is where the "before" count comes from. `rustfmt --check` on the file and `cargo fmt --package connectors-catalog-provider -- --check` both exit 0. `cargo clippy -p connectors-catalog-provider --test engine_adversary -- -D warnings` exits 0.

**4. Findings** (tree above, commit 2e14b1e18)

| # | file:line | measured | what reaches it | verdict / origin |
|---|---|---|---|---|
| 1 | `docs/local-catalog-provider.md:227` | The doc says only 7 reads' descriptor revisions change. In fact there is one revision per instance, not per operation: `digest(configuration_revision, op ids)` at `adapters/catalog/src/local.rs:362`, and the configuration revision digests `bundle_sha256` (`local.rs:293`). So every GitLab, Jira and Confluence instance moves, writes included. Case :236 is red. | Every operator with a write approval policy: those policies bind to the descriptor revision (`crates/connectors-host/src/local/owner/approval_issuance.rs:82`). The doc tells operators to re-issue for reads, which have no approval policies, and leaves out the writes, whose policies do go stale. The commit message says the same thing. | NEEDS-CHANGE / introduced |
| 2 | `adapters/catalog/src/lib.rs:844` | The declared type is the union `["array","string","integer","boolean"]`. The story's acceptance says `type: array`. The unit's own `engine_declares_array_schema` was changed to check that the type *contains* `"array"`. Case :84 is red. | Every describe of Jira `issues.search`, the 4 Confluence reads, and GitLab `issues.list` / `merge_requests.list` | CONFIRMED / introduced |
| 3 | `adapters/catalog/src/lib.rs:453` | When the input is one scalar, the element-type check is skipped (`typed = !repeated`) and the union type allows any scalar. So `"abc"` or `"1,2"` is sent where the declared elements are integers. Case :93 is red. At base, `declared_type(None)` also accepted any scalar (`git show 81b509745:adapters/catalog/src/lib.rs:696`). | Jira `reconcileIssues`, Confluence `id` and `space-id`, GitLab `not[iids]` | CONFIRMED / pre-existing |
| 4 | `crates/connectors-catalog/src/template.rs:643` | `carried` refuses `[]` on a required parameter, but the declared schema has no `minItems: 1`. Case :117 is red. | Nothing found. A scan of the bundles shows no shipped selection with a required repeated parameter. | INFEASIBLE / introduced |
| 5 | commit 2e14b1e18 message | The message says "Jira (107)" repeated parameters. The bundle has 110 (`jq`), and an independent count from the source also gives 110 (109 with defaults plus 1 with `explode: true`). | Readers of the commit message only | CONFIRMED / introduced |

**5. Attacked and could not break**
- **Array reaching a non-repeated parameter:** blocked twice, at `template.rs` `place` and at lib.rs `parameter_values`, including when two parameters share a name.
- **Element escaping:** each pair is form-encoded at `crates/connectors-host/src/http.rs:239`, and `template_repeats_query_pairs` checks `a%26b%3Dc`.
- **Bounds:** they apply to every element, and a comma string on a bounded parameter is refused.
- **Empty array on an optional parameter:** sends nothing.
- **Selection `required`:** naming a path or header parameter is refused at load, and `required` combined with bounds works as intended.
- **`rate_limit_reasons`:** matched exactly, never as a substring, and a non-JSON body stays `forbidden`.
- **Write 403 on commit:** recorded as `Refused` (`runtime/writes.rs:194`), and I found no retry path.
- **Empty bodies:** in text mode an empty body is `""`; for JSON it is `null`.
- **Bundle bytes:** `bundle_drift` passes, so the committed bundles match a fresh run.
- **GitLab 67 gaps:** my own count from `openapi_v3.yaml` agrees (67 `form` with `explode: false`, 45 repeated).
- **The list of 7 reads:** it matches the repeated parameters in the bundles. Only the revision mechanism and its scope (finding 1) are wrong.

**6. Paths written outside the worktree:** `~/.cache/w0930en/adv1/suite.log`. The throwaway `gitlab.json` I made there has been deleted, and the `tempfile` directories under that TMPDIR removed themselves. Build output went to the assigned `<worktree>/target`. My lease `wave0930c-engine-adv1` is released.

```findings
- file: docs/local-catalog-provider.md
  line: 227
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The descriptor revision belongs to the instance and digests the bundle sha, so the rebuild changes it for every GitLab, Jira and Confluence instance, and so every write approval policy, not only the 7 reads the guide names."
- file: adapters/catalog/src/lib.rs
  line: 844
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "A repeated parameter is declared with the type union [array, string, integer, boolean], where the story acceptance requires type array."
- file: adapters/catalog/src/lib.rs
  line: 453
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "One scalar for a repeated parameter skips the element type check, so a non-integer or comma-joined string reaches the provider for an integer-element array."
- file: crates/connectors-catalog/src/template.rs
  line: 643
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "The declared schema accepts an empty array for a required repeated parameter that the engine refuses as absent; no shipped selection reaches this."
- file: commit:2e14b1e18
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The commit message counts 107 repeated Jira parameters; the bundle and the pinned source both have 110."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
