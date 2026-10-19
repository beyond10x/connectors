---
format: aep.planning-md/3
id: story:remove-unshipped-material-20261019
kind: story
status: implemented
title: Material nothing ships, runs or reads is removed
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .github/workflows/custody-platforms.yml
- confidence: cited
  path: adapters/atlassian/contracts/documents
- confidence: cited
  path: adapters/sql/examples/live_deadlines.rs
- confidence: cited
  path: contracts/media
- confidence: cited
  path: crates/connectors-build/ess-boundary.json
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: cited
  path: crates/connectors-custody/examples
- confidence: cited
  path: crates/connectors-docs/src/reference.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: crates/connectors-spec
- confidence: cited
  path: docs/design.md
- confidence: cited
  path: docs/evidence
- confidence: cited
  path: ess/system.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:38Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:38Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-19T05:54:49Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":2}}}
---
## Outcome

The repository no longer carries material nothing ships, runs or reads, as the architecture
review of 2026-10-19 proposed and a referrer check verified: the evidence records (`docs/evidence`,
1,364 files, 23.4 MB) live on branch `history/evidence-2026-10`; the wave, plan and handoff records
under `docs/`, six dated documents and `docs/design.md` §27–30 and §32–34 are history; the native
Atlassian documents profile, the `datasource_reads` domain, the unexecuted session scenarios, the
`media` family and the compositions notes are removed; the specification-only `docker`, `sip` and
`rtvbp` directories are removed; the unused v2 generator of `connectors-spec`, `connectors-build
package` and `check`, the SQL deadline example and the keychain-probe examples with their
non-failing CI step are removed.

## Kept, with the reason

`planning.yml` (a required status check), `contracts/sessions` and the `sessions` domain (MCP
tests), `auth_access` and `discovery_state` (typed by other domains and the website example),
`adapters/atlassian/spec/ess` (read by `confluence_v1.rs`), `docs/local-execution-audit.md` and
`design.md` §31 (current material), `homer` and `alertmanager` (parity stories), the gzip-only
upstream sources and the in-temp projections (follow-up stories: 30 test files and 8 tests read
the plain copies), the MCP specification, the federation path and the metadata-store migration
floor (decided separately).

## Acceptance

- The full repository gate passes on the tree without the removed material; `connectors-docs
  generate --check` and the vocabulary check pass; `ess specify validate --path ess` passes.
- Every remaining reference to a removed path is a history note naming commit `6498a881c` or the
  history branch.
- The CHANGELOG entry lists each removal with its size.
