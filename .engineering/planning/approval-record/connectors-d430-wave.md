---
format: aep.planning-md/3
id: approval-record:connectors-d430-wave
kind: approval-record
status: approved
title: 'Wave D430: macOS provider start and the named derivation failure (0.65.2)'
summary: 'Fix release 0.65.2 for a downstream adopter: provider start through the private copy on macOS, and connect naming a failed provider start'
relations:
- decides: story:macos-release-provider-starts-through-the-private-copy
- decides: story:connect-names-a-failed-provider-start
revision: 4
transitions:
- {from: "draft", to: "approved", at: "2026-11-03T22:00:26Z", actor: "human:operator", revision: 3}
---
## Wave D430

The orchestrator, 2026-11-03, D430, under its grant: a fix release 0.65.2 for a downstream adopter on a macOS
Apple-silicon laptop. Under D169 a fix a consumer is blocked on today goes alone; the consumer is named only as "a
downstream adopter".

Integration branch `wave/d430-macos-provider-start`, managed tree `conn-d428`, from `origin/main` `413c60efc`
(v0.65.1), carrying the D428 record (`approval-record:connectors-d428-v0651-ignore-exemption`).

Units:

1. `story:connect-names-a-failed-provider-start`: a Linux red test first
   (`apps/connectors/tests/catalog_provider_failure.rs`), then the refusal names the failed provider start.
2. `story:macos-release-provider-starts-through-the-private-copy`: a `macos-latest` CI case that starts a
   release-built provider through the host twice, red before the fix; the cause with its CI log goes to the
   orchestrator before a commit that changes the copy is pushed.

Terms: targeted tests through `build-slot.sh`; the gate plan is asked of the orchestrator before the full gate; one
bot pull request merged through `PUT /pulls/<n>/merge`; tag `v0.65.2`, release, verify, announce. No customer, person,
host or channel name enters the repository, the store, a commit or a pull request.

## Scope of 0.65.2 (D440, 2026-11-04)

The orchestrator, after the CI case did not reproduce the kill, set the scope of 0.65.2:

- in: the fix for unit 1 (a killed or exited provider start answers `unavailable` at `installed_provider`, not
  `invalid_input` on `endpoint`), the error that names the failed provider start, and the macOS `release-start`
  job in `.github/workflows/macos-host.yml` as a standing guard;
- out: unit 2's fix. Its story stays active, citing the CI evidence and the questions put to the adopter. The
  CHANGELOG says the kill is under investigation and not reproduced on CI macOS 26.6.2. No change to the copy.

Also filed by this wave: `story:clock-quorum-test-tolerates-a-slow-loopback-source` (D433) and
`story:endpoint-publish-holds-one-writer` (D440), both drafts from findings.
