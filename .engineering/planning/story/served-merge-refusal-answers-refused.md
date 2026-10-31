---
format: aep.planning-md/3
id: story:served-merge-refusal-answers-refused
kind: story
status: active
title: A served pull_request.merge that GitHub refuses answers refused, not outcome_unknown
summary: 'D282: a merge GitHub refuses (branch behind a strict ruleset) answered HTTP 502 outcome_unknown in the G7 rehearsal; a definite refusal must answer refused with the provider''s message.'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/src
- confidence: inferred
  path: crates/connectors-host/src/server/v1alpha2.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-31T02:50:24Z", actor: "human:timo", revision: 3, executor: "agent:conn-d298", correlation: "wave-d298"}
- {from: "proposed", to: "active", at: "2026-10-31T02:50:24Z", actor: "human:timo", revision: 4, executor: "agent:conn-d298", correlation: "wave-d298"}
---
# Story: served-merge-refusal-answers-refused

## Outcome

An agent whose served `pull_request.merge` is refused by GitHub (for example a branch behind a
strict up-to-date ruleset) gets `refused` with GitHub's message, and can update the branch and
retry, instead of `outcome_unknown` (HTTP 502), which tells it nothing happened or may have.

## Context

D282. In the G7 cluster rehearsal (2026-10-30, D280), the served github instance answered HTTP 502
`outcome_unknown` to `pull_request.merge` on a pull request whose branch was behind main under a
ruleset with `strict_required_status_checks_policy: true`, three times. The instance log shows only
`operation completed operation="pull_request.merge" elapsed_ms=2760 success=false`.

Verified: the pull request read `mergeable: MERGEABLE`, `mergeStateStatus: BEHIND`, and the strict
ruleset applied. Inferred, not observed: GitHub refused the merge with a definite answer (expected
405 or 422, "Head branch was modified" or "not up to date"), and the instance classified that
definite refusal as `outcome_unknown`.

## Acceptance

- GitHub's actual status and body for a merge of a behind branch under a strict ruleset are
  recorded first, from the instance's audit or log or a reproduction, before the mapping changes.
- A definite GitHub refusal of a write (a 4xx answer that says the write was not performed, such
  as 405 or 409 or 422 on `pull_request.merge`) answers `refused` carrying the provider's message,
  with the mutation classified as not performed.
- `outcome_unknown` stays for transport failures, timeouts and answers that do not say whether the
  write happened (5xx, a cut connection).
- A test drives `pull_request.merge` against a stub answering 405 and 422 with GitHub's bodies and
  asserts `refused` with the message, and against a cut connection asserting `outcome_unknown`.

## Out of Scope

- Updating the branch or retrying on the agent's behalf.
- Changing the GitHub definition's operations in connectors-definitions.

## Scope in wave D298 (D313)

No recording of GitHub's answer to a merge of a behind branch under a strict ruleset exists in this
repository: the instance log of the G7 rehearsal shows only `success=false`.
D313: "if no recording of GitHub's real answer exists, cover only
the test-double refusal and leave the live record as a follow-up story". So this story's first
acceptance line moves to `story:served-merge-refusal-live-record`, and wave D298 delivers the
mapping and its test-double tests: a stub answering 405 and 422 with GitHub's documented bodies, and
a cut connection.
