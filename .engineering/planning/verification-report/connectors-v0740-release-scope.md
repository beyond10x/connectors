---
format: aep.planning-md/3
id: verification-report:connectors-v0740-release-scope
kind: verification-report
status: accepted
title: connectors 0.74.0 release scope
summary: Scope, exclusions and evidence of the 0.74.0 cut
revision: 3
transitions:
- {from: "draft", to: "accepted", at: "2026-11-22T16:12:27Z", actor: "human:timo", revision: 3}
---
## Scope of connectors 0.74.0

Released from `release/0.74.0` (pull request beyond10x/connectors#271) on 2026-11-22.

- Security review 2026-11-22 (private, `connectors-cfb6610c`): finding 1 (a token
  endpoint moved under a saved client secret), finding 2 (the git relay's upstream origin),
  finding 6 (an open MCP stdio session follows a replaced connection), finding 9 (Windows
  install containment), finding 11 (legacy `login` collection custody), and seven RustSec
  advisories: rustls-webpki, webpki-roots and webpki-root-certs, libsqlite3-sys
  (entity-runtime 0.32.0, eventlog-core 0.9.4, rusqlite 0.41.0), ctutils, and two in
  pulldown-cmark (0.13.6).
- Linux Secret Service custody decodes GNOME's escaped collection names
  (`Standard-Schlüsselbund`), which answered `custody_unavailable` on 0.73.0.
- Issue #268: a missing store is `metadata_missing` with `initialize_state`.
- Issue #269: Slack `file.upload`, three requests under one approval.
- Test budget: nextest with a 60 s per-test kill, committed emission manifests with an input
  digest, no debug info in test builds, 77 fewer test binaries.
- Docs: 34 public pages tightened; UX round R-01 of the docs site (UXF-004 fixed).

## Not in this release

- Issue #267 (the metadata store grows by about 35 KB per read; 592 MB on the operator's
  workstation). Its long-term fix needs Entity Runtime work. Interim on the workstation:
  separate, rotated state directories for Slack posts and reads.
- The test step does not yet fail above 120 s; about 300 s of tests remain (estimate).
- The docs site's UX gate does not fit its 120 s budget yet; `ux/baseline.json` is unwritten.
- The workstation stays on connectors 0.65.2 until the operator answers the install question.

## Evidence

The release pull request's required checks ("repository gate", "common / Security and
privacy", "planning validate") on the release commit, and the verification of the tag, the
release page, its four archives, `SHA256SUMS` and the container image after publication.

## Verified 2026-11-22

- Release commit `356fc31d51a1657421df3c322c3d396a9877a6d6` on `origin/main`, the squash merge of
  pull request #271, authored by `b10x-bot[bot]`; its tree `d50ccf99770b` equals the tree the
  repository gate ran on (4,231 tests passed, test step 635 s).
- Annotated tag `v0.74.0` by `b10x-bot[bot]`, peeled to the release commit, an ancestor of
  `origin/main`.
- Release page https://github.com/beyond10x/connectors/releases/tag/v0.74.0, authored by
  `b10x-bot[bot]`, Latest, notes from the tag.
- Assets: the four `connectors-0.74.0-<target>.tar.gz` archives and `SHA256SUMS`;
  `sha256sum -c SHA256SUMS` passes for all four. The first release-binaries run failed on the
  x86_64-apple-darwin artifact upload (`ENOTFOUND`); the rerun of the failed jobs passed.
- Image `ghcr.io/beyond10x/connectors:0.74.0`: index `sha256:9c3ccd8a6d4460a3d5991b31cad87a87521d7e4fa7bf5934226aab79c257a2bc`
  (the digest the job's summary names), `linux/amd64` and `linux/arm64`, revision label the
  release commit.
- Known red: "host and CLI on windows-latest" fails with `failed to remove file
  target\debug\connectors.exe`, as on `main` before this release; not a required check.
- Workstation: 0.74.0 installed beside 0.65.2 in `~/.local/share/connectors/0.74.0/bin`; only the
  Slack-only and Tavily-only setups use it (Timo's pick H5 A, 17:00 CET).
