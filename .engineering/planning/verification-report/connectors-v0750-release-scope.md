---
format: aep.planning-md/3
id: verification-report:connectors-v0750-release-scope
kind: verification-report
status: accepted
title: connectors 0.75.0 release scope
summary: Scope, exclusions and evidence of the 0.75.0 cut
revision: 3
transitions:
- {from: "draft", to: "accepted", at: "2026-11-23T08:30:53Z", actor: "human:timo", revision: 3}
---
## Scope of connectors 0.75.0

Released 2026-11-23 under Timo's instruction of that day: "We always merge what we can merge, and
if you're done, cut a new release" (`AGENTS.md`, Cutting a release), and host-operator's order of
03:59 CET that the gaps of Runbook and dev-operator go first.

- story:invoke-admission-queues-under-load: `connectors invoke --deadline-seconds <n>` names a
  read's deadline, 1 to 120 seconds, 30 when left out. Consumer gap: dev-operator's parallel
  reads were refused `timeout` at `admission` after the fixed 30 seconds.
- A read on `invoke` has one cutoff, taken before its owner is started. Before, a read that
  started its owner had up to 30 seconds for the start and 30 more for the read.
- A read that validates its stored credential again after an executable change ends that
  validation at the earlier of its deadline and the 30-second capture lease.
- Minor release: one added flag. Compatibility: a cold read that needed between 30 and 60
  seconds in total answered before and is now refused `timeout` unless it names a longer
  deadline.

## Not in this release

- Issue #267 (store growth per read): the Entity Runtime 0.33.0 pin
  (story:entity-runtime-0330-records-name-definition-by-digest) waits for that release, and
  story:settled-read-uses-leave-the-store follows it.
- story:timeout-refusal-says-how-long-it-waited (`waited_ms`), draft.
- Acceptance line 11 of story:invoke-admission-queues-under-load, the measurement of 16 parallel
  reads with `--deadline-seconds 120` on a real instance: it runs with the released binary and is
  recorded on the story and on issue #267. The story stays active until then.
- The other draft stories under epic:review-20261123-findings and epic:slack-complete-web-api.

## Evidence

Verified 2026-11-23 after publication:

- Release pull request #283 merged at 08:35 CET as `5983a6b950233cfe9e1766e82202861ff5d85289`; its
  tree `cce27bb37f49` equals the pull request's tree, whose required checks passed.
- Not green on that pull request: "host and CLI on windows-latest" (run 45124020139), which is
  not a required check: `failed to remove file target\debug\connectors.exe`, Access is denied,
  in the MCP step, as on every code change since 2026-11-22. The fix is pull request #284
  (story:windows-host-job-releases-the-cli-binary). The Windows binary of this release was
  therefore not shown by that job's MCP steps.
- Tag `v0.75.0` by `b10x-bot[bot]` at 08:35 CET; it peels to the merge commit, which is the head
  of `origin/main`.
- Release page https://github.com/beyond10x/connectors/releases/tag/v0.75.0, author
  `b10x-bot[bot]`, not a draft, Latest, published 08:37 CET.
- Release run 45132464036: created 07:37 UTC, ended 08:27 UTC, all 7 jobs succeeded, no rerun.
- The page has 5 assets: the 4 archives pass `sha256sum -c SHA256SUMS`.
- Image `ghcr.io/beyond10x/connectors:0.75.0`, index
  `sha256:b8f89529c29f1ad9a6b3f59213e26b2d9c9a0809c10ef43febeae3cbf3f760ca`, platforms
  linux/amd64 and linux/arm64, label `org.opencontainers.image.revision`
  `5983a6b950233cfe9e1766e82202861ff5d85289`.
- Four main-push runs on the merge commit were cancelled at host-operator's order for the runner
  queue (Shared source gates 45132211702, Windows host 45132211327, macOS host 45132211328, Rust
  gate 45132211258): the tag run and the pull request's runs cover the same tree.
- Acceptance line 11 of story:invoke-admission-queues-under-load, with the released binary at
  08:28 UTC on the Slack read-only instance of issue #267 (store 5.0 MB, host load 89.5): 16
  parallel reads with `--deadline-seconds 120`, 16 answered, none refused `timeout`, 9.2 to
  38.5 s per read, 11 of them over 30 s
  (https://github.com/beyond10x/connectors/issues/267#issuecomment-7095515304). Baseline on
  0.74.0: 5 of 16 answered, on a 12 MB store.
- Both read-only side instances of this workstation run 0.75.0 on their kept stores: the first
  read after the move answered in 12 s (Slack) and 14 s (Jira).
