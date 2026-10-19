---
format: aep.planning-md/3
id: story:docs-provider-pages-shared-guides
kind: story
status: draft
title: Provider pages keep their own facts and link shared guides for the repeated steps
summary: 'docs review run 3: about 36 KB repeated across provider pages; three wrong facts'
relations:
- serves: vision:independent-contract-adapters
- informed_by: specification:public-documentation-website
revision: 1
---
## Outcome

Each provider page under `website/docs/connectors/` keeps only its own facts (profile names, policy
line, example input, guard sentence, provider cause of a stale connection) and links to shared guides
for the steps every provider repeats.

## Why now

Post-0.54.0 docs review (run 3, 2026-10-19, lanes D3 and D4, merged and verified): blocks repeated
across provider pages, measured at 83aee7c96: first-read preamble (18 pages, 7,875 B), reconnect body
(20 pages, 12,233 B), approval block (9 pages, 7,755 B), signing-key paragraph (7 pages, 2,162 B),
instances JSON sample (11 pages, 2,348 B), Google setup (3 pages, 2,171 B), pinned-executable text
(3 pages, 1,806 B). Wrong facts on these pages: `$subject_sha256` used without saying where it comes
from (calendar:123, gitlab:124, jira:118, kubernetes:156, openrouter:111, runpod:110, slides:117);
`gitlab.md:5` says 30 reads, `adapters/catalog/providers/gitlab/operations.json` has 32; the 429
retry sentence differs across gmail, jira, drive, calendar and zendesk.

## Acceptance

- New `website/docs/guides/{first-read,guarded-write,reconnect-a-connection,google-oauth-setup,pinned-instances}.md`
  each hold one copy of the moved block; provider pages link to them.
- Every provider page keeps the eight section headings the `connectors-docs` gate requires.
- The three wrong facts above are fixed (`$subject_sha256` is the prepare answer's
  `subject_sha256`; 32 reads; one 429 sentence: "A read answered `429` (on Google, also a quota `403`)
  is sent once more when the delay the provider names fits before the call's deadline; otherwise it is
  `rate_limited` with `retry_after_seconds`. A write is never sent again.").
- The bytes of `website/docs/connectors/*.md` (except `index.md`, `mcp.md`) fall by at least 30,000,
  measured with `wc -c` before and after.
- Files: `website/docs/connectors/*.md` except `index.md` and `mcp.md`; the new guides.
