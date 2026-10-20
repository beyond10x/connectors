---
format: aep.planning-md/3
id: release-plan:connectors-v0550-approval-fields-present-checks
kind: release-plan
status: active
title: 'Release 0.55.0: approval refusals name their argument, present checks, the pool lock and the owner socket'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-20T10:28:55Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Minor release 0.55.0 of wave 20261019-post0540 (pull request
https://github.com/beyond10x/connectors/pull/193, merged as 9201b1177): approval refusals name the
refused argument in `Failure.field` (issue https://github.com/beyond10x/connectors/issues/180); a
catalog guard check can require a member present (`{"present": true}`, `"?"` in a definition) and
Jira `issue.create` and `issue.comment.add` use it; an owner request waits for its instance outside
the pool lock; the owner socket is linked into place only once it is owner-only; the CI flakes of
issue https://github.com/beyond10x/connectors/issues/183 in the Kubernetes exec-plugin and OAuth
drain tests.

Minor rather than patch: a new optional wire member (`Failure.field`), a new guard expectation and
definition syntax, and changed refusals (Jira create and comment answers without a new id are
`unknown`; an empty preflight answer is `upstream_protocol`).

## Scope

Stories moved to implemented before this release, by the wave:

- story:approval-issue-slack-bot-post
- story:owner-request-waits-outside-the-pool-lock

Merged in this release and still active, with what remains in their bodies:

- story:write-postflight-present-check (the connectors-definitions mirror)
- story:ci-timing-flakes-20261018 (20 consecutive macOS and Linux CI runs)

## Evidence

- The repository gate with MSRV (`connectors-build gate --msrv`, ESS 0.59.0, aep 0.71.3) on the release tree (9201b1177 plus the version, CHANGELOG, README and getting-started edits), run through `b10x-build-slot` on 2026-10-20: 28 steps exit 0, none other; 2858 tests passed, 0 failed, 145 ignored; 12 Rust 1.88.0 steps and 1 Rust 1.91 step among them; "all checks passed".
- `target/debug/connectors --version` printed `connectors 0.55.0`; `connectors-docs generate` wrote 0 of 135 generated files.
- The wave gate on f96b0c013 and CI on pull request https://github.com/beyond10x/connectors/pull/193 (8 of 8 checks on fb1d1a438).

## Limitations

As the CHANGELOG's 0.55.0 section states them: the connectors-definitions mirror of the Jira checks;
an exec plugin's busy or missing state not visible to the operator
(story:exec-plugin-busy-reaches-the-operator); the macOS OAuth drain cause inferred and not
confirmed by 20 CI runs.
