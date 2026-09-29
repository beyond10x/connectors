---
format: aep.planning-md/3
id: review-result:adversary-basic-auth-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the basic auth profile
relations:
- reviews: story:catalog-basic-auth-profile
revision: 1
---
unit: story:catalog-basic-auth-profile, uncommitted working tree on d215b3569 (branch impl/catalog-basic-auth-profile)
verdict: CONFIRMED (4 notes, nothing red)
cases: executed 17→22, red 0
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path
needs-coordinator: the two acceptance CLI tests only run with --include-ignored; record that run as evidence or the gate never checks them

Five cases added, each green on first run: adversary_basic_header_encodes_utf8_accounts_and_colon_bearing_tokens; adversary_basic_account_limit_is_1024_bytes_not_characters; adversary_inconsistent_basic_profiles_refuse_at_load; adversary_explicit_token_scheme_keeps_revisions_of_an_omitted_scheme; cli_journey::adversary_basic_cli_wrong_token_is_unauthorized_and_malformed_is_invalid_input (ignored, like its neighbours).

Suite (`CONNECTORS_TEST_CLI=$PWD/target/debug/connectors cargo test --locked -p connectors-catalog-provider -- --include-ignored`): main.rs ok 1; bundle_drift ok 1; engine ok 5; local_runtime ok 14 passed, 0 failed; shipped ok 1; EXIT=0. Clippy -D warnings and fmt --check clean.

Attacked and could not break: encoder padding, UTF-8 and ':' in token; account colon/control/1024-byte checks; leakage (Authorization marked sensitive at http.rs:253, main.rs:38 prints payload-free Failure, no account or token on stdout, stderr, config, owner state, child argv or environ); token revisions unchanged; host accepts http_basic (runtime.rs:250).

```findings
[
  {"file": "adapters/catalog/tests/local_runtime/cli_journey.rs", "line": 423, "category": "acceptance", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The file and stdin connect acceptance tests are #[ignore]d so the package gate never runs them; they pass under --include-ignored and that run must be recorded as the evidence."},
  {"file": "adapters/catalog/tests/local_runtime/cli_journey.rs", "line": 462, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The wrong-token refusal asserts only service_failure, which protocol and internal failures also produce; the added case pins service_code unauthorized."},
  {"file": "adapters/catalog/src/local.rs", "line": 227, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The unit's suite had no case reaching the basic/token load-refusal branches (header, bearer, account_label); coverage was added and is green."},
  {"file": "adapters/catalog/src/local.rs", "line": 156, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A hand-written base64 encoder duplicates the workspace base64 dependency; it is correct, and base64 encode_slice into the pre-sized Secret would keep the zeroizing."}
]
```
