---
format: aep.planning-md/3
id: decision-blocker:wave-20261013d-scope
kind: decision-blocker
status: cleared
title: What wave 20261013d delivers toward naming a store a newer binary wrote
relations:
- blocks: story:store-version-skew-refusal
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-13T21:21:19Z", actor: "human:timo", revision: 2}
---
## Question

What wave 20261013d delivers, released as 0.50.0.

## Options

| option | what | cost |
|---|---|---|
| A | `story:store-version-skew-refusal` as its Design section says: the refusal code `metadata_schema_newer` and the store-level writer marker in the specification first, then the marker and the check in `connectors-host`, the CLI mapping in `connectors`, and the CLI guide's state-directory section. One implementor, one adversary pass. | One tree, package gates of `connectors-host` and `connectors` (about 4-6G of target). |
| B | Only the documentation of the sharing rule; no refusal code. | No build; an older binary still tells a consumer to retry an error retrying cannot fix. |

Recommended: A.

Default on silence: A.

## Decided

Option A, 2026-10-13: wave 20261013d delivers story:store-version-skew-refusal as designed, on the integration branch `wave/20261013d`, with one pull request; release 0.50.0 also carries the OAuth loopback drain fix merged before it.
