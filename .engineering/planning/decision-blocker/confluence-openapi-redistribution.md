---
format: aep.planning-md/3
id: decision-blocker:confluence-openapi-redistribution
kind: decision-blocker
status: open
title: Nobody has decided whether the Confluence OpenAPI documents may be committed to a public repository
relations:
- blocks: story:catalog-confluence-reads
withholds: test_result
revision: 1
---
## Question

May the two Confluence Cloud OpenAPI documents be vendored (raw and gzip) into the public
`beyond10x/connectors` repository, together with the bundle generated from them?

## Facts (2026-09-29)

- `confluence-v2.json` (https://developer.atlassian.com/cloud/confluence/openapi-v2.v3.json, sha256
  edb639bbc700ee451a996acd2568e51db4ceab954449427537df30f0ce20ca08) declares no `license`; its
  `termsOfService` is https://developer.atlassian.com/platform/marketplace/atlassian-developer-terms/.
- `confluence-v1-search.json` (https://developer.atlassian.com/cloud/confluence/swagger.v3.json, sha256
  6c66a606fa7535268512f07f599fe1e3f9de2ba0b1da7eb6ead405509875577e) declares no `license`; its
  `termsOfService` is https://atlassian.com/terms/.
- The Jira platform v3 document, already merged in this wave, declares Apache 2.0 and its licence text is
  vendored beside it.
- `gh repo view beyond10x/connectors`: visibility PUBLIC.

## Held

The implemented unit stays on branch `impl/catalog-confluence-reads` in managed worktree
`wave0929b-confluence`, uncommitted, green on its own gate (catalog-provider 42 passed, catalog 139
passed, source-hashes 103 across 9 manifests). It includes the multi-source bundle pipeline
(`--source` repeated, `--select <file>=<operationId>`), which is useful without Confluence.

## Cleared by

An operator decision: vendor as is; vendor digests only and fetch at build time; or drop Confluence.
