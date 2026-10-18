---
format: aep.planning-md/3
id: story:definition-lowerer-declarations-onto-vocabulary
kind: story
status: implemented
title: Definitions lower to the shipped connector.json declarations on the vocabulary main
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- depends_on: story:definition-lowerer-slice
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T01:28:35Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-fix-20261018"}
- {from: "proposed", to: "active", at: "2026-10-18T01:28:35Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-fix-20261018"}
- {from: "active", to: "implemented", at: "2026-10-18T01:28:35Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-fix-20261018"}
---
## Outcome

Every catalog provider definition in `beyond10x/connectors-definitions` lowers to a
`connector.json` byte-identical to the committed `adapters/catalog/providers/<catalog>/connector.json`,
on top of the connector-vocabulary change, and both pull requests carry that state: #179
(`impl/definition-lowerer`) and connectors-definitions #2 (`adopt/declaration-ids`).

## State (2026-10-18)

- The vocabulary change merged to `main` as #182 (7eb6d9f18, 2026-10-17 16:59Z) and was merged into
  `impl/definition-lowerer` (7041e56ef). The lowerer and importer change is a0170a003 on that
  branch; #179's pushed head is b04416579, where all eight CI checks passed, the repository gate
  included (https://github.com/beyond10x/connectors/actions/runs/39081689581/job/117539768944).
- The definitions change is 288641c on `adopt/declaration-ids`, connectors-definitions #2, whose
  `task check` passed in CI. `crates/connectors-catalog/tests/fixtures/definitions/` equals that
  branch: 33 of 33 files byte-identical.
- `architecture-decision-record:provider-definitions-and-runtimes` is accepted (b04416579) and
  `decision-blocker:provider-definitions-adr-acceptance` is cleared.
- Not merged: both pull requests are open; merging is the coordinator's step.

## Acceptance

1. `cargo test -p connectors-catalog --test definition_import` passes on #179's head: 15 round
   trips, 15 committed comparisons, the refusal tests, every lowered `connector.json`
   byte-identical; a one-byte change to a committed declaration fails it.
2. `task check` in connectors-definitions: every schema and instance valid.
3. #179 and connectors-definitions #2 green in CI on the pushed heads.
4. Nothing merges before `architecture-decision-record:provider-definitions-and-runtimes` is
   accepted (`decision-blocker:provider-definitions-adr-acceptance`).

## Known

13 tests in connectors-catalog, connectors-build and connectors-catalog-provider require ess 0.59.0;
locally the ambient `ess` is 0.60.0 (use `CONNECTORS_ESS` with the 0.59.0 toolchain). CI pins the
right one. ESS gaps found on the way: beyond10x/ess issues 590-594. A hand-written trial definition
on 2026-10-18 found two defects this story's tests did not reach; they are
`story:definition-lowerer-trial-defects`.
