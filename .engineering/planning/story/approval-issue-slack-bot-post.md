---
format: aep.planning-md/3
id: story:approval-issue-slack-bot-post
kind: story
status: implemented
title: approvals issue issues the approval its prepare named for a Slack bot post
summary: 'issue #180: issue answers invalid_input with no field, so the approved write cannot run'
refs:
- provider: github
  reference: beyond10x/connectors#180
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/tests/local_runtime
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/slack_approval.rs
- confidence: cited
  path: apps/connectors-cli-contract/binding.json
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/approvals.rs
- confidence: cited
  path: apps/connectors/src/local/select.rs
- confidence: cited
  path: apps/connectors/src/local/session.rs
- confidence: cited
  path: apps/connectors/tests/approval_field_adversary.rs
- confidence: cited
  path: apps/connectors/tests/approval_field_adversary_2.rs
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance/tests/adversary_field.rs
- confidence: cited
  path: docs/catalog-openrouter.md
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: website/docs/reference/contracts/cli.md
- confidence: cited
  path: website/docs/reference/contracts/model/crossings.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-approval_issuers.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-artifact_provenance.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-auth_access.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-auth_bindings.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-cli.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-clock.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-connection_admission.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-credential_evidence.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-credentials.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-declarations.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-delegation.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-discovery.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-discovery_state.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-execution_audit.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-feed.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-idempotency.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-local_approval_policy.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-mutations.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-refresh.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-service_wire.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-sessions.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-transport.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-websearch.md
- confidence: cited
  path: website/docs/reference/contracts/model/index.md
- confidence: cited
  path: website/docs/reference/contracts/model/interactions.md
- confidence: cited
  path: website/docs/reference/contracts/model/topology.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-19T21:36:31Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-19T21:36:32Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-20T00:45:33Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":2,"review_outcome":2,"verification":1}}}
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

## Scope delivered

Delivered by wave 20261019-post0540 (`git diff --name-only 52769afb3...impl/approval-issue-slack-bot-post`); every path is **cited** by that diff. The `## Scope` section above is the plan; differences are listed under *Corrections*.

- `adapters/catalog/tests/local_runtime.rs`
- `adapters/catalog/tests/local_runtime/slack_approval.rs`
- `apps/connectors-cli-contract/binding.json`
- `apps/connectors/src/local.rs`
- `apps/connectors/src/local/approvals.rs`
- `apps/connectors/src/local/select.rs`
- `apps/connectors/src/local/session.rs`
- `apps/connectors/tests/approval_field_adversary.rs`
- `apps/connectors/tests/approval_field_adversary_2.rs`
- `contracts/cli/v1alpha1/semantics.md`
- `crates/connectors-build/src/ignored.rs`
- `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json`
- `crates/connectors-host/src/local/owner.rs`
- `crates/connectors-host/src/local/owner/approval_issuance.rs`
- `crates/connectors-host/src/local/owner/approval_issuance/tests.rs`
- `crates/connectors-host/src/local/owner/approval_issuance/tests/adversary_field.rs`
- `docs/catalog-openrouter.md`
- `ess/domains/cli.yaml`
- `website/docs/reference/contracts/cli.md`
- `website/docs/reference/contracts/model/crossings.md`
- `website/docs/reference/contracts/model/domains/connectors-approval_issuers.md`
- `website/docs/reference/contracts/model/domains/connectors-artifact_provenance.md`
- `website/docs/reference/contracts/model/domains/connectors-auth_access.md`
- `website/docs/reference/contracts/model/domains/connectors-auth_bindings.md`
- `website/docs/reference/contracts/model/domains/connectors-cli.md`
- `website/docs/reference/contracts/model/domains/connectors-clock.md`
- `website/docs/reference/contracts/model/domains/connectors-connection_admission.md`
- `website/docs/reference/contracts/model/domains/connectors-credential_evidence.md`
- `website/docs/reference/contracts/model/domains/connectors-credentials.md`
- `website/docs/reference/contracts/model/domains/connectors-declarations.md`
- `website/docs/reference/contracts/model/domains/connectors-delegation.md`
- `website/docs/reference/contracts/model/domains/connectors-discovery.md`
- `website/docs/reference/contracts/model/domains/connectors-discovery_state.md`
- `website/docs/reference/contracts/model/domains/connectors-execution_audit.md`
- `website/docs/reference/contracts/model/domains/connectors-feed.md`
- `website/docs/reference/contracts/model/domains/connectors-idempotency.md`
- `website/docs/reference/contracts/model/domains/connectors-local_approval_policy.md`
- `website/docs/reference/contracts/model/domains/connectors-mutations.md`
- `website/docs/reference/contracts/model/domains/connectors-refresh.md`
- `website/docs/reference/contracts/model/domains/connectors-service_wire.md`
- `website/docs/reference/contracts/model/domains/connectors-sessions.md`
- `website/docs/reference/contracts/model/domains/connectors-transport.md`
- `website/docs/reference/contracts/model/domains/connectors-websearch.md`
- `website/docs/reference/contracts/model/index.md`
- `website/docs/reference/contracts/model/interactions.md`
- `website/docs/reference/contracts/model/topology.md`

**Corrections:** `apps/connectors/src/local/select.rs` and `session.rs`, `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json` (regenerated digests) and `docs/catalog-openrouter.md` were not in the plan and were touched. `CHANGELOG.md` was planned (inferred) and not touched; the release step writes it. `binding.json` was inferred and was right.
