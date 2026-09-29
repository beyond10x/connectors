---
format: aep.planning-md/3
id: release-plan:connectors-0-15-0
kind: release-plan
status: active
title: 'Connectors 0.15.0: ESS hardening round and desktop-safe custody'
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-09-29T03:59:31Z", actor: "human:timo", revision: 2}
---
## Scope

Release 0.15.0 of `beyond10x/connectors`: the ESS hardening round (design review plus
mutation, random sequences, caller replay, determinism, metamorphic relations and guard
analysis) and the custody fix that stops reading the desktop `default` Secret Service
alias. See CHANGELOG 0.15.0.

## Not in scope

- 70 synthesized scenarios over nested struct inputs, blocked on beyond10x/ess #234; the
  fixture-input runner work and its gate step follow next round.
- ESS and Entity Runtime features filed as beyond10x/ess #231-#237.
