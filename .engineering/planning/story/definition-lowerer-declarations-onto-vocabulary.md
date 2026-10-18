---
format: aep.planning-md/3
id: story:definition-lowerer-declarations-onto-vocabulary
kind: story
status: draft
title: Definitions lower to the shipped connector.json declarations on the vocabulary main
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- depends_on: story:definition-lowerer-slice
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Every catalog provider definition in `beyond10x/connectors-definitions` lowers to a
`connector.json` byte-identical to the committed `adapters/catalog/providers/<catalog>/connector.json`,
on top of the connector-vocabulary change, and both pull requests carry that state: #179
(`impl/definition-lowerer`) and connectors-definitions #1 (`ess-validation`).

The lowerer and importer change is committed on `adopt/definition-lowerer-declaration` (daec32019,
on #179's head e51e7a08a); the definitions change on `adopt/declaration-ids` (288641c, on #1's head
14326c5, in connectors-definitions). The design records what cannot be lowered
(`architecture-design:provider-definitions-and-runtimes` section 14, revision 9).

## Waits on

The connector-vocabulary branch `design/connector-vocabulary` (3e9170466) merged to `main`: it adds
the twelve `connector.json` files the round-trip test compares against. Before that merge the
lowerer tests do not pass, so nothing is pushed to #179.

## Steps

1. After the vocabulary merge: merge `adopt/definition-lowerer-declaration` into
   `impl/definition-lowerer` (fast-forward), then `origin/main` into it (no force-push). Expect a
   conflict in `adapters/catalog/tests/jira.rs`.
2. Change `opsgenie/connector.json`'s identity to key order `source`, `path`, `kind`
   (`{"source": "configuration", "path": "v2/alerts", "kind": "opsgenie.integration"}`); the
   provider digests the parsed configuration, so no configuration revision moves.
3. If `main` has `providers/openrouter/connector.json`, drop `openrouter` from `UNDECLARED` in
   `crates/connectors-catalog/tests/definition_import.rs` and make `connectors/openrouter.yaml`
   lower to it; otherwise keep it in `UNDECLARED`.
4. Push `adopt/declaration-ids` to connectors-definitions #1 after its `task check`; keep
   `crates/connectors-catalog/tests/fixtures/definitions/` equal to it (`diff -r`, README excepted).
5. Push #179 through the bot route; both PRs green in CI; record `test_result` evidence here.

## Acceptance

1. `cargo test -p connectors-catalog --test definition_import` passes on #179's head: 15 round
   trips, 15 committed comparisons, the refusal tests, every lowered `connector.json`
   byte-identical; a one-byte change to a committed declaration fails it.
2. `task check` in connectors-definitions: every schema and instance valid.
3. #179 and connectors-definitions #1 green in CI on the pushed heads.
4. Nothing merges before `architecture-decision-record:provider-definitions-and-runtimes` is
   accepted (`decision-blocker:provider-definitions-adr-acceptance`).

## Known

13 tests in connectors-catalog, connectors-build and connectors-catalog-provider require ess 0.59.0;
locally the ambient `ess` is 0.60.0 (use `CONNECTORS_ESS` with the 0.59.0 toolchain). CI pins the
right one. ESS gaps found on the way: beyond10x/ess issues 590-594.
