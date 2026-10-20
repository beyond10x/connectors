---
format: aep.planning-md/3
id: story:catalog-tests-read-pinned-release-providers
kind: story
status: draft
title: Catalog provider tests of google-*, jira, confluence, slack and the local runtime read the pinned release
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-pin-covers-catalog
revision: 2
---
## Outcome

The 35 test files listed under Scope stop reading `adapters/catalog/providers/`,
`adapters/catalog/generated/bundles/` and the three-operation Jira copy by path, and read lowered
connectors from the pinned, installed release through the test-support function of
`story:definitions-pin-covers-catalog`. Epic step 3, second of two batches in
`adapters/catalog/tests/`.

## Context

- Same patterns as `story:catalog-tests-read-pinned-release-gitlab-engine`: `bundle::load` on
  `generated/bundles`, selections read from `providers/<name>/`, native configurations whose
  paths point into the tree (for example `google_drive.rs:797-798`, `jira.rs:97,111,434-435`).
- `jira.rs:846` reads `crates/connectors-catalog/tests/fixtures/definition/providers/jira.yaml`
  to show the catalog provider serving `issue.get` from a lowered definition (design section 13);
  it reads the Jira definition of the pinned release instead.
- A test, or part of one, that uses an instance derived from the declarations compiled into the
  catalog provider keeps doing so (`story:instances-resolve-through-locked-index`). Many files
  here do both, among them `local_runtime.rs` and the google read tests.
- ADR Consequences: a test function that only exercises provider data is removed rather than
  moved. Name patterns that point at such tests, for the implementor to confirm by reading each
  one: `shipped_*_selections_*`, `the_bundle_is_derived_from_the_pinned_discovery_document`.
- Why the rename check builds first: `adapters/catalog/build.rs:13-16,29-32` reads both
  directories when the crate builds and panics on a missing one, so the check renames them only
  after the test binaries exist.
- File-disjoint from the first batch; neither touches `hand_written/`, a manifest or
  `Cargo.lock` (`story:definitions-pin-covers-catalog` adds the dev-dependency).

## Domain relations

None beyond those of `story:definitions-pin-covers-catalog`.

## Scope

All under `adapters/catalog/tests/`, cited by a search for `providers` and `generated/bundles`
on 9201b1177.

- google-* (16): `google_calendar.rs`, `google_calendar_gmail_adversary.rs`,
  `google_calendar_gmail_adversary_pass2.rs`, `google_calendar_writes_adversary.rs`,
  `google_calendar_writes_adversary_pass2.rs`, `google_drive.rs`,
  `google_drive_writes_adversary.rs`, `google_drive_writes_adversary_pass2.rs`, `google_gmail.rs`,
  `google_gmail_writes_adversary.rs`, `google_quota_reasons.rs`, `google_reads_adversary.rs`,
  `google_reads_adversary_pass2.rs`, `google_slides.rs`, `google_slides_writes_adversary.rs`,
  `google_slides_writes_adversary_pass2.rs`
- jira, confluence, confluence-v1 (10): `confluence.rs`, `confluence_reads_adversary.rs`,
  `confluence_v1.rs`, `jira.rs`, `jira_attachment.rs`, `jira_cloud_reads_adversary.rs`,
  `jira_issue_writes.rs`, `jira_issue_writes_adversary.rs`, `jira_transition_run.rs`,
  `jira_write_postflight_present.rs`
- slack, slack-user (3): `slack.rs`, `slack_adversary.rs`, `slack_send_adversary.rs`
- local runtime (6): `local_runtime.rs`, `local_runtime/oauth2_refresh.rs`,
  `local_runtime/oauth2_refresh_adversary.rs`, `local_runtime/oauth2_refresh_adversary_pass2.rs`,
  `local_runtime/revision_recovery.rs`, `local_runtime/slack_approval.rs`

## Not in this story

- `connector_declarations.rs`, `guide_instances.rs`, `bundle_drift.rs`, as in the first batch.

## Acceptance

- With the `connectors-catalog-provider` test binaries built first and
  `adapters/catalog/providers`, `adapters/catalog/generated/bundles` and
  `crates/connectors-catalog/tests/fixtures/definition/providers` then renamed away, every file of
  this batch still in the tree passes when those binaries run.
- The delivered story body has a `## Removed tests` section naming each removed test function
  with its file and the reason, or stating that none was removed.
