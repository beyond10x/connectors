---
format: aep.planning-md/1
id: review-result:independent-repository-a-20260908
kind: review-result
status: active
title: Independent repository review A at 8903166
relations:
- reviews: specification:contract-driven-connectors-design
revision: 1
---
ship with fixes — build, tests, README gate (incl. Rust 1.88.0), GitLab regeneration and cargo audit all pass at HEAD 8903166; no code defect found; 8 minor doc/planning inconsistencies and 4 nits need small edits.

Reviewer: reviewer-a. Repository: /home/timo/beyond10x/connectors_v2, branch main, HEAD 8903166. Toolchain observed: rustc 1.98.1, cargo 1.98.1, rustup 1.88.0 present, `ess` on PATH = 0.18.0 (pin 0.20.0 resolved by the repository resolver), aep protocol 0.54.0, cargo-audit 0.22.0.

## Commands run

All with `TMPDIR=.local/independent-review/reviewer-a/tmp CARGO_TARGET_DIR=.local/independent-review/reviewer-a/target CARGO_BUILD_JOBS=6`. Logs under `/home/timo/beyond10x/connectors_v2/.local/independent-review/reviewer-a/logs/`.

| command | exit | wall | log |
|---|---|---|---|
| `cargo build --workspace --locked` | 0 | 19 s | cargo-build.log |
| `cargo test --workspace --locked` (39 tests + 1 compile-fail doctest, 0 failed, 0 ignored) | 0 | 39 s | cargo-test.log |
| `timeout 1200 cargo run --locked -p connectors-build -- gate --msrv` (fmt, 3 descriptor drift checks, offline build/test/clippy `-D warnings`, 3 `--no-default-features` lib builds + boundary trees, CLI boundary, `cargo +1.88.0 check --all-targets`, `ess specify validate/compile` = `connectors v1 — 4 file(s), valid` / 43 declarations, `ess verify conform synthesize` = 67 scenarios (15 authored) 0 refusals, `aep plan artifact validate` = 52 artifacts valid with 13 "prose-only findings" warnings) | 0 | 82 s | gate.log |
| `target/debug/connectors-spec --generate --specification adapters/gitlab/spec/adapter.json --output tmp/regen` then `diff -r tmp/regen adapters/gitlab/generated` | 0 / diff 0 (no drift, 22 files) | 2 s | regen.log, regen-diff.log |
| `target/debug/connectors-spec --generate --check --specification adapters/gitlab/spec/adapter.json --output adapters/gitlab/generated` → `adapter bundle matches` | 0 | 2 s | regen-check.log |
| `target/debug/connectors-build --root . check` → `generation and ESS declarations match` | 0 | 2 s | build-check.log |
| `cargo audit` (306 crate dependencies, 0 vulnerabilities, 0 warnings) | 0 | 3 s | cargo-audit.log |

Deviation: the gate's `--msrv` step wrote 13 files into the repository's shared `target/msrv/` (gate.rs:456 hardcodes `CARGO_TARGET_DIR=<root>/target/msrv`); no file under `target/debug/` was touched by this session. See M7.

## Findings

Severity: blocker 0, major 0, minor 8, nit 4.

| id | sev | v/i | file:line | statement | evidence |
|---|---|---|---|---|---|
| M1 | minor | verified | docs/design.md:1247 | §28 still says GitLab "lowers its selected local request types into ESS 0.9.2"; the pin is 0.20.0 | `crates/connectors-spec/toolchain.json` = `{"ess":"0.20.0"}`; `adapters/gitlab/generated/manifest.json` `"ess":"ess 0.20.0"` |
| M2 | minor | verified | adapters/gitlab/upstream/README.md:28 | says `ess-import.json` "retains the complete ESS 0.9.2 refusal"; the committed file is the 0.20.0 refusal | `adapters/gitlab/generated/ess-import.json`: `"refusals": ["/openapi: only OpenAPI 3.1 is supported, found 3.0.0"]`; docs/gitlab-generation.md:293-294 describes this as the 0.20.0 change |
| M3 | minor | verified | docs/design.md:1283-1284; contracts/README.md:31 | design says "14 proposed … 4 families deferred"; the index has 16 `proposed` rows and lists 3 deferred families; `configuration` (design.md:180) is in neither list; `contracts/service/v1alpha2` (proposed) is absent from the index | `grep -c "| proposed" contracts/README.md` = 16; README.md:31 "Deferred families …: `execution`, `events`, `resources`"; `grep -n v1alpha2 contracts/README.md` = no index row. E13/E22 are owned by draft `story:contracts-documentation-index`, but the v1alpha2 row and the 14-vs-16 count post-date that intake (db1c329) |
| M4 | minor | verified | .engineering/planning/story/contracts-mutation-outcomes.md:108; …/contracts-idempotency-scope.md:104 | both `implemented` stories state their changes "remain local and uncommitted"; both are committed | `git log`: 34f298a "docs: harden mutation outcomes…", 8903166 "docs: harden idempotency scope…"; `git show --stat` of each commit includes the respective story file |
| M5 | minor | verified | .engineering/planning/specification/contract-driven-connectors-design.md:38 (also :12) | "No implementation stories are decomposed, no lifecycle approval is asserted" / "does not claim that any adapter has been implemented" while 5 stories `derived_from` this artifact are `implemented` | status grep: three-adapters-e2e, gitlab-spec-service, full-review-remediation, ess-executable-pin, ess-pin-upgrade = `status: implemented`; artifact is `draft`, revision 2 |
| M6 | minor | verified | docs/stack-integration-proposal.md:3-4; contracts/service/v1alpha2/semantics.md:6 | the source of gaps G1–G10 is `.local/review/2026-09-08-concept-and-stack-integration-review.md`, a gitignored path; no tracked artifact preserves those findings (the two other reviews were preserved as review-result artifacts for exactly this reason) | `.gitignore:2` = `/.local/`; `git ls-files \| grep -i concept` = none; intake spec :19 "preserved … so the plan does not depend on ignored workspace files" |
| M7 | minor | verified | crates/connectors-build/src/gate.rs:456; README.md:24 | `--msrv` forces `CARGO_TARGET_DIR=<root>/target/msrv`, ignoring the caller's `CARGO_TARGET_DIR`; README says the gate "uses a task-owned temporary directory under `.local/tmp`" | `find target/msrv -newer logs/cargo-build.log \| wc -l` = 13 after a gate run with `CARGO_TARGET_DIR` set elsewhere; gate.rs:456 `.env("CARGO_TARGET_DIR", root.join("target/msrv"))` |
| M8 | minor | verified | docs/cli-migration-v1-to-v2.md:124 | refers to `story:cli-governed-surface` as the owner of 7 open decisions; no such planning artifact exists | `git grep cli-governed-surface` hits only the two docs; docs/stack-integration-proposal.md:63 "Not created" |
| N1 | nit | verified | crates/connectors-host/src/credentials.rs:51; server.rs:137-161; federation.rs:254-266,328-335; adapters/kubernetes/src/lib.rs:60-62,288-293; adapters/sql/src/lib.rs:79-99 | untested paths: `CredentialRef::Environment` resolution (no test constructs it; `git grep "Environment {" -- '*.rs'` hits only the definition), host `Capacity` (semaphore, result size) and the 20 s deadline, federation connect refusals (duplicate/`__`/cycle), `hosts.discover` disabled, SQL TLS handshake (only `database_roots` unit test at sql lib.rs:330-355), `schema.list` success (live runner only, conformance main.rs:494-507) | grep results above; test list in cargo-test.log |
| N2 | nit | inferred | crates/connectors-host/src/federation.rs:17-20; crates/connectors-client/src/lib.rs:39-45 | `DownstreamConfig` has no `ca_file`, and `Endpoint::new` builds a reqwest client with built-in roots only, so a downstream served over HTTPS with a private CA cannot be federated; provider HTTP has `ca_file` (http.rs:24) | code read; README.md:117 only says "use loopback locally or a trusted TLS-terminating ingress" |
| N3 | nit | verified | contracts/operations/v1alpha1/semantics.md:61 | the Invocation example carries `"version": "v1alpha1"` plus `idempotency_key`/`approval`, which the v1alpha1 strict decoder refuses; §7 (:207) and §10 (:241) take the `v1alpha2` wire | core lib.rs:91 `#[serde(deny_unknown_fields)]` on `Invocation` |
| N4 | nit | verified | contracts/service/v1alpha2/semantics.md:15 | "Unchanged: … error envelope shape" while §3.4 adds `audit_ref` to every response and §3.5 adds two codes; both are refused by v1alpha1 readers (core lib.rs:13-27 closed `ErrorCode`, :120 `deny_unknown_fields` on `Response`). §4 rule 10 makes the behaviour consistent (v1alpha1 clients refused); the summary row is imprecise | file lines cited |

Not findings, recorded so they are not re-raised: verification.md:48/191/253 test counts (14/24/35) are dated historical sections; current count is 39 (gate-020.log and this session agree). design.md:64/915/928 record ESS 0.9.2 as an observation at design time, not the pin.

## Coverage

| area | reviewed | result |
|---|---|---|
| crates/connectors-core (wire types, strict JSON, canonical digest) | lib.rs 1-258, tests/wire.rs | no defect; duplicate keys, non-finite numbers, unknown fields refused; serde_json depth limit protects the Strict visitor |
| crates/connectors-sdk (ports, cursors, schema validation, upstream error mapping) | lib.rs 1-210, tests/cursor.rs | no defect; HMAC cursor bound to context+expiry, ephemeral key; `jsonschema` built without remote resolvers |
| crates/connectors-host: credentials.rs | 1-155 | no defect; O_NOFOLLOW, mode/uid/size checks, `Secret` has no Debug/Serialize, 8 KiB bound |
| crates/connectors-host: http.rs | 1-318 | no defect; no redirects, no proxy, CA replaces built-in roots, framing headers refused as credential header, segments percent-encoded incl. `/`, `.`/`..` refused, `set_sensitive` on the credential header |
| crates/connectors-host: server.rs | 1-206 | no defect; constant-time token compare, 64 KiB body, 32-slot semaphore, 20 s deadline, input/output schema validation, 4 MiB result bound, log line carries only validated ids (escaping tested) |
| crates/connectors-host: federation.rs | 1-192 | no defect; 1–32 downstreams, `__` and duplicate names refused, federation-of-federation refused, atomic snapshot swap under a mutex, no replay on stale, token re-resolved per call |
| crates/connectors-host: schema.rs | 1-82 | no defect; only `urn:connectors:config:v1:*` resolved offline, instance data not walked |
| crates/connectors-client | lib.rs 1-187 | no defect; content-length and streamed 4 MiB bound, correlation/version check, no fallback |
| apps/connectors | main.rs 1-111, tests/input.rs | no defect; clap derive, strict input JSON, errors to stderr + exit 1 |
| adapters/gitlab (lib, main, spec, generated runtime/types/manifest/coverage) | all | no defect; allowlist before dispatch, page advance checks, base64 representation check; regeneration byte-identical |
| adapters/kubernetes | lib.rs 1-307, main.rs, spec | no defect; namespace/kind allowlists before dispatch, 410→stale cursor, 4096-endpoint expansion cap, no dial |
| adapters/sql | lib.rs 1-356, main.rs, spec, tests/protocol.rs | no defect; READ ONLY txn, statement/lock timeouts, fixed search_path, extended-protocol prepare (multi-statement refused), server-side 4 MiB row guard, limit+1 fetch for truncation, sanitized error mapping, connection task aborted on drop |
| crates/connectors-spec (v1 compile, v2 parse/import/lower/render/install, toolchain resolver) | lib.rs, v2.rs 1-913, toolchain.rs, tests | no defect; all `unwrap`/index sites guarded by prior `parse()` checks; symlink/ownership preflight; resolver never falls back from an explicit path |
| crates/connectors-build (check/package/gate) | main.rs 1-332, gate.rs 1-163 | M7 only; path containment via `inside()`; package refuses existing output |
| crates/connectors-conformance (live runner) | main.rs 1-365 | no defect; live-only, not runnable here |
| Dependency boundaries | gate output | provider libs have no host/client/sibling edge; CLI has no adapter edge; no runtime→spec edge |
| Contracts: service/v1alpha1 vs code | limits, auth, TLS, federation, SQL paragraphs | every stated limit matches a constant (64 KiB, 4 MiB, 32, 20 s, 15 s/5 s, 10 s/2 s, 1–100, 1000 rows, 8 KiB, 300 s) |
| Contracts: 16 proposed documents + operations verification docs + 15 scenarios | read in full | internally consistent within the intake's acknowledged 44 open findings; N3, N4 |
| v1alpha2 proposal vs v1alpha1 and mutation profile | full read | no contradiction beyond N4; `v1alpha2` wire choice agrees across operations §7/§10, service/v1alpha2 §4 rule 10, catalog §7 |
| docs/design.md | 1-1288 | holds together; §24 marks open decisions; §27-30 addenda consistent with code except M1, M3 |
| README, docs/gitlab-generation.md, docs/live-e2e.md, docs/verification.md | full read | reproducible claims reproduced (build, test, gate, regeneration, `check`, audit); live claims not reproduced (see below) |
| ess/system.yaml + 3 domains, spec-kinds v1/v2 schema+semantics, examples/ | full read | validate/compile in gate; examples match config structs |
| Planning store (5 implemented stories, 27 contract stories, epic, 2 specs, decision, 14 review-results, journal 493 entries) | full read + status grep | statuses match code (5 implemented, 2 hardening stories implemented, 25 draft, epic active, blocker cleared); M4, M5, M8 |
| Test quality | all 14 test targets | fixture transports are the correct seam; no tautological test found; SQL fixture returns the column OIDs it asserts (plumbing check, not PostgreSQL behaviour); gaps in N1 |

## Not verified

| item | why |
|---|---|
| Live acceptance claims (docs/verification.md live sections, docs/live-e2e.md, docs/gitlab-generation.md container recipe, evidence JSONs `status: passed`) | require Docker, k3s, PostgreSQL and network to gitlab.com; not run. Evidence files exist and parse (`2026-09-08-live-acceptance.json` 9 checks; `live-020.json` 3 checks) |
| docs/verification.md:250-258 (review remediation gate log "35 passing tests", audit "no vulnerabilities") | cites docs/evidence/review-2026-09-08/, a banned path; current-state equivalents were re-run instead (39 tests, audit clean) |
| `connectors-build package` (image build, ESS build/realization) | requires Docker; not run |
| Exact ESS binary used by the resolver | `.local/toolchains/…` is a banned path; inferred from the passing generation tests and gate output `ess 0.20.0` semantics only |
| ESS `verify conform` scenario semantics | ESS compiled 67 scenarios with 0 refusals; scenario correctness against a runtime was not assessable (no runtime exists, as the docs state) |

## Incidents

none — no path under `.local/` other than `.local/independent-review/reviewer-a/` was opened, listed or read; `docs/review-response-2026-09-08.md` and `docs/evidence/review-2026-09-08/` were not opened (their names appeared in `git ls-files` and `git grep` output only). One side effect: the gate's `--msrv` step wrote build metadata into the shared `target/msrv/` (M7); no tracked file was modified (`git status --short` clean, `git stash list` empty).
