---
format: aep.planning-md/1
id: review-result:cli-oauth-adversary-1-20260906
kind: review-result
status: active
title: Personal OAuth first whole-unit adversary report
relations:
- reviews: story:connect-session-oauth-custody-in-personal-posture
revision: 1
---
unit: connect-session-oauth-custody-in-personal-posture, formal pass 1; 6fae9df000986f39d009a2e8503bdff03db41762 plus retained tests.patch
verdict: CONFIRMED
cases: executed 1496→1509, red 1
origin: introduced 0 / pre-existing 0 / undecided 1
wrote-outside-worktree: 226 retained reviewer scratch paths; assigned build/TMPDIR and tool cache roots below
needs-coordinator: yes — record this completed pass and route the measured authoring mismatch; no production correction applied

```text
 .../catalog-build/tests/main/catalog_invariants.rs |  52 +++++
 crates/catalog-reader/tests/main/pack.rs           |  15 ++
 .../connect-session-transport/src/oauth_tests.rs   |  57 +++++
 crates/connector-spec/tests/main/personal_oauth.rs |  23 ++
 crates/connectors-cli/tests/cli_surface.rs         | 105 +++++++++
 .../connectors-config/src/personal_oauth_tests.rs  |  27 +++
 crates/connectors-console/tests/personal_oauth.rs  | 260 +++++++++++++++++++++
 crates/connectors-runtime/tests/personal_oauth.rs  |  56 +++++
 crates/integration-catalog/src/oauth_tests.rs      |   3 +
 9 files changed, 598 insertions(+)
 .../tests/personal_oauth_adversary.rs              | 294 +++++++++++++++++++++
 1 file changed, 294 insertions(+)
 .../src/oauth_adversary_tests.rs                   | 230 +++++++++++++++++++++
 1 file changed, 230 insertions(+)
```

The nine-file stat is verbatim `git diff --stat`; the following two are verbatim `git diff --no-index --stat /dev/null <new-test-path>`. All eleven changed files are tests. The original eleven-owner assignment was prospectively extended only by `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/oauth-test-owner-assignment.md`. All original bytes/assertions in ten preexisting owners remain in order; all 1,193 reviewed source hashes were rechecked, with every unowned source unchanged. The custody-refresh test owner remains byte-exact. Proof: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/final-original-test-preservation.json` and `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/final-source-scope-proof.json`.

This same first formal pass covers the complete OAuth union over published base `0c69450921ab1794c81dadec915b717a61bf0983`: helpers, FULL SQLite, custody/refresh, schema 4, runtime and trusted client/console/CLI. The brief, full prior reports, 608-member implementor seal, Design 21/story and current ESS were inspected. Separate Design 22/auth-protocol work supplies no implementation here. No provider/live account, Git/AEP, production, dependency, operator or cleanup mutation occurred.

Every stem below expands to `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/oauth-pass1-<stem>`, with complete verbatim `.log`, exact cwd/argv `.command.json`, actual `.result.json`, continuous `.resources.jsonl` and warm `.warm-target.json`. All 32 executions are indexed in `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/commands-and-counts.json`; successful compiler output remains there. Exact case file/line/name/assertions: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/case-inventory.json`.

| Deciding stem | Assertion | Final selection |
| --- | --- | --- |
| `spec-empty-first` | Explicit empty admission versus published minItems=1; omission control succeeds. | 0 pass / 1 fail, exit 101 |
| `client-handoff-first` | Public client binds success to expected target/profile/Callable state; one Create and no Invoke. | 1 pass / 0 fail, exit 0 |
| `client-private-first` | Redirect, oversized/chunked, cacheable, wrong-origin and control-code replies close without redirect/poll/private echo. | 1 pass / 0 fail, exit 0 |
| `catalog-compat-fixture-corrected` | All 65 current planned documents validate frozen v3 after only declared normalization; operations retained. | 1 pass / 0 fail, exit 0 |
| `reader-version-compile-corrected` | Actual nonempty v4 pack loads; independently digested v2/v3/future headers refuse before records. | 1 pass / 0 fail, exit 0 |
| `config-boundaries-first` | Actual TOML validation holds TTL endpoints and scope ceiling/uniqueness/encoding bounds. | 1 pass / 0 fail, exit 0 |
| `callback-final-slot-first` | After 62 malformed callbacks, final valid code/denial works once and retires liveness/listener. | 1 pass / 0 fail, exit 0 |
| `explicit-reauthorization-relocated` | Actual refresh uncertainty and reopen block reuse; wrong-subject repair refuses, coherent repair works; 429 sends once. | 1 pass / 0 fail, exit 0 |
| `device-denial-relocated` | Device slow_down/pending/denial retain original deadline, one authorization and terminal endpoint cleanup. | 1 pass / 0 fail, exit 0 |
| `runtime-ambiguous-first` | Actual local daemon refuses ambiguous/missing profile; label cannot select another binding. | 1 pass / 0 fail, exit 0 |
| `console-real-pty-compile-corrected` | Actual controlling PTY receives human instructions while redirected text/JSON/YAML outputs stay private. | 1 pass / 0 fail, exit 0 |
| `console-cancellation-first` | Cancellation erases original private inode through held descriptor, including rename/replacement. | 1 pass / 0 fail, exit 0 |
| `cli-private-refusal-first` | Actual CLI private setup error clears file and closes private text in four output formats. | 1 pass / 0 fail, exit 0 |

Each of the 13 new cases was written before its first execution. The first actual product-red command was `cargo test --locked --offline -p connector-spec oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract -- --nocapture`, exit 101. Its test output follows verbatim; the final formatted assertion is at test line 166:

```text
running 1 test

thread 'personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract' (4098534) panicked at crates/connector-spec/tests/main/personal_oauth.rs:159:5:
the published minItems=1 contract rejects explicit empty admission; the actual provider loader must retain that presence distinction
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract ... FAILED

failures:

failures:
    personal_oauth::oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 516 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p connector-spec --test main`
```

Other initial failures are retained separately:

- `catalog-compat-first`: one failed case because my fixture unwrapped absent auth on non-auth providers. Only that traversal was corrected; the selected case then passed.
- `reader-version-first`: zero executed, exit 101; my iterator `is_empty` call needed an unstable API. Only that call became `len() > 0`.
- `console-real-pty-first`: zero executed, exit 101; my cancellation fixture used `tokio::spawn` with the existing non-Send console future. Only the new fixture gained a LocalSet.
- `root-full`: 1,102 passed / two failed / one old ignore, exit 101. Besides the authoring assertion, the existing size fence rejected my growth of `oauth_tests.rs` from 1,470 to 1,699 lines. The prospectively assigned move retained the new 229-line suffix verbatim in `oauth_adversary_tests.rs`, preceded by `use super::*;`. The original parent prefix is exact; parent/child now have 1,473/230 lines. Both original and exact relocated deciding selections are retained. The required root rerun passed the size fence. Extraction proof/patch: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/oauth-test-relocation-proof.json` and `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/oauth-test-relocation.patch`.

These are fixture/compiler/test-placement corrections, with all preimages and actual outputs retained; no product fix, old assertion change, waiver or new pass occurred.

Final affected full commands used ordinary parallel runners and `--no-fail-fast`. No baseline suite preceded the deciding cases. Cargo argv are verbatim below; each command record retains the complete environment and cwd:

- `root-full-relocated`: `cargo test --locked --offline -p connector-spec -p connector-resolve -p catalog -p catalog-build -p catalog-reader -p connectors-client -p protocol -p server -p service -p connector-oauth --no-fail-fast` — 1103 passed / 1 failed / 1 ignored, exit 101.

- `runtime-full`: `cargo test --locked --offline -p connectors-config -p connect-session-transport -p integration-catalog -p connectors-runtime -p state-sqlite --no-fail-fast` — 221 passed / 0 failed / 0 ignored, exit 0.

- `console-full`: `cargo test --locked --offline --workspace --no-fail-fast` — 101 passed / 0 failed / 0 ignored, exit 0.

- `cli-full`: `cargo test --locked --offline --workspace --no-fail-fast` — 133 passed / 0 failed / 0 ignored, exit 0.

The comparable operational cohort is **1,496→1,509 executed: 1,508 passed, one failed, one old reader measurement ignored**. The root total additionally includes 36 existing connector-oauth tests, and runtime includes 14 existing SQLite tests; those 50 supplementary passes are excluded from the cohort. Expected compile-fail doctests passed. Exact counting, without summing repeated selections/reruns: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/final-counts.json`.

The separately assigned `published-reader-first` used the exact three rustc/test argv arrays and untouched base reader/pack, without Cargo or a shared target. It loaded the real schema-3 pack (65 providers, nonempty operations), then refused the actual current schema-4 pack before records: one passed case; all commands exit 0. It proves reader compatibility, not old provider-loader execution. Exact commands and source/output hashes: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/published-reader-probe/commands.json`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/published-reader-source-provenance.json`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/published-reader-output-inventory.json`.

All four strict Clippy checks passed with `--locked --offline`, the same package/workspace selections, and `--all-targets -- -D warnings`: `root-clippy`, `runtime-clippy`, `console-clippy`, `cli-clippy`. All four `cargo fmt --all -- --check` commands passed: `root-fmt`, `runtime-fmt`, `console-fmt`, `cli-fmt`. Completed root/console/CLI compilation inputs, dependencies and assertions were unchanged by test relocation; final runtime strict/fmt cover the child module. No final twelve-workspace gate, approval or live interoperability claim is made.

| File:line | Category / severity | Verdict / origin | Finding |
| --- | --- | --- | --- |
| `crates/connector-spec/src/provider/auth_validation.rs:300` | contract-drift / warning | CONFIRMED / undecided | The provider loader accepts an explicitly empty personal_flows array although the new authoring schema requires at least one entry. |

**Measured:** the omission control loads; explicit-empty admission fails the schema-derived assertion through the public provider loader, both alone and in both full root executions. **Reachable caller:** a provider author can supply `personal_flows = []` through `connector_spec::provider::load`; both public loading paths converge on loading.rs:135 and the same validator used for catalog loading. Empty flows confer no runtime OAuth admission, so this is an authoring contract mismatch, not measured credential exposure.

Origin is **undecided** because this loader case was not executed against the base. The field, schema constraint and early-empty guard are new unit-diff bytes; that source fact is separate from base execution. The historical reader witness does not answer the loader question. The coordinator owns correction/routing.

Attacked without a product break: real controlling-terminal/private-output separation; public-client destination/proxy/redirect/body-bound checks and bound Callable success; callback state/Origin/Host and final-slot retirement; device deadline and one authorization; durable refresh/reopen/coherent explicit repair and single-send 429; ambiguous Connection-v1 configuration and raw/OAuth exact-owner routing. Existing custody, refresh, raw, rate and GitLab assertions remain and their affected suites executed. Synthetic local fixtures do not claim live vendor interoperability, power-loss durability or model/inner-harness URL admission.

The direct audit confirms all 65 documents preserve published semantics after only declared version/GitLab normalization; 17 frozen catalog/Operation-v1/v2/Connection-v1 contract files are byte-exact. Retained checks: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/all-provider-published-base-comparison.json`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/frozen-contract-published-base-comparison.json`.

All 32 commands finished; no resource guard interrupted a run. Continuous minima: disk 30864338944 bytes, tmpfs 24581185536 bytes, MemAvailable 35998396416 bytes. Peaks: private target 9023692800 bytes, CLI target 2782941184 bytes, separate reader probe 76873728 bytes. All assigned reserves/caps held. The sole slot was explicitly lent/returned. The coordinator-reported harness usage-limit interruption was resumed through the already-owned CLI full process, which completed exit 0; it is not a product result or second pass. Note: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/harness-continuation-note.json`. Slot released after the last check; no further compilation occurred.

Every retained outside path is in `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/outside-paths.json`: 226 reviewer scratch files and two coordinator supplements. Additional assigned write roots: `/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target` and `~/.cache/cw6/a`. CLI target is inside the worktree at `~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685/crates/connectors-cli/target`. Ancillary Cargo/sccache bookkeeping is reported at existing tool-managed roots `~/.cache/sccache`, `~/.cargo/.global-cache`, `~/.cargo/.package-cache`, `~/.cargo/.package-cache-mutate`; this is not an exact per-cache-object write trace. No cache, target or worktree cleanup occurred.

Final artifacts: `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/tests.patch`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/final-test-inventory.json`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/reviewed-source-inventory.json`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/raw-report.md`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/report.md`, `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/whole-unit-adversary-1/evidence.sha256`. Raw/public reports are immutable. Portable output changes only the absolute local home-directory prefix to ~; all other prose, findings, counts and log excerpts remain identical. Scratch references are plain/code paths, with no local Markdown links.

```findings
- file: crates/connector-spec/src/provider/auth_validation.rs
  line: 300
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: The provider loader accepts an explicitly empty personal_flows array although the new authoring schema requires at least one entry.
```
