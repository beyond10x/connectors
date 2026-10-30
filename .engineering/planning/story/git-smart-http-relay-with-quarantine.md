---
format: aep.planning-md/3
id: story:git-smart-http-relay-with-quarantine
kind: story
status: active
title: Git smart-HTTP relay with quarantine, ref policy, identity check and pre-push check
summary: An agent pod clones and pushes through the instance with no credential and no forge egress
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-github-app-installation-token
- depends_on: story:served-instance-admits-a-service-account-token
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:14:36Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "proposed", to: "active", at: "2026-10-30T03:14:36Z", actor: "human:timo", revision: 3, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
---
## Outcome

A served instance relays git smart-HTTP for the repositories it is configured for, so an agent
pod clones, fetches and pushes with no credential and no egress to the forge. Route
`/git/<owner>/<repo>.git/{info/refs,git-upload-pack,git-receive-pack}`.

- upload-pack (clone and fetch) is relayed with the installation token.
- receive-pack is buffered up to a configured bound and indexed into a per-request quarantine
  repository the instance owns. The instance then checks, in order: the ref policy
  (`refs/heads/<allowed prefix>/*` only, no delete, no tag); the author and committer of every
  new commit equal the configured bot identity; the configured pre-push check (for example a
  secret scan whose policy is mounted outside the pod). Only then does it record an attempt and
  relay the original commands with the installation token. The old object id of each command is
  the forge's own compare-and-swap. `report-status` maps to applied, refused or unknown, and a
  push is never resent.
- Any git client works unchanged.

Next wave after D243-C1; not in that wave.

## Decided

- C1: git smart-HTTP with quarantine (orchestrator, 2026-10-30).
- D1: a configurable pre-push check on the instance; the policy stays outside the pod.

## Order

Depends on `story:catalog-github-app-installation-token` (the token) and
`story:served-instance-admits-a-service-account-token` (the caller).

## Acceptance (named tests)

Against a loopback git smart-HTTP upstream and a stock `git` client:
- `a_clone_and_fetch_through_the_relay_carry_the_installation_token`
- `a_push_to_an_allowed_branch_is_relayed_once_and_reported_applied`
- `a_push_outside_the_ref_prefix_is_refused_with_no_upstream_request`
- `a_ref_delete_or_a_tag_push_is_refused_with_no_upstream_request`
- `a_commit_by_another_author_or_committer_is_refused_with_no_upstream_request`
- `a_failing_pre_push_check_refuses_the_push_with_no_upstream_request`
- `a_push_over_the_buffer_bound_is_refused`
- `a_lost_upstream_answer_is_unknown_and_never_resent`
- `the_relay_admits_only_the_configured_principal`
- `a_repository_outside_the_configuration_is_not_found`
