---
format: aep.planning-md/1
id: review-result:independent-repository-b-20260908
kind: review-result
status: active
title: Independent repository review B at 8903166
relations:
- reviews: specification:contract-driven-connectors-design
revision: 1
---
ship with fixes — the code, docs and planning store are consistent and honest, but the README gate is nondeterministic (a fork/exec race in one toolchain test failed 5 of 11 multi-threaded runs) and the SQL adapter's "mandatory database statement deadline" is bypassable by the caller's own query, with no server-side cancellation.

Reviewer: reviewer-b. Repository HEAD `8903166`, branch `main`, tree clean before and after (git status: 0 lines). Toolchain: rustc 1.98.1, cargo 1.98.1, Rust 1.88.0 installed; `ess` on PATH is 0.18.0, pin is 0.20.0 (resolved by the repository's own resolver from its checkout cache, not opened by me); `aep` protocol 0.54.0.

## Commands run

All with `TMPDIR=<scratch>/tmp CARGO_TARGET_DIR=<scratch>/target CARGO_BUILD_JOBS=6`, scratch = `/home/timo/beyond10x/connectors_v2/.local/independent-review/reviewer-b`. Log paths are relative to `<scratch>/logs/`.

| # | command | exit code | wall | log |
|---|---|---|---|---|
| 1 | `cargo build --workspace --locked` | 0 | 28 s | `01-cargo-build.log` |
| 2 | `cargo test --workspace --locked` | 101 (stopped at `connectors-spec --lib`: 3 passed, 1 failed; 14 later test targets not run) | 22 s | `02-cargo-test.log` |
| 3 | `cargo run --locked -p connectors-build -- gate` (README command, `--msrv` omitted, see F03) run 1 | 1 (failed at `cargo test` step, same test) | 18 s | `03-gate.log` |
| 4 | same, run 2 | 1 (same test) | 7 s | `04-gate-rerun.log` |
| 5 | `cargo test --locked --offline -p connectors-spec --lib` ×6 default threads, ×3 `--test-threads=1` | 1 of 6 parallel runs failed (`Text file busy (os error 26)`); 0 of 3 serial | 8 s | `05-flake-experiment.log` |
| 6 | `cargo +1.88.0 check --workspace --all-targets --locked --offline` (the gate's `--msrv` step, run outside the gate) | 0, 0 warnings | 21 s | `06-msrv-check.log` |
| 7 | `cargo run --locked --offline -p connectors-spec -- --generate --specification adapters/gitlab/spec/adapter.json --output adapters/gitlab/generated --check` | 0, "adapter bundle matches", tree still clean | 4 s | `07-gitlab-regen-check.log` |
| 8 | `cargo test --workspace --locked --no-fail-fast` | 101: 38 passed, 1 failed (same test), 0 ignored | 14 s | `08-cargo-test-no-fail-fast.log` |
| 9 | gate, run 3 | 0: 39 passed / 0 failed / 0 ignored; fmt, 3 descriptors match, clippy `-D warnings`, 3 library boundaries + CLI boundary hold; `ess specify validate`: "connectors v1 — 4 file(s), valid"; compile: 43 declarations; `verify conform synthesize`: "67 scenario(s) (15 authored), 0 refusal(s)"; `aep plan artifact validate`: 52 artifacts, valid, 13 prose-only-findings warnings | 77 s | `09-gate-run3.log` |
| 10 | live SQL check: `postgres:17-alpine` (PostgreSQL 17.11) in Docker, `target/debug/connectors-sql` from scratch target, `target/debug/connectors invoke` ×4, `pg_stat_activity` reads, SIGTERM, container removed | script 0; control `pg_sleep(30)` → `timeout` at 10.1 s and 0 active backends; bypass query → adapter `timeout` at 15.0 s, backend `active` at 18 s, 33 s, and after adapter exit (exit 0 on SIGTERM) | 51 s | `10-sql-live-timeout-check.log` |

Flaky-test tally across all multi-threaded runs of the `connectors-spec` lib test binary this session: 5 failures / 11 runs (rows 2, 3, 4, 5×1, 8 failed; 5×5, 9 passed); 0 / 3 serial.

## Findings

| id | severity | verified/inferred | file:line | statement | evidence |
|---|---|---|---|---|---|
| F01 | major | verified | `crates/connectors-spec/src/toolchain.rs:136-146`, `:182-198`, `:54-58`; `crates/connectors-build/src/gate.rs:37` | `missing_pin_reports_record_and_search_locations_and_cache_is_supported` is flaky: each of the 3 toolchain tests writes a shell script with `std::fs::write` and then `exec`s it via `Command::output()`; a sibling test's fork can hold the write fd open across the exec window, producing ETXTBSY. The README gate runs `cargo test --workspace`, so the gate fails nondeterministically (2 of 3 gate runs here). `docs/verification.md:263-268,289-293` and README present the gate as the reproducible acceptance. | `03-gate.log`: `Searched: …/cache/ess: Text file busy (os error 26); …/wrong/ess: found ess 99.0.0`; `05-flake-experiment.log`: `parallel run 4: … Text file busy`; serial runs 3/3 ok; row 9 passed only on the third gate attempt |
| F02 | major | verified | `adapters/sql/src/lib.rs:119` (`SET LOCAL statement_timeout = '10s'`), `:183-211` (per-row `query_portal`, each Execute re-arms the timer from the current GUC), `:222-227` (`ConnectionTask` drop only aborts the socket task), `:289-291` (15 s adapter timeout); contract `contracts/service/v1alpha1/semantics.md` "Database-enforced read-only transactions and statement timeouts are mandatory" and "SQL cancellation includes a database statement deadline and connection cleanup" | An authenticated caller's query can disable the database statement deadline from inside the read-only transaction (`set_config('statement_timeout','0',true)` is USERSET and not a write), and when the adapter's 15 s timeout then fires nothing cancels the server-side statement (no `CancelToken`/`cancel_query`); the backend stays active after the adapter gives up and even after the process exits. Each such request pins one backend for the query's duration; 32 slots × repeated calls exhausts `max_connections`. The contract's "mandatory" claim is false as written. | `10-sql-live-timeout-check.log`: `sleep: exit=1 wall=10.1s … "code":"timeout"` / `active backends after control: 0`; `bypass: exit=1 wall=15.0s … "database request timed out"`; `backend after bypass: 97\|active\|18\|SELECT CASE WHEN (COALESCE(octet_length(result.c0::text)…`; `backend +15s: 97\|active\|33`; `backend after adapter exit: 97\|active\|33` |
| F03 | minor | verified | `crates/connectors-build/src/gate.rs:118-124` | `--msrv` unconditionally sets `CARGO_TARGET_DIR` to `<root>/target/msrv`, overriding the caller's `CARGO_TARGET_DIR`; the README command cannot be run against a private target directory, and it writes into the shared `target/` even when the rest of the gate does not. | `.env("CARGO_TARGET_DIR", root.join("target/msrv"))` at `gate.rs:124`; I ran the equivalent check separately (row 6, exit 0) |
| F04 | minor | verified | `adapters/gitlab/upstream/README.md:28`; `docs/design.md:1247`; `docs/design.md:3` | ESS-version drift after the 0.20.0 upgrade: the upstream README says `ess-import.json` "retains the complete ESS 0.9.2 refusal", design §28 says GitLab "lowers … into ESS 0.9.2", and the design status line stops at "sections 27–28" although §29–30 exist; the committed bundle was regenerated with 0.20.0. | `adapters/gitlab/generated/manifest.json`: `"ess":"ess 0.20.0"`; `docs/gitlab-generation.md:80-92` describes the 0.20.0 regeneration changing `ess-import.json`; row 7 confirms the bundle matches the 0.20.0 pin |
| F05 | minor | verified | `docs/design.md:1283-1285`; `contracts/README.md:9-31,33` | Contract index counts disagree: design §30 says "14 proposed … 4 families deferred"; `contracts/README.md` lists 16 proposed rows over 16 proposed documents (17 `semantics.md` files, 1 implemented) and 3 deferred families (`execution`, `events`, `resources`); `contracts/service/v1alpha2/semantics.md` (committed in `f493827`) is absent from the index (0 mentions). `docs/stack-integration-proposal.md:44` defers the index row to a draft story, so the index is knowingly stale. | `ls contracts/*/*/semantics.md contracts/*/*/*/semantics.md \| wc -l` = 17; `grep -c v1alpha2 contracts/README.md` = 0 |
| F06 | minor | verified | `docs/stack-integration-proposal.md:3-4`; `contracts/service/v1alpha2/semantics.md:6` | Both documents ground their gaps G1–G10 in `.local/review/2026-09-08-concept-and-stack-integration-review.md`, an ignored untracked file, so the basis of the v1alpha2 proposal cannot be audited from the tree. The planning store's own rule (`.engineering/planning/specification/contract-review-intake-20260908.md`: reviews are preserved "so the plan does not depend on ignored workspace files") was applied to F/E findings but not to G findings. | `.gitignore:2` `/.local/`; `git ls-files \| grep -c '^.local'` = 0 |
| F07 | minor | inferred | `crates/connectors-host/src/federation.rs:149` | The federation descriptor advertises `configuration_schema: {"type":"object"}` instead of the `FederationConfig` schema; the wire contract says `/v1/describe` returns the "configuration schema" (`contracts/service/v1alpha1/semantics.md`, Wire boundary) and the host derives real schemas for `ServiceConfig`/`HttpConfig`/`CredentialRef` (`crates/connectors-host/src/schema.rs:22-24`). | source line: `configuration_schema: json!({"type":"object"}),` |
| F08 | minor | inferred | `crates/connectors-host/src/server.rs:137-148`, `:184-206` | The 32-slot semaphore and 20 s deadline bound adapter execution only; `axum::serve` has no header/body read timeout or connection cap, so idle or slow connections are unbounded. Acceptable for the loopback profile the README describes, but the contract's "32 concurrent operations" is not a connection bound. | `try_acquire()` at `:139`; `axum::serve(listener, router(…)).with_graceful_shutdown(…)` at `:190-204` with no `timeout` layer |
| F09 | minor | inferred | `adapters/sql/tests/protocol.rs:143-189`; `crates/connectors-core/tests/wire.rs:5-23` | Test quality: the PostgreSQL protocol fixture answers any Parse with a fixed description and never inspects the wrapped statement, so the `SET LOCAL statement_timeout/lock_timeout/search_path` batch, the `CASE … ARRAY[…]` wrapper and text-parameter encoding are asserted by no automated test (only `READ ONLY` at `:147-149`); the live run is the only cover, which is how F02 stayed unnoticed. `wire.rs` round-trip asserts only `request_id`, not the outcome. Untested code paths: federation cycle refusal (`federation.rs:122-129`), duplicate/`__` route names (`:55-60`), 32-slot capacity (`server.rs:137-140`), 20 s deadline (`:141`), 4 MiB result bound (`:155-161`), `hosts.discover` disabled refusal (`adapters/kubernetes/src/lib.rs:288-293`), endpoint expansion cap (`:253-258`), `read_config` 1 MiB bound (`crates/connectors-host/src/lib.rs:13`). | fixture: `b'P' => { prepares += 1; … send(&mut stream, b'1', b"") }` ignores `body`; `b'D' … description(prepares > 1)` |
| F10 | nit | inferred | `crates/connectors-client/src/lib.rs:175` | `parse_error` decodes error bodies with plain `serde_json::from_slice`, not the duplicate-key-refusing `read_json` used for every other envelope. | `serde_json::from_slice(bytes).unwrap_or_else(…)` |
| F11 | nit | inferred | `crates/connectors-sdk/src/lib.rs:33-34` | `Secret(pub Vec<u8>)` documents "no Debug or Serialize" but the field is public, so any holder can print it; the guarantee rests on convention. | `pub struct Secret(pub Vec<u8>);` |
| F12 | nit | inferred | `crates/connectors-spec/src/v2.rs:374-380` | `import()` unwraps the operation and field lookups; safe after `Spec::parse` but `Spec` has all-`pub` fields and `import` is `pub`, so a caller constructing a `Spec` directly can panic the compiler. Library-misuse only. | `.find(\|o\| o.id == mapping.operation).unwrap();` `.find(\|f\| f.name == *name).unwrap();` |

No blocker. Counts: blocker 0, major 2, minor 7, nit 3.

Note on F02 remediation surface (not a finding): `tokio_postgres::Client::cancel_token()` exists and is unused; PostgreSQL's `client_connection_check_interval` is 0 by default, so a closed socket is not noticed inside `pg_sleep`.

## Coverage

| area | files read | result |
|---|---|---|
| AGENTS.md, README.md, Cargo.toml, .gitignore | all | README commands reproduce (rows 1, 7, 9) except the gate's nondeterminism (F01) and `--msrv` target override (F03); dependency-boundary claims hold (row 9 `cargo tree` checks) |
| docs/design.md | §1–30 | internally consistent; open decisions marked in §24 with defaults; v1alpha2 proposal does not contradict v1alpha1 (it is a version bump with an explicit projection rule, `contracts/service/v1alpha2/semantics.md` §4 rule 10, §7); stale version/count references only (F04, F05) |
| crates/connectors-core | `src/lib.rs`, `tests/wire.rs` | strict JSON (duplicate keys refused at every depth), canonicalization independent of `serde_json` map order, `deny_unknown_fields` everywhere; no issues |
| crates/connectors-contracts | `src/lib.rs` | plain typed DTOs; no issues |
| crates/connectors-sdk | `src/lib.rs`, `tests/cursor.rs` | HMAC cursors bound to context digest and expiry, ephemeral key per process; `verify_handlers` refuses duplicates; only F11 |
| crates/connectors-client | `src/lib.rs` | https-only unless `allow_plaintext`, no redirects, no proxy, 20 s/5 s timeouts, 4 MiB bounded stream, request-id correlation checked; only F10 |
| crates/connectors-host credentials/http/schema | `src/credentials.rs`, `src/http.rs`, `src/schema.rs`, `tests/http.rs` | `O_NOFOLLOW`, mode `0o077` and euid checks, 8 KiB bound; upstream URL requires scheme/no userinfo/query/fragment, segments individually encoded (no traversal or SSRF from input), auth header marked sensitive, framing headers refused as credential header, custom CA replaces built-in roots (test covers both roots); no issues |
| crates/connectors-host server/federation | `src/server.rs`, `src/federation.rs`, `tests/service.rs` | constant-time token compare, revision checked before dispatch and re-checked in `invoke_at` (no TOCTOU against refresh), refresh never replays, snapshot swap atomic under `RwLock<Arc<_>>` + tokio mutex; log line escapes injected identifiers (test); F07, F08 only |
| adapters/gitlab | `src/lib.rs`, `src/main.rs`, `generated/runtime.rs`, `tests/*.rs`, `spec/adapter.json`, `upstream/README.md`, generated manifest/coverage/ess-import | allowlist before dispatch, `file_path` passed as one encoded segment, non-advancing pagination refused, cursors bound to instance/revision/project/limit; regeneration `--check` matches (row 7); compile-fail doctest guards missing bindings; F04 only |
| adapters/kubernetes | `src/lib.rs`, `src/main.rs`, `tests/provider.rs`, `spec/adapter.json` | namespace charset/length validated at config, allowlists before HTTP, 410 → `stale_cursor`, expansion bounded at 4096, discovery never dials; no issues beyond untested paths (F09) |
| adapters/sql | `src/lib.rs`, `src/main.rs`, `tests/protocol.rs`, `spec/adapter.json` | read-only transaction, per-row portal fetch, byte bounds in SQL and in JSON, sanitized error mapping (fixture proves no password/detail leak), TLS via rustls with CA replacement; F02, F09 |
| apps/connectors | `src/main.rs`, `tests/input.rs` | clap derive, strict input JSON, JSON error on stderr with exit 1; no issues |
| crates/connectors-spec | `src/lib.rs`, `src/main.rs`, `src/toolchain.rs`, `src/v2.rs`, `tests/compiler.rs`, `tests/generation.rs`, `toolchain.json` | v1/v2 validation is conservative (refuses unmapped/required/constrained cases, tests enumerate 7+7 refusals), output ownership via manifest, symlink refusal, preflight before mutation; F01, F12 |
| crates/connectors-build | `src/main.rs`, `src/gate.rs` | `inside()` prevents declaration path escape, package output must be new, docker build `--network=none`; F03 |
| crates/connectors-conformance | `src/main.rs` | live runner only; uses generic client; not executed here (see Not verified) |
| spec-kinds/adapter v1, v2 | `schema.json`, `semantics.md` ×2 | schemas closed; semantics match compiler behavior (`v2.rs`), shared `urn:connectors:config:v1:*` imports resolve offline only |
| adapters/*/generated/descriptor.json | 3 | match compiled specs (gate rows 3, 4, 9: "descriptor matches") |
| ess/ | `system.yaml`, `domains/{declarations,mutations,idempotency}.yaml` | 4 files valid, 43 declarations, UNMAPPED markers present where the contracts say so; 15 authored scenarios compile with 0 refusals (row 9) |
| examples/ | 9 files | match current config types (`deny_unknown_fields` structs); no credentials |
| contracts/service v1alpha1, v1alpha2 | both | v1alpha1 limits match code constants (64 KiB, 4 MiB, 32, 20 s, 15 s/5 s, 10 s/2 s, 1–100, 1000, 8 KiB, 300 s) except F02's timeout guarantee; v1alpha2 is marked proposed and advertised by no descriptor |
| contracts/operations v1alpha1 + verification + idempotency-verification + 15 scenarios | all | code line citations into `core/sdk/server` checked and accurate; documents separate "compiled trace" from runtime proof explicitly |
| contracts/auth ×6, datasources ×3, discovery ×2, media, sessions, catalog | status headers, UNMAPPED counts, §10 open-decision sections | all marked "proposed, not implemented", each has an open-decisions section |
| docs/verification.md, live-e2e.md, gitlab-generation.md | all | test counts 14/24/39 match the tracked evidence logs (`docs/evidence/*`); "zero failures" is true of the recorded runs but not reproducible now (F01); ESS-version drift (F04) |
| docs/stack-integration-proposal.md, cli-migration-v1-to-v2.md | all | consistent with v1alpha2 and with each other; non-parity items cite the boundary; F06 |
| docs/adapters ×6 | status headers + scope sections | all "design, not implemented" (Kubernetes: extension of implemented base); none advertised by descriptors |
| .engineering/planning | `project.yaml`, 2 specifications, epic, decision-blocker, 5 implementation stories in full, 27 contract stories' status/edges, 16 review results (listing), journal tail | honest: 5 implementation stories `implemented` with evidence paths that exist; 2 hardening stories `implemented`, 25 `draft`; epic `active`; dependency edges acyclic; historical records (`v2.rs:8` pin citation) are status `cleared` and dated; `aep plan artifact validate` = valid with 13 prose-only warnings the epic itself discloses |

## Not verified

| item | why |
|---|---|
| Live GitLab, Kubernetes (k3s) and federated acceptance (`connectors-conformance`), `docs/live-e2e.md` end to end | needs public-network GitLab and a privileged k3s container; not re-run; the recorded evidence JSON under `docs/evidence/` was read, not reproduced |
| `connectors-build package` (Docker image, ESS build/realization) | not run; would write a package directory and a Docker image |
| PostgreSQL TLS path (`adapters/sql/src/lib.rs:79-99`) | only the root-store unit test ran; the live check used `allow_plaintext` like the recorded run |
| Gate `--msrv` through the gate binary | run separately (row 6) because of F03 |
| `cargo audit` claim (`docs/verification.md:254`) | audit evidence is under the banned `docs/evidence/review-2026-09-08/`; not rerun |
| The 35-test remediation gate and 17-finding disposition | `docs/review-response-2026-09-08.md` and `docs/evidence/review-2026-09-08/` are banned |
| Gaps G1–G10 behind the v1alpha2 proposal | source is `.local/review/…` (banned and untracked) |
| Cross-machine reproducibility of the generated bundle | verified only on this host with the pinned ESS and rustfmt 1.9.0-stable |

## Incidents

none. Gate output printed directory names under `.local/tmp/gate-*` (the gate's own temp-dir names in `ess` argument lists and in the flaky test's error string); no file under a banned path was opened or read.
