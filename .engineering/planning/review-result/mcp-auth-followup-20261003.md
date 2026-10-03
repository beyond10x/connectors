---
format: aep.planning-md/3
id: review-result:mcp-auth-followup-20261003
kind: review-result
status: active
title: 'Auth final adversary: unsupported HTML table residue'
relations:
- reviews: story:mcp-outbound-auth-lifecycle
revision: 1
---
unit: story:mcp-outbound-auth-lifecycle — corrected working tree based on b360aaaa771ad9a31cab46ff4249015b74a4d50f
verdict: CONFIRMED
cases: executed 27→28, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none authored
needs-coordinator: route HTML-block residue once and verify retained tests; no third adversary campaign

1. `git --no-pager diff --stat`: empty because the three inherited source files remain untracked. Against the captured second-pass frozen test, the review-only diff is:

```
 .../tests/mcp_outbound_auth_lifecycle.rs           | 23 ++++++++++++++++++++++
 1 file changed, 23 insertions(+)
```

Only the appended regression changed. `review-only.patch` preserves the exact additions. No implementation, contract, JSON, existing assertion or previous regression was edited. Author correction report SHA256: `5c3e190c6218749bf006b45cd4b35587a4cabcec7984e202e2c7dc5536cfc7f2`.

Frozen source SHA256:

```
ac46fdca07cb1cb607bef35db115682bc1d46a5dc4d0a5773eca1c1ec4156852  adapters/mcp/contracts/client/v1alpha1/auth.md
65d655e428eeaa03afc57af1182246131e94359b874e97ec4086a10d94830b48  adapters/mcp/contracts/client/v1alpha1/auth-cases.json
13319d3bf3dd607eaee0ad00e6b68e20920248be2562f69ffce1344fffafba2b  crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs
```

Final test SHA256 after formatting only the appended test: `77837371f54c662e907b2bb93acced01d236c0da4a1e96ea6c2fcd8c964891a2`. Both contract hashes are unchanged. The initial test hash at execution before formatting was `8638da7e12672a60b5c2e124444b9d03b05342423c4f2f5376beb22abb572377`.

2. Before any execution, added `review_visible_html_case_inventory_cannot_bypass_correspondence` at `crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs:714`. It appends a syntactically ordinary HTML table with the same four header names and an existing missing-credential case ID, but a contradictory outcome. It exercises both selected revisions through the actual document validator. Both copies were accepted. First execution was RED, 0 passed/1 failed/7 filtered, exit 101. No production source document was mutated.

```
CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache TMPDIR="$PWD/.local/mcp-two-waves/auth-review-second" cargo test --locked --offline -p connectors-build --test mcp_outbound_auth_lifecycle review_visible_html_case_inventory_cannot_bypass_correspondence -- --exact
```

Verbatim first deciding output:

```
    Blocking waiting for file lock on package cache
   Compiling connectors-build v0.25.1 (<worktree>/crates/connectors-build)
    Finished `test` profile [optimized] target(s) in 4.52s
     Running tests/mcp_outbound_auth_lifecycle.rs (target/debug/deps/mcp_outbound_auth_lifecycle-5b0696f248a44bdb)

running 1 test
test review_visible_html_case_inventory_cannot_bypass_correspondence ... FAILED

failures:

---- review_visible_html_case_inventory_cannot_bypass_correspondence stdout ----

thread 'review_visible_html_case_inventory_cannot_bypass_correspondence' (3932646) panicked at crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs:727:5:
contradictory visible HTML case inventory accepted for: ["auth.2026-07-28.missing", "auth.2025-11-25.missing"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    review_visible_html_case_inventory_cannot_bypass_correspondence

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.06s

error: test failed, to rerun pass `-p connectors-build --test mcp_outbound_auth_lifecycle`
```

3. Affected suite ran only after the deciding result. Baseline 27 is from the frozen author correction report. Actual 28 executed: auth 7 passed/1 failed, lifecycle 11 passed, invocation 9 passed; 27 passed/1 failed/0 ignored, exit 101. The original contradictory Markdown-row regression and the author's structural-table regression both pass.

```
CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache TMPDIR="$PWD/.local/mcp-two-waves/auth-review-second" cargo test --locked --offline -p connectors-build --no-fail-fast --test mcp_outbound_auth_lifecycle --test mcp_outbound_invocation_results --test mcp_outbound_connection_lifecycle
```

Verbatim suite output:

```
    Finished `test` profile [optimized] target(s) in 0.37s
     Running tests/mcp_outbound_auth_lifecycle.rs (target/debug/deps/mcp_outbound_auth_lifecycle-5b0696f248a44bdb)

running 8 tests
test anonymous_retry_and_private_disclosure_claims_fail ... ok
test authored_auth_contract_has_complete_unique_source_bound_cases ... ok
test uncertain_refresh_and_unacknowledged_persistence_cannot_claim_success ... ok
test review_visible_html_case_inventory_cannot_bypass_correspondence ... FAILED
test missing_duplicate_and_orphan_rows_fail ... ok
test real_fact_changes_invalidate_old_outcomes ... ok
test review_contradictory_duplicate_document_rows_are_rejected ... ok
test correspondence_requires_one_visible_structural_table ... ok

failures:

---- review_visible_html_case_inventory_cannot_bypass_correspondence stdout ----

thread 'review_visible_html_case_inventory_cannot_bypass_correspondence' (3940718) panicked at crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs:727:5:
contradictory visible HTML case inventory accepted for: ["auth.2026-07-28.missing", "auth.2025-11-25.missing"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    review_visible_html_case_inventory_cannot_bypass_correspondence

test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.80s

error: test failed, to rerun pass `-p connectors-build --test mcp_outbound_auth_lifecycle`
     Running tests/mcp_outbound_connection_lifecycle.rs (target/debug/deps/mcp_outbound_connection_lifecycle-55b446a1c20745b9)

running 11 tests
test the_document_does_not_close_the_revision_set_the_model_leaves_open ... ok
test every_refusal_names_a_declared_wire_code_and_loss_is_not_failure ... ok
test every_state_the_story_names_has_a_named_outcome_and_a_scenario_file ... ok
test every_refusal_names_whose_act_it_reports ... ok
test outbound_stdio_is_held_by_its_blocker_and_nothing_else_is ... ok
test every_scenario_this_story_owns_names_a_row_of_the_register ... ok
test no_citation_into_an_editable_document_carries_a_line_number ... ok
test no_state_is_answered_by_sending_the_request_again ... ok
test every_citation_resolves_into_an_archived_file_of_the_pin ... ok
test every_section_and_scenario_resolves_which_revision_it_holds_for ... ok
test no_sentence_writes_a_field_of_the_binding_entity ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s

     Running tests/mcp_outbound_invocation_results.rs (target/debug/deps/mcp_outbound_invocation_results-e1c0ff323d459de1)

running 9 tests
test review_exact_octet_ceilings_and_terminal_loss_preserve_uncertainty ... ok
test review_non_string_and_unselected_result_types_across_selected_families ... ok
test modern_resource_cache_metadata_is_required_by_the_pinned_schema ... ok
test review_peer_errors_and_malformed_answers_never_establish_non_execution ... ok
test revision_and_error_and_limit_mutations_are_rejected ... ok
test contradictory_effect_retry_authority_and_bytes_are_rejected ... ok
test invocation_document_and_cases_agree_with_selected_sources ... ok
test partial_business_and_structured_observations_cannot_be_relabelled ... ok
test missing_duplicate_and_document_only_mappings_are_rejected ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

error: 1 target failed:
    `-p connectors-build --test mcp_outbound_auth_lifecycle`
```

Initial `cargo fmt -p connectors-build --check` exited 1 solely because my appended iterator expression needed line wrapping. Ran `cargo fmt -p connectors-build`, then `cargo fmt -p connectors-build --check` exited 0 with empty fmt-final.log. No semantic edit after the deciding/suite executions. No Clippy repeat or full gate.

4. Finding covers the corrected frozen tree plus the single second-pass appended test.

| File:line | Verdict | Origin | Finding | What was measured | What reaches it |
|---|---|---|---|---|---|
| crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs:447 | CONFIRMED | introduced | Whole HTML blocks bypass the case-inventory guard, allowing a second contradictory visible case table despite the unique-correspondence guarantee. | The added test's two HTML table mutations both return Ok; first run exit 101. | The normal authored-contract test reads the complete Markdown into validate; raw HTML is neither forbidden by the authored contract nor rejected by this checker. An ordinary edit appending this table reaches the same path. No current candidate contains the table, and no runtime auth path is implicated. |

Supported subset observed from code: pulldown-cmark Markdown with ENABLE_TABLES; one unquoted four-column case table; inline text/code are collected, rows may reorder, fenced examples are non-authoritative, quoted case tables and visible case-shaped paragraphs are refused. HTML inside a Markdown table cell is explicitly refused. Whole HTML blocks outside such cells fall through the final wildcard, so this is an unexpressed unsupported subset, not a declared authoring restriction. Refusing unsupported raw HTML explicitly is sufficient; no general HTML parser or broader documentation redesign is required.

Reachability limits: the source Markdown is the artifact checked and reviewed in the repository; HTML table syntax is visible content in ordinary repository Markdown rendering. This pass measured the exact guard's acceptance and source path, not a browser or deployed documentation rendering. No evidence is claimed that this new auth document is currently selected for website publication. The defect is incomplete drift protection for a reachable source edit; no deployed malicious table or credential behavior is alleged.

The auth guard path is absent at base b360aaaa, so origin is introduced. This is nearby residue in the same complete-inventory class, not an unrelated security finding. Preserve both adversary regressions and the author's structural test while correcting it. No additional judgement findings.

5. The corrected Markdown duplicate, fenced/quoted/misplaced/reordered-table controls all execute green in the affected suite. The one new attack is the unhandled HTML-block boundary, with two revision inputs. No additional mutation class or third campaign was run; runtime authentication and unresolved native ESS bindings remain outside this document-guard pass.

6. No authored paths outside the assigned tree. Scratch `.local/mcp-two-waves/auth-review-second` holds baseline-test.rs, baseline-sha256.txt, review-only.patch, deciding.log/exit, suite.log/exit, fmt.log/exit, fmt-final.log/exit, and this report. Cargo used its existing tree-local target and the assigned scratch TMPDIR with two jobs/sccache; shared tool caches and lease metadata are tool-owned effects. Free space was about 30 GiB before the incremental build. Own lease released on handoff; coordinator owns final correction routing, publication and cleanup.

Publication normalization: the private absolute compiler checkout path is rendered as <worktree>. All findings, test names, assertions, counts and exit statuses are unchanged. The original immutable unpublished report and raw logs remain in the private recovery archive.

```findings
- file: crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs
  line: 447
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: Whole HTML blocks bypass the case-inventory guard, allowing a second contradictory visible case table despite the unique-correspondence guarantee.
```
