---
format: aep.planning-md/3
id: review-result:wave-20261029-adversary-u1
kind: review-result
status: active
title: Adversary pass over wave 20261029-definitions-runtime, unit U1
summary: Eight findings on definitions install, two should-fix; all fixed in 1dd9b8653
relations:
- reviews: story:definitions-install
revision: 1
---
# Adversary pass over wave 20261029-definitions-runtime, unit U1

**Verdict: eight findings, one should-fix (unbounded inflation), one should-fix (an existing cache entry accepted with extra files, writable modes or as a link), six nits; all fixed in `1dd9b8653`, merged into `wave/2026-10-29-definitions-runtime` as `db73f6d16`.**

The pass ran on 2026-10-29 over `f9548810d` (`story:definitions-install`). Cases executed went from 18 to 34; 9 were red before the fix and all pass after it.

| Finding | What the code did | Fix |
|---|---|---|
| F1 | a 3 MB asset with a 640 MiB GNU long-name entry drove install to 1.36 GB resident | inflation capped at the indexed files, padded to 512-byte blocks, plus 8 KiB per file and 1 MiB; past it `release_invalid` |
| F2 | an existing entry with an extra file, writable modes or a symbolic link in place of the directory was accepted | the entry must be a directory holding exactly `index.json` and the indexed files, modes 0444 and 0555; else `digest_mismatch` |
| F3 | a lock that could not be written left a new empty `installed-definitions/` | a cache root this call created is removed on refusal |
| F4 | a path segment over 255 bytes failed the write as a retryable `unavailable` | `release_invalid` at index read |
| F5 | two paths differing only in case installed on Linux and would fail on macOS | `release_invalid` |
| F6 | an unsorted or repeated `features` list was accepted and reported as is | `release_invalid` |
| F7 | a `Source::Https` built without `Source::parse` fetched over plain http | the fetcher re-checks the source |
| F8 | the redirect policy followed four redirects, the contract says five | `previous().len() > 5` (reqwest counts the initial URL) |

Held: path traversal through `..`, absolute paths, backslashes and pax or GNU long-name overrides; link entries; duplicate entries; header size against data; duplicate JSON keys; 18 concurrent installs; install before `setup init`; HTTP 404, 410, 500, a redirect to http, an over-long Content-Length; one live install of connectors-definitions 0.4.0 over HTTPS (145 files, lock mode 0600).

The red cases are kept as tests: `crates/connectors-definitions-install/src/adversary_tests.rs` and `apps/connectors/tests/definitions_install_adversary.rs`.

```findings
[
{"file":"crates/connectors-definitions-install/src/archive.rs","line":26,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"a 3 MB asset with a 640 MiB GNU long-name entry drives install to 1.36 GB resident, past the 580 MiB the stated index, asset and file bounds sum to"},
{"file":"crates/connectors-definitions-install/src/store.rs","line":169,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"an existing entry with an extra file, writable modes or a symlink in place of the directory is accepted as installed, against semantics.md and the ESS read-only CacheEntry"},
{"file":"crates/connectors-definitions-install/src/store.rs","line":104,"category":"acceptance","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"when the lock cannot be written the refusal leaves a new empty installed-definitions directory"},
{"file":"crates/connectors-definitions-install/src/index.rs","line":53,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a path segment over 255 bytes passes the index checks and fails the write as unavailable with retry_explicitly, which no retry can satisfy"},
{"file":"crates/connectors-definitions-install/src/index.rs","line":50,"category":"property","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"two index paths differing only in case install on Linux and fail as a retryable write on a case-insensitive macOS file system"},
{"file":"crates/connectors-definitions-install/src/index.rs","line":138,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"an unsorted or duplicated features list is accepted and reported as is, though ess/domains/cli.yaml calls the result's features sorted"},
{"file":"crates/connectors-definitions-install/src/fetch.rs","line":21,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the public Source::Https variant lets a library caller skip Source::parse, and install then fetches over plain http and locks an http source"},
{"file":"crates/connectors-definitions-install/src/fetch.rs","line":88,"category":"contract-drift","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the redirect policy follows at most four redirects because reqwest counts the initial URL in previous(), while semantics.md promises five"}
]
```
