---
format: aep.planning-md/3
id: release-plan:connectors-v0510-slack-writes-incident-reads
kind: release-plan
status: active
title: 'Release 0.51.0: Slack writes, incident history reads and call profiles'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-14T22:05:53Z", actor: "human:timo", revision: 2}
---
## Outcome

Minor release 0.51.0 of waves 20261014a, 20261014b and 20261014c (pull request
https://github.com/beyond10x/connectors/pull/170, merge 0db71408f): Slack message send, edit and
delete through the catalog provider; Opsgenie and Grafana Cloud IRM history reads; SQL read
profiles and Loki line and trace profiles for call lookup and call traces; read-only mounted-secret
custody; release archives carrying the catalog provider; the clock refusal naming its cause; a
bootstrap revision change keeping a recovery path; stopping an older owner after an upgrade; the
Windows sidecar retirement admission; planning with AEP 0.71.3.

Minor rather than patch: new operations, a new custody binding, a new failure field
(`clock_refusal`) and changed refusal codes for SQL statements and connect.

## Scope

The stories moved to implemented with this release:

- story:aep-pin-0713
- story:catalog-opsgenie-history-reads
- story:clock-refusal-names-its-cause
- story:grafana-irm-history-reads
- story:incident-call-lookup
- story:incident-call-traces
- story:mounted-secret-custody
- story:owner-stop-after-upgrade
- story:parity-slack-message-writes
- story:release-ships-catalog-provider
- story:revision-change-keeps-a-recovery-path
- story:store-version-skew-refusal
- story:windows-sidecar-retirement-admission

## Evidence

- The pull request's CI on head 7371f097d, all green: repository gate, host and CLI on
  macos-latest and windows-latest, custody on macos-latest and windows-latest, documentation
  build, planning validate, common security and privacy. The merge commit's tree equals that head's
  tree (`git diff 7371f097d 0db71408f` is empty).
- The Windows run before it failed `local::clock::tests::each_refusal_names_its_cause`: Windows
  refuses an overlong datagram where other systems truncate it. Fixed in 2179194b1 before the
  merge; package gate of connectors-host on that commit 487 passed, 0 failed, with clippy on Linux
  and on the Windows GNU target.
- The release commit changes only the version, the lockfile, the README version line and the
  changelog; the generated documentation and CLI contract were checked current on it.

## Completion boundary

- story:jira-feed-binding and story:confluence-feed-binding did not ship: no binding under
  `adapters/catalog/contracts/feed/v1alpha1/` besides GitLab's. They stay active.
- The Slack writes need an approval, and an approval needs the single-source clock to answer; the
  quorum clock (story:approval-clock-quorum) replaces it in a later release. No write was invoked
  against a real Slack.
- The OAuth loopback race on the macOS runner is story:oauth-drain-macos-race.
- macOS release signing waits on the signing secrets.
