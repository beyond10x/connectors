---
format: aep.planning-md/3
id: verification-report:connectors-v0740-release-scope
kind: verification-report
status: draft
title: connectors 0.74.0 release scope
summary: Scope, exclusions and evidence of the 0.74.0 cut
revision: 1
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
