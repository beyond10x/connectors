---
format: aep.planning-md/3
id: decision-blocker:provider-definitions-adr-acceptance
kind: decision-blocker
status: cleared
title: Accept the provider definitions and runtimes decision record
relations:
- blocks: story:definition-lowerer-declarations-onto-vocabulary
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-17T12:17:40Z", actor: "human:timo", revision: 3}
---
## Question

Does the operator accept `architecture-decision-record:provider-definitions-and-runtimes`
(proposed)? Connector definitions in `beyond10x/connectors-definitions`, lowered by
`connectors-build definition` into the catalog provider's bundle, selection and `connector.json`.

## Holds

Neither #179 (`impl/definition-lowerer`) nor connectors-definitions #1 merges until the record is
accepted. Work on both branches goes on: `story:definition-lowerer-declarations-onto-vocabulary`.

## Evidence for the decision

The design (`architecture-design:provider-definitions-and-runtimes`, revision 9) lists in section
14 what cannot be lowered and why; the round-trip tests show all 15 shipped selections lowered
byte-identical (bundle, selection on every field, declaration).


## Decided

The operator accepted `architecture-decision-record:provider-definitions-and-runtimes` on 2026-10-17. #179 and connectors-definitions #1 merge after the connector-vocabulary landing, as `story:definition-lowerer-declarations-onto-vocabulary` says.
