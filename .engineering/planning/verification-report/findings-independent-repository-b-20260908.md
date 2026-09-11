---
format: aep.planning-md/1
id: verification-report:findings-independent-repository-b-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:independent-repository-b-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: c4762dfd6551e1d0f7f14d2474cdc58b7e120cbecca3abbc58bc315570306ba5
relations:
- verifies: review-result:independent-repository-b-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:independent-repository-b-20260908

This supplements [the immutable original](../review-result/independent-repository-b-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

12 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": "crates/connectors-spec/src/toolchain.rs",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "| F01 | major | verified | `crates/connectors-spec/src/toolchain.rs:136-146`, `:182-198`, `:54-58`; `crates/connectors-build/src/gate.rs:37` | `missing_pin_reports_record_and_search_locations_and_cache_is_supported` is flaky: each of the 3 toolchain tests writes a shell script with `std::fs::write` and then `exec`s it via `Command::output()`; a sibling test's fork can hold the write fd open across the exec window, producing ETXTBSY. The README gate runs `cargo test --workspace`, so the gate fails nondeterministically (2 of 3 gate runs here). `docs/verification.md:263-268,289-293` and README present the gate as the reproducible acceptance. | `03-gate.log`: `Searched: …/cache/ess: Text file busy (os error 26); …/wrong/ess: found ess 99.0.0`; `05-flake-experiment.log`: `parallel run 4: … Text file busy`; serial runs 3/3 ok; row 9 passed only on the third gate attempt |",
    "line": 136
  },
  {
    "file": "adapters/sql/src/lib.rs",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "| F02 | major | verified | `adapters/sql/src/lib.rs:119` (`SET LOCAL statement_timeout = '10s'`), `:183-211` (per-row `query_portal`, each Execute re-arms the timer from the current GUC), `:222-227` (`ConnectionTask` drop only aborts the socket task), `:289-291` (15 s adapter timeout); contract `contracts/service/v1alpha1/semantics.md` \"Database-enforced read-only transactions and statement timeouts are mandatory\" and \"SQL cancellation includes a database statement deadline and connection cleanup\" | An authenticated caller's query can disable the database statement deadline from inside the read-only transaction (`set_config('statement_timeout','0',true)` is USERSET and not a write), and when the adapter's 15 s timeout then fires nothing cancels the server-side statement (no `CancelToken`/`cancel_query`); the backend stays active after the adapter gives up and even after the process exits. Each such request pins one backend for the query's duration; 32 slots × repeated calls exhausts `max_connections`. The contract's \"mandatory\" claim is false as written. | `10-sql-live-timeout-check.log`: `sleep: exit=1 wall=10.1s … \"code\":\"timeout\"` / `active backends after control: 0`; `bypass: exit=1 wall=15.0s … \"database request timed out\"`; `backend after bypass: 97\\|active\\|18\\|SELECT CASE WHEN (COALESCE(octet_length(result.c0::text)…`; `backend +15s: 97\\|active\\|33`; `backend after adapter exit: 97\\|active\\|33` |",
    "line": 119
  },
  {
    "file": "crates/connectors-build/src/gate.rs",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| F03 | minor | verified | `crates/connectors-build/src/gate.rs:118-124` | `--msrv` unconditionally sets `CARGO_TARGET_DIR` to `<root>/target/msrv`, overriding the caller's `CARGO_TARGET_DIR`; the README command cannot be run against a private target directory, and it writes into the shared `target/` even when the rest of the gate does not. | `.env(\"CARGO_TARGET_DIR\", root.join(\"target/msrv\"))` at `gate.rs:124`; I ran the equivalent check separately (row 6, exit 0) |",
    "line": 118
  },
  {
    "file": "adapters/gitlab/upstream/README.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| F04 | minor | verified | `adapters/gitlab/upstream/README.md:28`; `docs/design.md:1247`; `docs/design.md:3` | ESS-version drift after the 0.20.0 upgrade: the upstream README says `ess-import.json` \"retains the complete ESS 0.9.2 refusal\", design §28 says GitLab \"lowers … into ESS 0.9.2\", and the design status line stops at \"sections 27–28\" although §29–30 exist; the committed bundle was regenerated with 0.20.0. | `adapters/gitlab/generated/manifest.json`: `\"ess\":\"ess 0.20.0\"`; `docs/gitlab-generation.md:80-92` describes the 0.20.0 regeneration changing `ess-import.json`; row 7 confirms the bundle matches the 0.20.0 pin |",
    "line": 28
  },
  {
    "file": "docs/design.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| F05 | minor | verified | `docs/design.md:1283-1285`; `contracts/README.md:9-31,33` | Contract index counts disagree: design §30 says \"14 proposed … 4 families deferred\"; `contracts/README.md` lists 16 proposed rows over 16 proposed documents (17 `semantics.md` files, 1 implemented) and 3 deferred families (`execution`, `events`, `resources`); `contracts/service/v1alpha2/semantics.md` (committed in `f493827`) is absent from the index (0 mentions). `docs/stack-integration-proposal.md:44` defers the index row to a draft story, so the index is knowingly stale. | `ls contracts/*/*/semantics.md contracts/*/*/*/semantics.md \\| wc -l` = 17; `grep -c v1alpha2 contracts/README.md` = 0 |",
    "line": 1283
  },
  {
    "file": "docs/stack-integration-proposal.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| F06 | minor | verified | `docs/stack-integration-proposal.md:3-4`; `contracts/service/v1alpha2/semantics.md:6` | Both documents ground their gaps G1–G10 in `.local/review/2026-09-08-concept-and-stack-integration-review.md`, an ignored untracked file, so the basis of the v1alpha2 proposal cannot be audited from the tree. The planning store's own rule (`.engineering/planning/specification/contract-review-intake-20260908.md`: reviews are preserved \"so the plan does not depend on ignored workspace files\") was applied to F/E findings but not to G findings. | `.gitignore:2` `/.local/`; `git ls-files \\| grep -c '^.local'` = 0 |",
    "line": 3
  },
  {
    "file": "crates/connectors-host/src/federation.rs",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| F07 | minor | inferred | `crates/connectors-host/src/federation.rs:149` | The federation descriptor advertises `configuration_schema: {\"type\":\"object\"}` instead of the `FederationConfig` schema; the wire contract says `/v1/describe` returns the \"configuration schema\" (`contracts/service/v1alpha1/semantics.md`, Wire boundary) and the host derives real schemas for `ServiceConfig`/`HttpConfig`/`CredentialRef` (`crates/connectors-host/src/schema.rs:22-24`). | source line: `configuration_schema: json!({\"type\":\"object\"}),` |",
    "line": 149
  },
  {
    "file": "crates/connectors-host/src/server.rs",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| F08 | minor | inferred | `crates/connectors-host/src/server.rs:137-148`, `:184-206` | The 32-slot semaphore and 20 s deadline bound adapter execution only; `axum::serve` has no header/body read timeout or connection cap, so idle or slow connections are unbounded. Acceptable for the loopback profile the README describes, but the contract's \"32 concurrent operations\" is not a connection bound. | `try_acquire()` at `:139`; `axum::serve(listener, router(…)).with_graceful_shutdown(…)` at `:190-204` with no `timeout` layer |",
    "line": 137
  },
  {
    "file": "adapters/sql/tests/protocol.rs",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| F09 | minor | inferred | `adapters/sql/tests/protocol.rs:143-189`; `crates/connectors-core/tests/wire.rs:5-23` | Test quality: the PostgreSQL protocol fixture answers any Parse with a fixed description and never inspects the wrapped statement, so the `SET LOCAL statement_timeout/lock_timeout/search_path` batch, the `CASE … ARRAY[…]` wrapper and text-parameter encoding are asserted by no automated test (only `READ ONLY` at `:147-149`); the live run is the only cover, which is how F02 stayed unnoticed. `wire.rs` round-trip asserts only `request_id`, not the outcome. Untested code paths: federation cycle refusal (`federation.rs:122-129`), duplicate/`__` route names (`:55-60`), 32-slot capacity (`server.rs:137-140`), 20 s deadline (`:141`), 4 MiB result bound (`:155-161`), `hosts.discover` disabled refusal (`adapters/kubernetes/src/lib.rs:288-293`), endpoint expansion cap (`:253-258`), `read_config` 1 MiB bound (`crates/connectors-host/src/lib.rs:13`). | fixture: `b'P' => { prepares += 1; … send(&mut stream, b'1', b\"\") }` ignores `body`; `b'D' … description(prepares > 1)` |",
    "line": 143
  },
  {
    "file": "crates/connectors-client/src/lib.rs",
    "category": "legacy-review",
    "severity": "note",
    "message": "| F10 | nit | inferred | `crates/connectors-client/src/lib.rs:175` | `parse_error` decodes error bodies with plain `serde_json::from_slice`, not the duplicate-key-refusing `read_json` used for every other envelope. | `serde_json::from_slice(bytes).unwrap_or_else(…)` |",
    "line": 175
  },
  {
    "file": "crates/connectors-sdk/src/lib.rs",
    "category": "legacy-review",
    "severity": "note",
    "message": "| F11 | nit | inferred | `crates/connectors-sdk/src/lib.rs:33-34` | `Secret(pub Vec<u8>)` documents \"no Debug or Serialize\" but the field is public, so any holder can print it; the guarantee rests on convention. | `pub struct Secret(pub Vec<u8>);` |",
    "line": 33
  },
  {
    "file": "crates/connectors-spec/src/v2.rs",
    "category": "legacy-review",
    "severity": "note",
    "message": "| F12 | nit | inferred | `crates/connectors-spec/src/v2.rs:374-380` | `import()` unwraps the operation and field lookups; safe after `Spec::parse` but `Spec` has all-`pub` fields and `import` is `pub`, so a caller constructing a `Spec` directly can panic the compiler. Library-misuse only. | `.find(\\|o\\| o.id == mapping.operation).unwrap();` `.find(\\|f\\| f.name == *name).unwrap();` |",
    "line": 374
  }
]
```

