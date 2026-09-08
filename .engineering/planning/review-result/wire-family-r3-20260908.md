---
format: aep.planning-md/1
id: review-result:wire-family-r3-20260908
kind: review-result
status: active
title: Final independent family compatibility approval
relations:
- reviews: story:contracts-wire-compatibility
revision: 1
---
# Independent final compatibility review A

**Verdict: approve E02 specification changes. No remaining actionable E02 findings.**

Reviewed 2026-09-08 against the final working-tree sources snapshotted in `sources-final/`; `source-hashes-final.json` records SHA-256 and size for 34 files. The runtime baseline remains `3ba2d29870d577aac70c8a68ee901b3b7c6c99ac`. This was an independent read-only review: no other review was read, no new agent was delegated, and no tracked source was changed by this reviewer.

## Scope and disposition

The final common compatibility owner covers all fifteen proposed v1alpha1 family documents and the proposed governed service binding. Public additions are assigned to one of: closed extended descriptor/invocation/response fields; selected operation-local schemas; private host/SDK values; independent configuration/specification/artifact readers; reserved profiles; or explicitly unbound callback/control/data/delegation transports. Unsupported or unbound support cannot be advertised merely because a string or ESS type exists.

The original eight review findings are resolved: strict-reader refusal is explicit; mutation classification/replay is representable without conflation; descriptor profile and auth alternatives have one shape; sessions/media have separate binding requirements; mediated forwarding is private; generic limits are explicit; artifact versions are independent; and legacy projection preserves the entire selected old behavior with its own revision and invoke allowlist.

| Recheck finding | Final evidence | Result |
|---|---|---|
| A-R2-01 records/series inventory | compatibility §6 now names actual document input/item/body fields and unpaged behavior. Series retains numeric timestamps plus string values and reserves instant/labels. Series §4 explicitly puts `step_s` minimum in input_schema and window arithmetic in the selected profile, not extra Operation.limits members. | Fixed |
| A-R2-02 scopes omission | auth.profile §3 now gives the jira.api_token alternative `scopes: []`, matching compatibility §4, catalog and ESS AuthAlternative. | Fixed |
| A-R2-03 generic limit selection | compatibility §7 selects by `realization: generic`, explicitly covering generic read, mutation and page. These consistently use 256 KiB / 4 MiB / 40 s execution / 30 s provider / 5 s connect; ordinary non-generic mutation retains 64 KiB / 4 MiB / 20 / 15 / 5. | Fixed |
| A-R2-04 completion binding | acquisition §3 now distinguishes safe begin/status management from protected callback/entry `auth.complete`, explicitly excluding secret completion evidence from generic invoke/federation. | Fixed |

The final descriptor framing uses the common extended Response for successful and failed describe, null GET correlation, and a complete Descriptor in success.result. Both version fields agree. The closed extended error set and exact HTTP mapping are explicit. Safe early/audit failures do not invent trusted coordinates or acknowledged references. Legacy describe remains its original bare Descriptor shape.

The final mutation mapping preserves all four effect classifications, required nullable fields, current versus original correlation, replay delivery metadata, and secondary safe cause. Denied original disclosure omits mutation metadata. A known applied effect remains applied when result delivery or terminal persistence fails. Unknown remains outcome_unknown, and neither malformed responses nor HTTP errors authorize retry. The audit fields preserve acknowledged gateway/origin identity and separate audit completeness from effect certainty.

## Evidence reviewed

- Reviewed the full final compatibility/service documents and family changes, including the last series constraint-placement and stack S4 named-requirement corrections. Confirmed `git diff --numstat -- crates adapters spec-kinds` has no entries.
- Inspected all 27 proposed textual response vectors: 20 positive and 7 negative, with distinct IDs. Their expected classifications agree with the proposed rules for describe, generic profile/limits, replay, null fields, denied disclosure, audit/store failure, undeliverable known success, leaf audit provenance and HTTP/application consistency. These are authored textual examples, not outputs of a v1alpha2 codec.
- Verified all 68 legacy vector IDs uniquely match their recorded decode observations and expectations. The recorded temporary Rust harness links unchanged connectors-core and calls its existing read_json types; its preserved source SHA-256 matches `b91f3bd7124e672eeaa5081d4dad1b130e4bc22baa41ffd8af211893c23fdcf0`. This establishes recorded core decoding evidence, not future route/negotiation execution.
- Inspected the gate log and checked its test totals: 50 passed, 0 failed, 0 ignored. The evidence distinguishes pinned ESS 0.20.0 declaration/scenario compilation from sequential execution and future wire conformance. I did not rerun the full gate or present another agent's recorded run as my own execution.
- Verified all 34 final source snapshots against their recorded hashes and lengths.

## Remaining scope is explicit

This approval is for E02's textual transport/version disposition. F03 delegation; management/acquisition, readiness, discovery/log/media behavior; exact future callback/duplex/data codecs; current-authority verification; audit/attempt persistence; and runtime old/new binding conformance remain with their named owners. The model correctly leaves codec emission, cross-field predicates and trusted/persistent ownership as explicit obligations. None is silently claimed implemented or stabilized by this review.

No new runtime code or adapter-kind/generated schema change is needed to accept this E02 specification revision. This approval does not establish completion of the broader “all specs reviewed and stabilized” goal or authorize external rollout.
