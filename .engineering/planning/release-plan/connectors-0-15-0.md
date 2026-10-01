---
format: aep.planning-md/3
id: release-plan:connectors-0-15-0
kind: release-plan
status: implemented
title: 'Connectors 0.15.0: ESS hardening round and desktop-safe custody'
revision: 4
transitions:
- {from: "draft", to: "active", at: "2026-09-29T03:59:31Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-01T07:33:25Z", actor: "human:timo", revision: 4}
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

## Closed

Closed 2026-10-01 by `story:planning-store-hygiene-20261001`: tag `v0.15.0` on `beyond10x/connectors` peels to `d8e5eeb0eafab89cc8320a9439781ca804fbf237` (2026-09-29); later releases up to v0.20.0 followed.
