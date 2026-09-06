---
format: aep.planning-md/1
id: verification-report:cli-schema3-publication-gate-20260906
kind: verification-report
status: draft
title: Complete GitLab and rate schema 3 publication gate
relations:
- verifies: story:personal-gitlab-schedules-are-discoverable-and-governed
- verifies: story:rate-limit-in-the-protocol
revision: 1
---
Schema 3 publication verification — complete local gate, 2026-09-06

All twelve authoritative workspace gates passed, comprising thirteen completed test configurations. All thirteen strict all-target Clippy configurations, twelve final format checks, twelve locked/offline metadata graphs, bundle/catalog checks, final governance checks, release refusal guard and final-HEAD complete-history scanner passed. This was verification of the accepted candidate; no additional adversary tests or product repairs were introduced by this verifier.

Final candidate: 900fac0a435b6e0d2922825f366e0c9607150400.
Managed source: ~/.local/state/worktree/trees/b10x/connectors/wt-af054beacfba
Evidence root: ~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/stage2-publication-gate

The final inventory contains 1,168 tracked files. All 998 nonplanning files remain byte-identical to the approved 493a6f4e source freeze. Parent-owned changes after that freeze were planning only: the cf20627b heading correction and the 900fac0a prospective reserve note. The initial candidate was 8a622cf0. Before compilation, the parent corrected only three preexisting SQL formatting files at 493a6f4e. The final worktree is clean; no candidate source, tests, helper, lock, dependency, generated file, AEP artifact, commit or ref was modified by this verifier. The explicitly authorized bare verification fixture is separate scratch state.

| Workspace | Configuration | Passed | Failed | Ignored | Test exit | Strict Clippy exit |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| root | default | 1214 | 0 | 4 | 0 | 0 |
| connectors-runtime | default | 349 | 0 | 2 | 0 | 0 |
| connectors-runtime | no-default-features | 349 | 0 | 2 | 0 | 0 |
| connectors-cli | default | 131 | 0 | 0 | 0 | 0 |
| connectors-console | default | 87 | 0 | 0 | 0 | 0 |
| driver-audio | default | 15 | 0 | 0 | 0 | 0 |
| driver-speech | default | 22 | 0 | 2 | 0 | 0 |
| driver-cdp | default | 29 | 0 | 2 | 0 | 0 |
| driver-sip | default | 5 | 0 | 0 | 0 | 0 |
| driver-sql | default | 28 | 0 | 14 | 0 | 0 |
| rtvbp-voice-endpoint | default | 7 | 0 | 0 | 0 | 0 |
| voice-local-audio | default | 4 | 0 | 1 | 0 | 0 |
| voice-runtime | default | 4 | 0 | 0 | 0 | 0 |

Counts include each invocation's Rust unit, integration and doc-test summaries. There is no inferred unique total across repeated dependencies or feature configurations. SQL's first invocation stopped during compilation before tests ran; the completed row is its full resumed authoritative lane. No test or Clippy product failure occurred in the completed configurations. Ignored cases are listed verbatim below.

The authoritative workspace list and test implementation came from scripts/gate.sh (SHA256 dec01707adf0915adbe961ab2293231ac74d2f271f90605e1b528c361b0f23b7). Each lane used bash scripts/gate.sh --workspace <exact listed path>; the script itself ran the additional runtime --no-default-features tests. No alternate test list or replacement gate was used. Strict Clippy used cargo clippy --workspace --all-targets --locked --offline -- -D warnings in each workspace, with an additional runtime --no-default-features check. Format used cargo fmt --all -- --check. Metadata used cargo metadata --locked --offline --format-version 1. Full exact argv, cwd, environment, exits, times and logs are retained in commands-final.json and the command-prefixed raw files.

All commands used the recorded PATH selecting ~/.cargo/bin/ess 0.18.0, cargo/rustc 1.98.0 and sccache 0.16.0. Compiling commands unset CARGO_TARGET_DIR, used TMPDIR=~/.cache/cw6/r, RUSTC_WRAPPER=/usr/bin/sccache, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0 and CARGO_BUILD_JOBS=1. Test scheduling remained the repository's ordinary default.

Root checks completed while its target was warm:

- operation_v2_bundle check exited 0: schema and bundle match; '85 vectors checked by Rust and Draft 2020-12'.
- catalog diff exited 0: '69 artifacts up to date (65 providers checked)'.
- scripts/gate.sh --final exited 0: '65 providers, 69 artifacts verified'; 'markdown links are repository-portable'; 'story index and 73 records are consistent'; 'connectors v1 — 10 file(s), valid'; committed ESS Clap tree matched fresh generation.
- Root tests included JSON governance and the retained catalog/schema fidelity/fixed-point assertions. Existing legacy-source diagnostics in catalog output remain in the raw logs; this report does not claim unsupported legacy semantics were newly implemented.

Root tests began at 493a6f4e and root --final ran at cf20627b. The later 900fac0a change affects only journal/runbook text; all nonplanning source hashes stayed identical. check-links.py and check-stories.py were rerun at that exact final HEAD and both exited 0. Catalog, schema, ESS models and generated source were unchanged.

The release local-identity proof is explicit. The first direct cargo check --manifest-path crates/connectors-cli/Cargo.toml --locked --release --features local-identity was interrupted by the resource floor (exit 143). The exact resumed command exited 101 at identity-http/src/lib.rs:15 with its intended compile_error refusing the feature in a release build. The unchanged scripts/check-local-identity-refused.sh then exited 0: 'local-identity is refused by the release profile and absent from the image and archive builds'. The actual compiler refusal is preserved separately from the wrapper's passing exit.

Retained red observations and their bounded resolution:

- Initial driver-sql cargo fmt --check exited 1. credentials.rs, lib.rs and tests/live.rs were byte-identical to published main 4b32397df2cc2f5bd5ee1c5737891163fb750957. The parent made only formatter changes in those three files; the corrected format invocation exited 0. The original diff/origin proof remains in fmt-driver-sql.* and preparation-origin.json.
- Original unchanged scripts/check-secrets.sh over the shared repository's all-local-ref view exited 1: 438 patch-bearing commits and 15 findings. The pinned fully redacted JSON diagnostic remains intact. All seven distinct finding commits are outside both the candidate and published-main ancestry. Value-free reconstruction established that all 15 captured values equal existing unpublished OAuth scratch-evidence basenames in planning references. These are planning-reference false positives; this does not validate unrelated SHA256 citations elsewhere on those lines. No matched values or credentials are exposed in this report. Only RuleID, Description, File, StartLine, Commit and Fingerprint are exported in secret-findings-fields.json.
- CLI release compilation stopped at 17,081,683,968 free bytes, exit 143. After its process group was confirmed gone, expressly authorized cargo clean --profile dev removed only completed CLI debug artifacts. Frozen logs/source and all 3,180 partial release files were verified unchanged before resuming.
- SQL's first compile stopped at 17,021,902,848 free bytes, exit 143, before tests. Its process group was confirmed gone. A roughly 191 MB partial target remained while available shared space subsequently fell to about 10 GB with no wave compiler active. Compilation stayed paused. At 900fac0a, the coordinator prospectively changed subsequent wave builds to a 12 GiB reserve and later explicitly confirmed that policy. SQL resumed only after headroom returned; its full tests and Clippy passed.
- One scratch commands.json write exceeded the shell argument limit after an authorized bare fetch had already started. That fetch completed successfully. The log writer switched to bounded chunks without rerunning the fetch or changing source. This orchestration error was separate from all gate/compiler results.

The secret-scan scope was closed against the actual CI checkout behavior. release.yml's checks job uses actions/checkout@v7 with fetch-depth: 0. The observed v7 source was 3d3c42e5aac5ba805825da76410c181273ba90b1; its ref helper fetches all public heads/tags, not only the candidate ancestry. At the stable observed snapshot there were 13 heads and 27 tags, including three heads and tag v0.5.1 outside candidate ancestry. The same authorized scratch bare fixture contains those 40 exact public refs plus the final candidate ref; it imported no unpublished local refs and changed no primary refs.

At exact 900fac0a, independently enumerated expected and isolated graphs match byte-for-byte: 445 reachable commits (SHA256 ababd13fdd572349f9d64af56c263c10d2fa3caf359cf8f0d4a5499760872f76) and 391 patch-bearing commits (SHA256 adce0fc47b9873f6eb2b9ed39f07eaac9da8488995897a3d80d6dded79f70b56). No shallow boundary, graft, replacement or alternate exists. The exact unchanged scanner script ran with the isolated GIT_DIR and unchanged candidate GIT_WORK_TREE; setup/Trace2 evidence proves its log --all read that database. It exited 0: 391 commits, 106,043,742 bytes, no findings. Before/after public advertisements are byte-identical. Gitleaks remained pinned at 8.30.1, archive SHA256 551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb. Script, flags, rules and fingerprint ignores were unchanged. ci-900f-history-proof.json is the deciding final-HEAD proof; earlier candidate-only and cf20627b public-ref proofs remain retained.

This scan establishes the observed final-candidate plus public-ref snapshot. It cannot cover future ref changes or an as-yet uncreated synthetic pull-request merge; the actual publication CI remains authoritative for that later checkout. No source publication, external adoption or schema 4 / OAuth shipment is claimed by this verification.

Resource and cleanup evidence:

Exactly one workspace compiled in this wave at a time. Five-second free-byte sampling guarded each command; below the active reserve the runner sent TERM to its owned process group and preserved the real exit. Earlier commands used 16 GiB (17,179,869,184 bytes). Commands starting with the resumed SQL lane used the recorded prospective 12 GiB (12,884,901,888 bytes); none crossed that new floor, and their lowest monitored sample was 15,696,175,104 bytes. The two older floor interruptions remain unchanged. resource-timeline-final.json records every sample, command and applicable floor, while the hold snapshots record periods with no active compiler.

Every completed owned target had full command outputs and source inventories hashed and verified before cargo clean, followed by an unchanged-source/evidence check. The authorized CLI debug-profile cleanup additionally proved partial release bytes unchanged. All twelve owned workspace targets are absent at handoff; no other session's target, shared cache or primary checkout was cleaned by this verifier. All thirteen immutable workspace/profile checkpoint manifests reverified successfully. The compilation slot was released to the coordinator after the last verified cleanup. Free space at the final aggregate was 15,960,186,880 bytes; no verifier compilation remained active.

Final source inventory SHA256: f8a07dbbff20c101f7126d72a3da6a7224161648519b86dca4d52fef1fa3c32c. Nonplanning inventory SHA256: ced19f15d0939a6e7f23f89612da223e4b1b9650c23d7e5232c885c52c79232d. verification-final.json contains the per-lane results, complete verbatim Rust summaries, ignored cases, check exits, cleanup proofs and resource minima. commands-final.json, source-final.sha256, resource-timeline-final.json and evidence-final.sha256 provide the frozen command/source/evidence inventory. The report's portable version mechanically replaces the original home-directory prefix with ~; all other report text is identical.

Ignored cases (existing repository controls, not silently counted as passed):

. / default:
- test pack::measure_read_costs ... ignored, a measurement for predecessor:docs/designs/catalog-artifact.md, not an assertion
- test file::tests::a_foreign_owned_directory_is_refused_without_repair ... ignored, requires euid 0 to plant a foreign-owned directory; CI invokes it explicitly with sudo
- test file::tests::a_foreign_owned_store_is_refused_without_repair ... ignored, requires euid 0 to plant a foreign-owned file; CI invokes it explicitly with sudo
- test keyring::tests::a_credential_round_trips_through_the_real_keyring ... ignored, requires a running freedesktop Secret Service and secret-tool

crates/connectors-runtime / default:
- test port_tests::the_postgres_backend_conforms ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
- test port_tests::the_postgres_backend_serves_grant_evaluation ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL

crates/connectors-runtime / no-default-features:
- test port_tests::the_postgres_backend_conforms ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
- test port_tests::the_postgres_backend_serves_grant_evaluation ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL

crates/driver-speech / default:
- test piper::live::probing_resolves_every_component_without_emitting_audio ... ignored, requires an installed synthesizer and voice model
- test piper::live::speaks_one_audible_utterance ... ignored, plays audible speech on the local device

crates/driver-cdp / default:
- test live::opens_a_dedicated_profile_and_reads_a_page_as_structure ... ignored, launches a real browser window
- test live::the_dedicated_profile_is_separate_from_the_operators_own ... ignored, launches a real browser window

crates/driver-sql / default:
- test mysql_auth_failure_never_echoes_the_password ... ignored, needs a live MySQL; see the module docs
- test mysql_byte_cap_truncates_honestly ... ignored, needs a live MySQL; see the module docs
- test mysql_inventory_reads_work ... ignored, needs a live MySQL; see the module docs
- test mysql_row_cap_truncates_honestly ... ignored, needs a live MySQL; see the module docs
- test mysql_show_and_describe_run ... ignored, needs a live MySQL; see the module docs
- test mysql_small_result_returns_untruncated ... ignored, needs a live MySQL; see the module docs
- test mysql_write_statement_refused_pre_connection ... ignored, needs a live MySQL; see the module docs
- test postgres_auth_failure_never_echoes_the_password ... ignored, needs a live PostgreSQL; see the module docs
- test postgres_byte_cap_truncates_honestly ... ignored, needs a live PostgreSQL; see the module docs
- test postgres_caller_max_rows_narrows_the_cap ... ignored, needs a live PostgreSQL; see the module docs
- test postgres_inventory_reads_work ... ignored, needs a live PostgreSQL; see the module docs
- test postgres_row_cap_truncates_honestly ... ignored, needs a live PostgreSQL; see the module docs
- test postgres_small_result_returns_untruncated ... ignored, needs a live PostgreSQL; see the module docs
- test postgres_write_statement_refused_pre_connection ... ignored, needs a live PostgreSQL; see the module docs

crates/voice-local-audio / default:
- test tests::loopback_is_audible_on_the_real_device ... ignored, uses the real microphone and speaker

Verification is complete within the recorded local gate scope. Source publication, the eventual CI checkout and operator actions remain coordinator-owned.
