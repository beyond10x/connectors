---
format: aep.planning-md/1
id: review-result:cli-rate-adversary-2-20260906
kind: review-result
status: active
title: Final independent rate protocol review
owner: agent:review_one_shot_final
relations:
- reviews: story:rate-limit-in-the-protocol
revision: 1
---
unit: rate-limit-in-the-protocol, second/final ordinary adversary pass, frozen 2f89a6fccf9878ed8275565d68667cbd1fc08d43 plus tests below
verdict: green
cases: executed 1772→1779, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 109 scratch files and one TMP namespace; exact inventory at ~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/outside-inventory.txt
needs-coordinator: yes, record this immutable report and tests, then own integration/AEP/publication and remaining repository gates

```text
 .../catalog-build/tests/main/catalog_invariants.rs | 113 ++++++++++++++++
 crates/connectors-cli/tests/one_shot_operations.rs |  98 ++++++++++++++
 crates/integration-slack/src/backend_tests.rs      | 130 +++++++++++++++++++
 crates/protocol/tests/rate_adversary.rs            |  76 +++++++++++
 crates/server/src/egress_rate_adversary_tests.rs   |  66 ++++++++++
 .../server/src/hosted/tests/contract_validation.rs | 142 +++++++++++++++++++++
 crates/server/tests/rate_adversary_local.rs        | 119 +++++++++++++++++
 7 files changed, 744 insertions(+)
```

1. Deciding cases and disposition

No new or residual product finding. Each of seven new cases was written before execution, selected alone with exactly one executed test, and passed before its affected full suite. Both unchanged pass-1 URL countercases also executed exactly one and passed (`retained-protocol.log`, `retained-canonical.log` in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2`). Their original assertions remain intact. Green describes this bounded routing/test outcome; it is not architecture acceptance.

| Case | Test owner | Passed / failed / ignored | Retained deciding log |
|---|---|---|---|
| protocol | `crates/protocol/tests/rate_adversary.rs` | 1 / 0 / 0 | `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-protocol.log` |
| canonical | `crates/catalog-build/tests/main/catalog_invariants.rs` | 1 / 0 / 0 | `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-canonical.log` |
| egress | `crates/server/src/egress_rate_adversary_tests.rs` | 1 / 0 / 0 | `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-egress.log` |
| local | `crates/server/tests/rate_adversary_local.rs` | 1 / 0 / 0 | `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-local.log` |
| hosted | `crates/server/src/hosted/tests/contract_validation.rs` | 1 / 0 / 0 | `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-hosted.log` |
| slack-selected | `crates/integration-slack/src/backend_tests.rs` | 1 / 0 / 0 | `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-slack-selected.log` |
| cli | `crates/connectors-cli/tests/one_shot_operations.rs` | 1 / 0 / 0 | `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-cli.log` |

Exact case names, parameter matrices and caller limits are in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/case-matrix.json`; exact commands and runner summaries are in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/command-evidence.md` and `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/commands.jsonl`. The initial Slack selector mistakenly named an include module and selected zero tests, exit 0 (`~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/deciding-slack.log`); it is excluded from deciding coverage. The corrected exact selector selected one, passed, and required no source/assertion change. No compiler error or deciding red test was observed. Standalone-format restoration before execution is documented in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/preparation-notes.md`.

2. Affected suites and checks

All four lanes used `cargo test --workspace --locked --offline --no-fail-fast`, ordinary parallel runner execution, followed by `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` and `cargo fmt --all -- --check`. Cwd is `~/.local/state/worktree/trees/b10x/connectors/wt-af054beacfba` for root and its `crates/connectors-<lane>` directory otherwise. Every raw `full-<lane>.log`, `clippy-<lane>.log`, `fmt-<lane>.log` and matching `.exit` is retained in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2`; full runner summaries and exact cwd/argv are in command-evidence.md.

| Workspace | Executed before → after | Passed / failed / ignored | Test / clippy / fmt exits |
|---|---|---|---|
| root | 1209 → 1214 | 1214 / 0 / 4 | 0 / 0 / 0 |
| runtime | 346 → 347 | 347 / 0 / 2 | 0 / 0 / 0 |
| cli | 130 → 131 | 131 / 0 / 0 | 0 / 0 / 0 |
| console | 87 → 87 | 87 / 0 / 0 | 0 / 0 / 0 |

Before counts come from the frozen implementor's final `stage2-repair-<lane>-full.log` reports, not a pre-attack rerun. All existing case-name multisets remain selected, including all six existing ignored tests. Only the seven named new cases increase the totals. See `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/counts-and-preservation.json`. This is the four affected workspaces, not a final twelve-workspace gate claim.

3. What was reached and could not be broken

- protocol: Published complete Describe vector -> ResponseEnvelope::validate -> Version::encode_response; no runtime URL destination authorization claim.
- canonical: Actual provider::load_with_spec with shipped cached specs -> document::render -> full schema3 validation; provider files remain read-only, accepted spelling and complete vendor operation retained.
- egress: Real loopback HTTP -> reqwest parsed status/headers -> actual private read_http_response owner; no DNS/TLS or Connection destination-admission claim.
- local: Actual LocalOperationDaemon, Unix frames and injected backend; four correlated requests, four backend calls, strict validation before v1 advice loss.
- hosted: Actual /operations router and shared admission/projection under fixture verified principal; six requests, four verifier calls and four backend calls. Real Identity/provider services are not contacted.
- slack-selected: Configured fixture Connection and memory credentials -> actual Slack inner.invoke (called by backend.rs:731) -> durable audit.begin -> fixture egress -> failed real audit.finish path; one egress call and retained attempted-only record. Deterministic temporary storage failure injection does not establish a naturally occurring operator failure.
- cli: Actual CLI child process -> Unix fixture -> LocalClient strict response reader -> product JSON/YAML projection; listener remains alive until actual child exit to count every request. No hosted CLI/TLS/provider deployment claim.

Retained full-suite cases additionally cover actual local/hosted early Grant/Approval refusals in both identities, generic and curated 429 mapping, registry identity and description validation, MCP stale-only retry with structured delay, and CLI zero/max/error/wrong-version/wrong-correlation no-resend behavior. No live provider, installation, credential issuance, TLS deployment or model auth-result protocol was exercised or added.

4. Source boundaries and preservation

The review covers stage1 `837f7e08` against whole-unit base `e101a24e`, stage2 `b4dcc815` against prerequisite baseline `fe5da04a`, retained first-pass cases at `cc143696`, and the final repair at `2f89a6fc`; reviewed GitLab/one-shot prerequisite bulk remains distinct. Re-read the active acceptance in integration story revision 135 and the actual changed callers. The complete repair report and all 122 supplied manifest hashes were verified before execution; their exact hashes are retained in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/freeze-verification.json`.

The declared RFC 3986 ASCII URI profile is the oracle: literal lowercase https, nonempty host, no userinfo/fragment, bounded decimal port, empty/default and leading-zero spelling retained, no invented normalization or DNS rule. Full canonical comparisons reproduce 65 documents / 1,011 operations against fe5da04a, removing only intended Slack history conditional metadata. Complete remaining vendor operations, frozen v1 bundle bytes against e101a24e and historical schema2 bytes against fe5da04a are unchanged. All 1,164 tracked files were hashed: only seven assigned test files differ, with insertion-only preservation of every original byte/assertion. No production helper, manifest, dependency, model, AEP, Git or operator mutation was made.

Tests: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/tests.patch`, SHA256 `074a14c8b34081cb700cd47155d6cf416f9b14da6d7b1b3847cda0ddc0271eef`. Exact changed-path hashes: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/source-inventory.txt`. Ownership and byte proof: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/source-integrity.json`; canonical/frozen contract audit: `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/source-preservation.json`. All prior reports and cases remain retained.

5. Resources and outside paths

Builds used jobs 1, `RUSTC_WRAPPER=/usr/bin/sccache`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, no CARGO_TARGET_DIR, and TMPDIR `~/.cache/cw6/r/rate-a2`. Every Cargo command started above the 21474836480-byte floor. Shared space fell below it by the completed CLI deciding run's end (21095084032 bytes), with 20696752128 bytes observed immediately afterward; no rate build was active when detected. Further Cargo commands paused while the coordinator restored capacity, then explicitly authorized cleanup of the inactive pre-review console target after all 122 repair evidence hashes and 1,164 source hashes were reverified; that restored 23174062080 bytes. This observation is retained in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/disk-incident.json` and does not replace any test result. Each completed target was cleaned only after all its logs/checks and source snapshot were frozen and verified outside target, then the same 1,164 source hashes and retained evidence were reverified. Exact authorized cargo-clean commands, targets and before/after values are in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/resources.json` and each `lane-<lane>-clean-verification.json`. No shared cache cleanup.

Every retained scratch path and TMP namespace is listed in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/outside-inventory.txt`. Complete hashes are in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/adversary-2/SHA256SUMS`. The portable report mechanically replaces only the absolute local home-directory prefix with ~; all other text is identical. All local references are plain/code paths.

```findings
[]
```
