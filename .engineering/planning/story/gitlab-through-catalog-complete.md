---
format: aep.planning-md/1
id: story:gitlab-through-catalog-complete
kind: story
status: implemented
title: Expose every GitLab operation through the catalog provider with a committed selection set
relations:
- decomposes: epic:retire-native-gitlab-adapter
- derived_from: specification:catalog-http-runtime-handoff
- serves: vision:independent-contract-adapters
revision: 5
---
## Outcome

The catalog provider serves every GitLab operation the native adapter serves,
from a selection set committed in the repository rather than typed into each
operator's configuration, with `merge_request.merge` guarded declaratively.

## Scope

- `adapters/catalog/src/lib.rs`: a guard's preflight and postflight each carry a
  list of `checks`; a check compares a JSON pointer in the observed body against
  `{"input": "body.sha"}` or `{"literal": "opened"}`. A selection may declare
  `"response": "text"` for a read whose source declares JSON where the provider
  answers text; otherwise the bundle's declared 2xx media types decide.
- `adapters/catalog/src/local.rs`: format `connectors-catalog-local/2`;
  `operations_file` names a shipped selection set (`connectors-catalog-operations/1`,
  provider must match) appended to the inline `operations`.
- `adapters/catalog/providers/gitlab/operations.json`: 14 selections — 11 reads
  including `branch.get` and `job.trace` as text, `merge_request.create`,
  `merge_request.update`, `merge_request.merge` under a five-check guard (pinned
  `body.sha`, `state` opened, `detailed_merge_status` mergeable, `head_pipeline/id`
  pinned, `head_pipeline/status` success; postflight state merged, sha pinned).
- `adapters/catalog/tests/engine.rs` (5 tests), `tests/shipped.rs` (the shipped
  set resolves against the committed bundle and carries every native id).
- `docs/local-catalog-provider.md`, `docs/evidence/gitlab-sandbox-20260913/`
  (`shipped-*`, `gitlab-catalog-2.json`), `website/docs/adapters/catalog.mdx`,
  `README.md`, `CHANGELOG.md`, the two native GitLab write contracts.

Nothing under `adapters/gitlab/` changed except the two contract pages naming the
epic that retires them.

## Acceptance

- Sandbox, 2026-09-13, `cfg8`: all 11 reads `200`; merge of request 10 `applied`
  with 1 PUT to the merge endpoint (nginx access log); stale-head merge and
  wrong-pipeline merge `not_attempted` with 0 PUTs; update of the merged request
  `not_attempted` with 0 PUTs. Record: `docs/evidence/gitlab-sandbox-20260913/README.md`,
  section *The whole native surface from the shipped selection set*.
- `cargo test -p connectors-catalog-provider`: 7 tests, 0 failed.
- Repository gate: recorded as evidence on this story when it completes.
