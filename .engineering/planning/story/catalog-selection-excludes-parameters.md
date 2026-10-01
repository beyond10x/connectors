---
format: aep.planning-md/3
id: story:catalog-selection-excludes-parameters
kind: story
status: draft
title: A catalog selection can withhold a declared parameter
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/providers/gitlab/operations.json
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: inferred
  path: adapters/catalog/tests/engine.rs
- confidence: inferred
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_commit_reads_adversary.rs
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 3
---
## Observed

2026-10-01, adversary pass on story:catalog-gitlab-commit-reads (review-result:adversary-gitlab-commit-reads-pass-1,
`adapters/catalog/tests/gitlab_commit_reads_adversary.rs:140`): `commits.list` sends `pagination=keyset` to GitLab
unchanged. Under keyset GitLab pages by a response header the provider drops (GitLab behaviour, not observed here),
so the documented page/per_page walk breaks for a caller who passes it. A selection in `operations.json` can bound a
parameter (`bounds`) but cannot exclude one, and `adapters/catalog/src` has no mechanism for it.

## Acceptance

- A selection entry can name declared parameters it withholds; an input carrying one is refused as `invalid_input`
  before any request, and `operations describe` does not list it.
- GitLab `commits.list` withholds `pagination`; the adversary case at :140 asserts the refusal.
