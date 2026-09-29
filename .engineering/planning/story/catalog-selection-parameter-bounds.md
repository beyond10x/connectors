---
format: aep.planning-md/3
id: story:catalog-selection-parameter-bounds
kind: story
status: implemented
title: A catalog selection can bound a parameter, so per_page above the provider cap is refused
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/providers/gitlab/operations.json
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/engine.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_repository_reads_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/selection_bounds_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/shipped.rs
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T16:41:38Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-29T16:41:38Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-09-29T20:39:43Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Source

Adversary pass 1 on story:catalog-gitlab-repository-reads (2026-09-29): the provider sends
`per_page=101` unrefused on all four repository reads
(`adapters/catalog/tests/gitlab_repository_reads_adversary.rs`,
`per_page_above_the_provider_cap_is_refused_before_any_request`, ignored with this story's id).
GitLab caps `per_page` at 100 (its documented behaviour, not measured here), so with the documented
stop rule a walk ends after page one. The pinned bundle declares `per_page` with no maximum, and a
selection entry (`adapters/catalog/src/lib.rs` `Selection`) cannot narrow a parameter.

## Acceptance

- A selection entry can declare a bound for a query parameter; the GitLab list reads bound
  `per_page` to 100 (issues, merge requests, pipelines and the four repository reads).
- The ignored adversary case runs and passes; a `per_page` above the bound is refused as
  `invalid_input` before any request.

## Decided for the wave (coordinator, 2026-09-29)

- Selection key: `"bounds": {"<parameter>": {"maximum": <n>}}`, optional, skipped when absent so an
  unbounded selection serialises byte for byte as before; the format string stays
  `connectors-catalog-operations/1`.
- `declare` types every parameter `["string","integer","boolean"]` (`lib.rs:605`) and
  `parameter_values` stringifies scalars (`lib.rs:114`, `:320`), so a schema `maximum` alone would
  let `"101"` through. The bound is checked on the value parsed as an integer, before any request, and
  a non-integer or out-of-range value is refused as `invalid_input`. A bound naming a parameter the
  operation does not declare as a query parameter is refused when the selection loads.
- Every shipped GitLab read that declares `per_page` gets `maximum: 100`, `pipeline.jobs` included.
  No Jira bound: none of its walks ends on a short page.
- The pinned GitLab source carries no maximum for these operations, so the bound lives in the
  selection; the docs cite GitLab's cap as the reason.
