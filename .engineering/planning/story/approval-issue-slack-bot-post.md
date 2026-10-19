---
format: aep.planning-md/3
id: story:approval-issue-slack-bot-post
kind: story
status: active
title: approvals issue issues the approval its prepare named for a Slack bot post
summary: 'issue #180: issue answers invalid_input with no field, so the approved write cannot run'
refs:
- provider: github
  reference: beyond10x/connectors#180
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/catalog/tests/local_runtime
- confidence: inferred
  path: adapters/catalog/tests/local_runtime.rs
- confidence: inferred
  path: apps/connectors-cli-contract/binding.json
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/src/local/approvals.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-build/src/ignored.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance/tests.rs
- confidence: inferred
  path: ess/domains/cli.yaml
- confidence: inferred
  path: website/docs/reference/contracts/model/domains/connectors-cli.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-19T21:36:31Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-19T21:36:32Z", actor: "human:timo", revision: 6}
---
## Outcome

`connectors approvals issue --approve-subject <subject>` issues the approval its `approvals prepare`
named for a Slack bot `chat.postMessage` under a revision-2 write policy and the `roughtime-quorum/1`
clock; when `issue` refuses with `invalid_input`, the refusal names the field it rejects.

## Why now

https://github.com/beyond10x/connectors/issues/180 (opened 2026-10-17, seen on 0.53.1): `prepare`
succeeds, `issue` answers `invalid_input` with no field, so `operations invoke --approval-file` has
no proof and the approved write cannot run. The cause is not traced; `approval_issuance.rs:131-359`
holds about 15 field-less `InvalidInput` sites (post-0.54.0 re-plan, 2026-10-19). Whether 0.54.0
still shows it is not known.

## Acceptance

- From fixtures, no live credential: a Slack bot connection, a revision-2 write policy and the quorum
  clock; `approvals prepare` then `approvals issue --approve-subject <subject from prepare>` issues
  the approval, and `operations invoke --approval-file` accepts it.
- Every `invalid_input` that `approvals issue` can answer names its field (a test per site in
  `crates/connectors-host/src/local/owner/approval_issuance.rs`).
- The test that reproduces the issue's sequence fails on 83aee7c96 or the story records that it
  passes there and names the release that fixed it.

## Scope

Derived 2026-10-19 by `story-scoper` on origin/main 0c6cbba7f. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/connectors-host/src/local/owner/approval_issuance.rs` — cited, story Acceptance line 2 names it; the 15 field-less `Code::InvalidInput` sites are at :131, :133, :135, :205, :207, :209, :220, :236, :293, :311, :355-359
- **Files:** `crates/connectors-host/src/local/owner/approval_issuance/tests.rs` — cited, "a test per site"; quorum-clock `setup` at :76-100, CLI journey at :588-590
- **Files:** `crates/connectors-host/src/local/owner.rs` — inferred, `Error` (:98-155) has no member that can carry a field, and `From<Code>` (:156-176) builds every refusal; `F::InvalidInput` is also mapped at :241
- **Files:** `apps/connectors/src/local.rs` — inferred, `owner_failure` (:230-326) renders `InvalidInput` as `("arguments","none",true)` with no field key
- **Files:** `apps/connectors/src/local/approvals.rs` — inferred, `text()` at :15 and `expected_revision` at :30 answer field-less `InvalidInput` for the `approval-issue` callable's own arguments
- **Files:** `ess/domains/cli.yaml` — inferred, `connectors.cli.Failure` (:703-760) declares the wire keys; a new `field` key goes here
- **Files:** `apps/connectors-cli-contract/binding.json` — inferred, the compiled plan (`src/lib.rs:5`), presumed regenerated from cli.yaml
- **Files:** `adapters/catalog/tests/local_runtime/<new Slack approval journey>.rs`, `adapters/catalog/tests/local_runtime.rs` — inferred, the prepare → issue → `--approval-file` harness exists for GitLab in `custody_journey.rs:179-205`; the Slack `chat.postMessage` write fixture lives in `adapters/catalog/tests/slack.rs:2529-2544`
- **Files:** `crates/connectors-build/src/ignored.rs` — inferred, every ignored CLI journey is registered by name (:165, :183-208); a new one must be too
- **Also likely:** `crates/connectors-host/src/local/owner/approval_issuance.rs:512-550` (`policy_error`/`key_error` map store `InvalidInput` field-less) — inferred
- **Documents:** `contracts/cli/v1alpha1/semantics.md` (`invalid_input` rows :263-268), `CHANGELOG.md`, `website/docs/reference/contracts/model/domains/connectors-cli.md` (generated) — inferred
- **Confidence:** medium — the story names the defect file and site range, but the issue's cause is untraced and the wire carrier for "names its field" is not in the tree yet
- **Would collide with:** `story:refusals-name-the-field` (plans `connectors.cli.Failure` + `field` across `local.rs` and `owner.rs`), the `connectors.cli.Failure` wire shape, `owner::Error`, `owner_failure`, and the approval issuance module with its tests
- **Safety fact:** `owner::Error` serializes with `skip_serializing_if` on every optional member (owner.rs:100-155), so an added optional `field` changes no existing reply when absent — unproven
- **Prerequisites:** acceptance 1 is an ignored Disposable custody + CLI journey (`ignored.rs:164-168`) and needs a qualified disposable Secret Service; acceptance 3 needs a build of `v0.54.0` (83aee7c96). No credential, live provider call or paid run.
