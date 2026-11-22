---
format: aep.planning-md/3
id: story:main-failing-ci-journeys
kind: story
status: draft
title: Eleven ignored journeys fail in the CI journeys job on main
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Finding

Eleven ignored journeys fail in the CI journeys container (archlinux image, ext4 /journeys,
root-owned /target) on main 7c22ff3878 and on wave/zero-issues-c alike: probe PR 249 run
43055896285 (11 of 136 failed) and PR 250 run 43055454584 (11 of 137 failed), the same names.

- catalog provider cli_journey: basic_catalog_cli_connects_by_file_and_stdin_and_refuses_by_code,
  gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart,
  lifecycle::catalog_cli_failed_repair_and_busy_stop_preserve_authority, revision_recovery (2),
  revision_recovery_adversary (2)
- catalog provider slack_approval::a_slack_bot_post_is_issued_the_approval_its_prepare_named_and_invoked_with_it
- connectors-host: custody::tests::custody_neither_reads_nor_changes_the_default_alias,
  final_fence::withdrawal_at_final_dispatch_prevents_real_child_invocation,
  launch::tests::disposable_consumer_launch_delivers_fd3_and_refuses_by_code

They are deferred in crates/connectors-build/src/ignored.rs so the journeys job holds the other
125 or 126. Each run's logs are its `ignored-journeys` artifact (14 days).

## Outcome

Each of the eleven passes in the CI journeys job, or is shown to need what that job cannot give
and is classified for it, and leaves the Deferred arm.

## Acceptance

- The journeys job on a pull request runs the eleven as required and they pass, or each one left
  deferred names the prerequisite it lacks.
