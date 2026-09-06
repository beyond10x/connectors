---
format: aep.planning-md/1
id: review-result:cli-rate-adversary-1-20260906
kind: review-result
status: active
title: Complete rate protocol adversary pass 1
relations:
- reviews: story:rate-limit-in-the-protocol
revision: 1
---
unit: rate-limit-in-the-protocol adversary pass 1; frozen 90735eacf44a96b8a2d8f9f18955867a0bc73dbf
verdict: red
cases: executed root 1199→1207; runtime 343→346; CLI 129→130; console 87→87, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 81 scratch files enumerated in ~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/outside-inventory.txt; assigned TMPDIR descendants in ~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/temporary-paths.txt
needs-coordinator: yes; route the two URL contract mismatches to the implementation owner, then record this report verbatim and retain the test patch

```text
$ git --no-pager diff --stat
 .../catalog-build/tests/main/catalog_invariants.rs |  46 +++++++
 crates/catalog/tests/main/pack_table.rs            |  40 ++++++
 crates/connector-spec/tests/main/ir_roundtrip.rs   |  27 ++++
 crates/connectors-cli/tests/one_shot_operations.rs | 109 +++++++++++++++
 crates/integration-catalog/src/tests.rs            |  65 +++++++++
 crates/integration-slack/src/backend_tests.rs      |  85 ++++++++++++
 crates/server/src/egress_rate_adversary_tests.rs   |  65 +++++++++
 .../server/src/hosted/tests/contract_validation.rs | 150 +++++++++++++++++++++
 crates/server/src/hosted/tests/mcp.rs              |  73 ++++++++++
 9 files changed, 660 insertions(+)
```

Git’s stat excludes three untracked test files: `crates/protocol/tests/rate_adversary.rs` (69 lines), `crates/server/tests/rate_adversary_local.rs` (111), and `crates/connectors-runtime/tests/rate_adversary_registry.rs` (162). The complete patch has 12 authorized test files, 1002 insertions, no deletions. No production, manifest, dependency, model, AEP, Git, worktree-management, integration or provider mutations were performed. The coordinator’s egress registration is already in the frozen HEAD.

Complete patch: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/tests.patch` (SHA256 `beb4974cc383aead748452647713003471a755e02b3e53c3e222954495d2f3aa`). The insertion-only audit preserves every original test line and verifies all 400 original Rust files outside the authorized edits and all 81 stage2 inventory hashes: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/source-integrity.json`. `git diff --check` exited 0.

## 1. Deciding cases

Each named case was written before its first execution, selected alone, and run before its affected full suite. The table counts actual matched runner cases, not parameter iterations. The two red cases failed on their first execution; neither assertion was weakened. Final formatting changes line numbers, so original panic locations remain as captured below.

| Deciding label / file | Measurement | Current passed / failed; exit |
|---|---|---:|
| `deciding-protocol`; `crates/protocol/tests/rate_adversary.rs` | Published v2 Describe/schema versus strict reader; v1 loss for absent/zero/max delay. | 0 / 1; 101 |
| `deciding-spec`; `crates/connector-spec/tests/main/ir_roundtrip.rs` | Fixed JSON unchanged; 16 Unicode/unknown-rate alternatives round-trip, 17 refuse. | 1 / 0; 0 |
| `deciding-canonical`; `crates/catalog-build/tests/main/catalog_invariants.rs` | Actual pipeline Slack document/schema3 versus public authoring reader URL mutations. | 0 / 1; 101 |
| `deciding-pack`; `crates/catalog/tests/main/pack_table.rs` | Every packed operation retains fixed/conditional advice in the typed table, including all Slack alternatives. | 1 / 0; 0 |
| `deciding-local-fixture-fixed`; `crates/server/tests/rate_adversary_local.rs` | Real owner-only Unix daemon: both versions, invalid context/unknown version, malformed backend response, exact dispatch count. | 1 / 0; 0 |
| `deciding-egress`; `crates/server/src/egress_rate_adversary_tests.rs` | Real HTTP duplicate/ambiguous headers and unfinished oversized body: definite 429 differs from 200 body failure. | 1 / 0; 0 |
| `deciding-hosted`; `crates/server/src/hosted/tests/contract_validation.rs` | Actual /operations, real Grant/Approval records, both identities, early refusals and admitted mutating Invoke. | 1 / 0; 0 |
| `deciding-mcp`; `crates/server/src/hosted/tests/mcp.rs` | Actual /mcp absent/zero/max rate refusal; stale-then-rate allows exactly the one stale retry. | 1 / 0; 0 |
| `deciding-registry`; `crates/connectors-runtime/tests/rate_adversary_registry.rs` | Public registry merges equal advice, refuses incompatible contributors, invalidates old leases, dispatches fresh lease once. | 1 / 0; 0 |
| `deciding-catalog-adapter`; `crates/integration-catalog/src/tests.rs` | Bound generic adapter refuses unknown binding, stale lease and bad input before egress/rate disclosure; valid 429 stays typed. | 1 / 0; 0 |
| `deciding-slack`; `crates/integration-slack/src/backend_tests.rs` | Configured mutating Slack adapter: two explicit zero-delay refusals produce attempted/refused twice, one egress each. | 1 / 0; 0 |
| `deciding-cli`; `crates/connectors-cli/tests/one_shot_operations.rs` | Real CLI JSON/YAML: absent/zero/max delay, protocol/uncertain errors, wrong version/correlation; listener counts until CLI exits. | 1 / 0; 0 |

Exact case names, cwd, complete commands, environments, timestamps, exit codes and verbatim runner summaries are in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/command-evidence.md`; machine-readable originals are `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/commands.jsonl`. Each deciding label identifies its retained `.log` and `.exit` file in the fully enumerated inventory.

Verbatim first red output, excluding successful compiler lines:

`~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/deciding-protocol.log`; exit 101:

```text
thread 'rate_adversary_published_schema_matches_source_url_reader' (519560) panicked at crates/protocol/tests/rate_adversary.rs:38:5:
source-URL validity is not one of the two documented schema limitations:
"HTTPS://docs.example.test/rate": Rust=true, schema=false
"https://docs.example.test:65536/rate": Rust=false, schema=true
"https://docs.example.test/a b": Rust=true, schema=false
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test rate_adversary_published_schema_matches_source_url_reader ... FAILED

failures:

failures:
    rate_adversary_published_schema_matches_source_url_reader

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p protocol --test rate_adversary`
```

`~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/deciding-canonical.log`; exit 101:

```text
thread 'catalog_invariants::rate_adversary_canonical_source_urls_match_authoring_reader' (592143) panicked at crates/catalog-build/tests/main/catalog_invariants.rs:2535:5:
the public authoring reader and generated schema3 disagree:
"HTTPS://docs.example.test/rate": authoring=true, schema3=false
"https://docs.example.test:65536/rate": authoring=false, schema3=true
"https://docs.example.test/a b": authoring=true, schema3=false
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test catalog_invariants::rate_adversary_canonical_source_urls_match_authoring_reader ... FAILED

failures:

failures:
    catalog_invariants::rate_adversary_canonical_source_urls_match_authoring_reader

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 10.93s

error: test failed, to rerun pass `-p catalog-build --test main`
```

The first local attempt (`~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/deciding-local.log`) exited 101 before running any case: the new fixture omitted the required `ready` trait method and referenced unavailable `tempfile`. Only that new fixture was repaired, using the existing trait and assigned TMPDIR without adding dependencies. `deciding-local-fixture-fixed` then ran exactly one case and passed. This compiler failure is not a product finding. All other deciding selections executed exactly one case.

## 2. Affected suites and checks

One workspace lane ran at a time, using `TMPDIR=~/.cache/cw6/r/rate-a1`, `RUSTC_WRAPPER=/usr/bin/sccache`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`, and unset `CARGO_TARGET_DIR`. Root cwd was `~/.local/state/worktree/trees/b10x/connectors/wt-af054beacfba`; other cwd values were its `crates/connectors-runtime`, `crates/connectors-cli`, and `crates/connectors-console` directories.

Every lane ran these exact Cargo commands after its deciding cases:

```sh
cargo test --workspace --locked --offline --no-fail-fast
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

| Lane | Executed before → after | Final passed / failed / ignored | Test / clippy / fmt exits |
|---|---:|---:|---:|
| root | 1199 → 1207 | 1205 / 2 / 4 | 101 / 0 / 0 |
| runtime | 343 → 346 | 346 / 0 / 2 | 0 / 0 / 0 |
| cli | 129 → 130 | 130 / 0 / 0 | 0 / 0 / 0 |
| console | 87 → 87 | 87 / 0 / 0 | 0 / 0 / 0 |

Only the two new URL cases fail. Existing case-name multisets and ignored cases are retained. Before counts come from the frozen implementor’s final same-selection logs (`stage2-root-final`, `stage2-runtime-full`, `stage2-cli-full-3`, `stage2-console-full`), not a pre-attack suite run or stage1 counts. Full per-lane output is retained in the four `full-*.log` files; exact paths, parsed counts and baseline references are in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/counts-and-preservation.json`. Full clippy/fmt output and exits are indexed in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/command-evidence.md`. This pass makes no 12-workspace repository-gate or deployment claim.

## 3. Findings, measured reachability and origin

| File:line | Category / severity | Verdict / origin | Finding |
|---|---|---|---|
| `crates/protocol/src/operation/schema.rs:132` | contract-drift / warning | CONFIRMED / introduced | The published v2 source URL schema and strict wire reader disagree for uppercase HTTPS, an out-of-range port, and a literal path space. |
| `crates/catalog-build/src/document_schema.rs:22` | contract-drift / warning | CONFIRMED / introduced | The canonical schema3 source URL rule and public authoring reader disagree for uppercase HTTPS, an out-of-range port, and a literal path space. |

For both boundaries, uppercase HTTPS and a literal path space are accepted by the Rust URL parser but rejected by the published schema; port 65536 is schema-valid but refused by Rust. Normal lowercase HTTPS remains valid in both cases; the wire case also retains the valid explicit port 443 control. These are URL grammar differences beyond v2’s documented arithmetic and serialized-byte limitations.

Wire measurement: the committed valid Describe vector was mutated only in rate advice, then passed to the published Draft 2020-12 schema with formats enabled and `wire::ResponseEnvelope::validate`; the retained assertion is `crates/protocol/tests/rate_adversary.rs:64`. The actual local and hosted clients call that validator through `connectors-client/src/response.rs:17` and `lib.rs:88,489`. Catalog advice reaches Describe through `service/src/rate_limit.rs:10` and the generic/Slack adapter callers. The test measures the public response-reading contract directly; it does not send malformed advice through a live provider.

Authoring measurement: the actual read-only pipeline produced the shipped Slack canonical document, whose source URL was mutated in memory, then compared using the public `connector_spec::ConditionalRateLimit` deserializer and committed schema3; the retained assertion is `crates/catalog-build/tests/main/catalog_invariants.rs:2558`. Authored TOML reaches this reader through `provider/loading.rs:44` and `provider/declaration.rs:746`; publication copies the declaration at `provider/publishing.rs:836`, and canonical rendering validates it at `catalog-build/src/document.rs:596`. This establishes the authoring/schema boundary, not a completed custom-provider compilation with a malformed URL. The current shipped lowercase Slack URLs pass; no deployed malformed provider or credential effect was demonstrated.

Origin is established by read-only base-object diffs: stage1 `837f7e08ad0e3f8cb5c9531218c7904d20e0ccab` adds the wire reader/schema definitions against whole-unit base `e101a24e73fe22a58cb67e0a2e7ac95a7b83b688`; stage2 `b4dcc8155fbff66cae20de2d46c0ef87072a1b5b` adds authoring/schema3 definitions against combined baseline `fe5da04ab9ea0a71fc1a75194a87cddd88f46cb2`. The reviewed prerequisite bulk is distinct. Exact diffs and caller excerpts are retained in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/finding-origin.txt` and `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/finding-reachability.txt`. No base checkout was modified or executed.

These are bounded contract warnings. The red header records the two failing adversary cases; it does not assert a deployed operational blocker or architecture acceptance. The implementation owner should make URL spelling/range policy consistent across both schema generators and both readers, keeping the deciding cases intact.

## 4. Boundaries not broken

- All 65 canonical documents and 1011 operations match the combined baseline after removing only the intended three Slack history alternatives; vendor schemas and all other fields remain equal. Frozen v1 bundle bytes and canonical schema2 bytes are unchanged. The separate read-only comparison is reproducible with `audit.py` and retained in `source-preservation.json`.
- Fixed declaration serialization, 16/17 alternative bounds, Unicode applicability, absent numeric advice, and complete pack/table projection passed the new cases and retained suites.
- Real Unix transport, hosted Grant/Approval refusal ordering, v1 loss and v2 integer extremes, registry identity, MCP stale-only retry and CLI request counts passed the selected matrices.
- The HTTP case directly reaches the production response reader over loopback HTTP; it does not exercise TLS/destination admission. It proves duplicate cardinality before flattening and prompt definite-429 handling while the fixture body remains unfinished.
- Generic and curated adapter cases use configured fixture bindings and credential stores through current invocation owners. New cases cover refusal ordering and explicit zero-delay writes; retained suites cover other delay/uncertainty branches. No new terminal-audit-write-failure injection, live connector call or complete personal one-shot provider egress was performed.

## 5. Retained paths and mechanical public copy

Every scratch file is enumerated by full absolute path in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/outside-inventory.txt`; all retained test-created TMPDIR descendants are enumerated in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/temporary-paths.txt`. Transient test files used that same assigned namespace. Full commands are in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/commands.jsonl`. The four compiler targets remain in the supplied worktree and are listed in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/resource-record.json`. The authorized existing sccache cache received ordinary compiler-cache writes. No installation, target cleanup, cache cleanup, worktree retirement or other cleanup was performed. Every recorded build started above 20 GiB; final free space was 22234112000 bytes.

Frozen raw report: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/raw-report.md`. Portable public report: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/report.md`. The public copy replaces only the absolute local home-directory prefix with `~`; all other prose, findings, commands and counts are unchanged. Paths are plain code references, with no local Markdown links. Hash manifest: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-1/SHA256SUMS`. The original stage2 reports and coordinator erratum remain unchanged; this pass used the final green stage2 counts clarified by that erratum.

```findings
- file: crates/protocol/src/operation/schema.rs
  line: 132
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The published v2 source URL schema and strict wire reader disagree for uppercase HTTPS, an out-of-range port, and a literal path space.
- file: crates/catalog-build/src/document_schema.rs
  line: 22
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The canonical schema3 source URL rule and public authoring reader disagree for uppercase HTTPS, an out-of-range port, and a literal path space.
```
