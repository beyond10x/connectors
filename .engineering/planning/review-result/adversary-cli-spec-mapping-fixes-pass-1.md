---
format: aep.planning-md/3
id: review-result:adversary-cli-spec-mapping-fixes-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: cli-spec-mapping-fixes'
relations:
- reviews: story:cli-spec-mapping-fixes
revision: 1
---
unit: story:cli-spec-mapping-fixes, worktree wave1001b-spec at a63a7f9dd (base b19027874) plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 116→121, red 3
origin: introduced 1 / pre-existing 2 / undecided 0
wrote-outside-worktree: 3 paths (scratch limit/, examples/, suite.log, def-base.json under wave-20261001b/spec/scratch)
needs-coordinator: whether "the fallback `build` request is modelled" in the acceptance means the request types the CLI sends, or only the build reply

Cases (crates/connectors-build/tests/cli_spec_mapping_adversary.rs), run alone: `test result: FAILED. 2 passed; 3 failed`.

| case | asserts | now |
|---|---|---|
| the_owner_greeting_reply_is_declared_with_the_hosts_fields | LocalOwnerGreeting fields equal Reply::Hello's, build is Optional<String> | green (control) |
| the_owner_greeting_request_the_cli_sends_is_declared | some cli struct carries Request::Hello {version, challenge, configuration, authority, build} | red: no connectors.cli struct declares the Request::Hello fields {"authority", "build", "challenge", "configuration", "version"} |
| the_build_digest_format_is_declared | build resolves to a hex-alphabet newtype | red: connectors.cli.LocalOwnerBuild.build is `String`: the 64-hex digest format is not declared |
| the_er_rs_citation_lands_on_the_cursor_expiry_path | lines cited as er.rs:a-b mention ConnectionListCursor | red: cli.yaml cites er.rs:3114-3118 for the cursor removal path, which reads: if !er.baseline.contains_key(key) { …creation_stages… |
| the_corrected_semantics_citations_land_on_their_rules | semantics.md:345 and :349 | green (control) |

Suite: `cargo test --locked -p connectors-build --no-fail-fast` EXIT=101, 118 passed, 3 failed. `ess specify validate` valid; `cli --check` current; `metadata-entities --check` exit 0.

Attacked, not broken: both ESS-LIMIT comments true at 0.45 (pinned generate cli refuses invariants on ApprovalClockCheckResult and Failure); semantics.md :345/:349 correct; website example model unaffected (no non-cli domain contains `connectors.cli.`; closure recomputed, 14 domains); generated outputs digests-only / unchanged.

```findings
- file: ess/domains/cli.yaml
  line: 146
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: the Request::Hello greeting the CLI sends (caller build, configuration) and the fallback Build request are declared by no connectors.cli type, only described in a comment, although ESS 0.45 accepts a struct for it
- file: ess/domains/cli.yaml
  line: 153
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: LocalOwnerGreeting.build and LocalOwnerBuild.build (:160) are bare String although the host emits and checks a 64-character hex SHA-256 and ESS 0.45 accepts a hex-alphabet newtype on these types
- file: ess/domains/cli.yaml
  line: 832
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the cursor-removal citation er.rs:3114-3118 lands on create-stage code; the Active-to-Expired transition is at er.rs:3538-3540
```
