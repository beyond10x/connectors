---
format: aep.planning-md/3
id: story:native-models-in-connectors-namespace
kind: story
status: draft
title: Adapter-owned models are named connectors.<owner>.<domain>
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Every adapter-owned ESS model is named `connectors.<owner>.<domain>` instead of
`connectors_<owner>.<domain>`, so native types read as part of the one Connectors namespace
(`connectors.tavily.requests.SearchRequest`, beside the shared `connectors.websearch.SearchInput`).
Requested by the operator on 2026-10-05.

## Findings (2026-10-05)

| Fact | Source |
|---|---|
| ESS requires a domain to start with its system's name: under `system: connectors_tavily` the domain `connectors.tavily.requests` is refused ("a domain of `connectors_tavily` is named `connectors_tavily.something`") | ESS 0.45.0 `ess specify validate`, probe |
| Under `system: connectors` the same domain validates | same probe |
| The boundary gate checks each adapter model's system name as `connectors_<owner>` | `crates/connectors-build/src/ess_boundary.rs:383` |
| The rule is stated for every owner | `adapters/README.md`, Owners |
| 19 files under `adapters/*/spec/ess` and `crates/connectors-build/src` use `connectors_<owner>.` | `grep -rln 'connectors_[a-z]*\.'` |

## Acceptance

- Every `adapters/*/spec/ess/system.yaml` declares `system: connectors`, and every native domain is
  `connectors.<owner>.<domain>`; each root still validates and compiles on its own.
- The boundary gate checks the `connectors.<owner>.` domain prefix where it checked the system name,
  and still refuses a model under another owner's prefix and any shared-root import of a native type.
- `adapters/README.md` states the new rule; its gate fixtures use it.
- Nothing in the shared model (`ess/`) changes.
