---
format: aep.planning-md/3
id: story:git-smart-http-relay-with-quarantine
kind: story
status: implemented
title: Git smart-HTTP relay with quarantine, ref policy, identity check and pre-push check
summary: An agent pod clones and pushes through the instance with no credential and no forge egress
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-github-app-installation-token
- depends_on: story:served-instance-admits-a-service-account-token
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:14:36Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "proposed", to: "active", at: "2026-10-30T03:14:36Z", actor: "human:timo", revision: 3, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "active", to: "implemented", at: "2026-10-30T10:11:28Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
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

## Implemented (wave D265-C4, unit C4)

- `adapters/catalog/src/served/git.rs` serves `/git/<owner>/<repo>.git` (info/refs, upload-pack,
  receive-pack) behind the host's admission (`Admission::admit`, `serve_with_routes` in
  `crates/connectors-host/src/server.rs`). Repositories outside `git.repositories` are 404 before
  any forge request.
- receive-pack is read whole up to `push_limit_bytes` (413), then: ref policy (branch under
  `ref_prefixes`; no delete, tag, shallow, signed or push-option push), quarantine (mirror fetch
  without redirects, pack indexed beside the mirror, connectivity, and every pack object reached
  from a pushed tip or already in the mirror), `commit_identity`, `pre_push_check` (own process
  group, cleared environment). Only then is the original request sent once; outcomes are logged
  `applied`, `refused` or `unknown`, a lost answer is `unknown` and never resent, and a push whose
  caller hangs up still runs to its end.
- The forge gets the instance's token (stored `token` without prefix, or the `github_app`
  installation token) in HTTP basic authentication; git receives it through `GIT_CONFIG_*`
  environment, never argv.

Evidence: `adapters/catalog/tests/git_relay.rs` (11, the acceptance list),
`adversary_git_relay.rs` (5) and `security_git_relay.rs` (4), red before the review fixes;
review results `review-result:wave-d265-c4-adversary` and `review-result:wave-d265-c4-security`.

Not done: the pre-push check runs as the instance's user
(story:git-relay-pre-push-check-sandbox).
