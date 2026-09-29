---
format: aep.planning-md/3
id: review-result:adversary-basic-auth-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the basic auth profile
relations:
- reviews: story:catalog-basic-auth-profile
revision: 1
---
unit: story:catalog-basic-auth-profile, uncommitted working tree on d215b3569 (wave0929-basic-auth), after the base64-crate correction
verdict: nothing found
cases: executed 22→24, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path
needs-coordinator: none

Three cases added in adapters/catalog/tests/local_runtime/basic_auth_adversary_pass2.rs, all green on first run: adversary2_documented_basic_auth_example_loads_as_http_basic; adversary2_maximum_account_encodes_exactly (1024-byte account, 3001-byte token, exact bytes); adversary2_duplicate_account_key_refuses_before_any_request.

Suite (`CONNECTORS_TEST_CLI=$PWD/target/debug/connectors cargo test --locked -p connectors-catalog-provider -- --include-ignored`): bundle_drift 1, engine 5, local_runtime 17, shipped 1, all ok, EXIT=0. Clippy -D warnings clean; fmt --check clean.

Attacked and could not break: base64 0.23.1 encoded_len/encode_slice sized exactly, Secret zeroizes on drop (crates/connectors-sdk/src/lib.rs:38); only the existing base64 lock entry added; the documented example loads; redirects cannot forward the header (Policy::none(), crates/connectors-host/src/http.rs:124).

```findings
[]
```
