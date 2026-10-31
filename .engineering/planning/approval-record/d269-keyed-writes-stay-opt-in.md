---
format: aep.planning-md/3
id: approval-record:d269-keyed-writes-stay-opt-in
kind: approval-record
status: approved
title: Keyed writes under a standing grant stay opt-in in 0.62.0 (D269)
relations:
- decides: story:served-keyed-writes-under-a-standing-approval
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-30T09:51:49Z", actor: "human:timo", revision: 2, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
---
## Decision D269 (2026-10-30)

Keyed writes under a standing grant stay opt-in in 0.62.0. A host whose service configuration has
no `standing_grants` dispatches unkeyed writes on `POST /v1alpha2/invoke` as before. The default
changes only after loom sends `Client::invoke_v1alpha2_keyed`, so no consumer breaks.

Follow-ups recorded in the 0.62.0 CHANGELOG under Limitations:

- keyed attempts on the HTTP host never expire;
- a keyed write cut off by a restart is not recovered (its replay answers `outcome_unknown`);
- `connectors-oauth2` and the local host's OAuth handling still read a JSON-array token answer
  field by field;
- the lowerer and the catalog engine check a definition's members separately.
