---
format: aep.planning-md/3
id: release-plan:connectors-v0610-definitions-install
kind: release-plan
status: draft
title: 'Release 0.61.0: connectors definitions install'
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Minor release 0.61.0, the closing release of wave 20261029-definitions-runtime under
`approval-record:wave-20261029-definitions-runtime`:

- `connectors definitions install --source <source> --tag <tag>` fetches a connectors-definitions release
  from an `https` repository or a local directory, verifies every file against `index.json`, and only then
  writes a read-only cache entry and `definitions.lock` beside the configuration file
  (`story:definitions-install`).
- Refusals are named (`digest_mismatch`, `unknown_format_version`, `unmet_feature`, `release_invalid`) and
  leave the lock and the cache as they were. The adversary pass's eight findings are fixed and kept as tests
  (`review-result:wave-20261029-adversary-u1`).

The CLI gains one command and four failure codes; nothing is removed or renamed.

## Scope

| story | status |
|---|---|
| story:definitions-install | implemented |

## Limitations

- No instance resolves through the lock yet: installing grants nothing and starts no process.
- The four features install admits (`embedded_text`, `query_fixed`, `query_folds`, `rate_limit_headers`) are
  not yet read by the definition lowerer, which refuses a published definition that uses them (D247).
- The CI gate on the pull request is the full gate; locally the touched crates' tests and the projection
  checks ran, under the shared target budget.
