---
format: aep.planning-md/3
id: story:catalog-selection-required-parameters
kind: story
status: implemented
title: A selection can mark a parameter required when the provider requires it but its document does not
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T17:43:07Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-09-30T17:43:07Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"verification":1}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:44Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Source

Adversary pass 1 on Google Drive reads (review-result:adversary-google-drive-slides-reads-pass-1,
finding 2): Drive refuses `about.get` without `fields`, but the pinned Discovery document does not
mark it required, `declare` copies `required` from the bundle only (`adapters/catalog/src/lib.rs:687`)
and `Selection` has no override, so a describe-driven caller's `{}` spends a request on a certain 400.

## Acceptance

- A selection entry may carry `required: ["<query parameter>"]`; the engine refuses at load a name
  that is not a query parameter of the operation, and `declare` lists it as required.
- `adapters/catalog/providers/google-drive/operations.json` marks `about.get` `fields` required;
  `about_get_declares_fields_required` in `adapters/catalog/tests/google_reads_adversary.rs` then
  asserts the requirement instead of today's state.
- Invoking `about.get` with `{}` is `invalid_input` before any request.
