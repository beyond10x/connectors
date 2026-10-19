---
format: aep.planning-md/3
id: story:upstream-sources-gzip-only
kind: story
status: draft
title: Vendored upstream documents are tracked as their gzip archive only
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:remove-unshipped-material-20261019
revision: 1
---
## Outcome

Each vendored upstream document is tracked as its gzip archive only; the redacted plain copy is
derived into the build's temporary directory by `connectors-build catalog --source` and the gate's
`source-hashes` and redaction steps. Removes 14 plain files (14.6 MB) and the duplication the
architecture review of 2026-10-19 named (#2).

## Why not on 2026-10-19

`upstream_redaction::run` reads the plain file; `definition import` locates it by name and digest;
about 30 test files and 15 fixture READMEs name the plain path; five providers have no archive yet
and two keep no upstream copy at all. The referrer check put this at two hours with the highest
chance of breaking the release gate, so it was deferred.

## Acceptance

- `git ls-files 'adapters/*/upstream/*' | grep -v -E '\.gz$|README|LICENSE|amendments'` is empty.
- The gate's `source-hashes` and redaction steps and every catalog test pass reading the archive.
- Bundles record the same `source.file_name` as before (no bundle drift).
