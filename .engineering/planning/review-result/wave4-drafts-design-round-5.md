---
format: aep.planning-md/3
id: review-result:wave4-drafts-design-round-5
kind: review-result
status: active
title: Design critic, round 5, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at 0426b51e0. Nothing was built or run. Both approvals are for the text at that commit.

- Provider budget that ends in an unanswered TLS handshake: approve.
- Sixteen ess_model tests: approve, with two warnings on wording that do not set the verdict.

## The two warnings of round 4

- Provider budget: resolved. The sentence on a budget that ends before the TCP connection
  opens carries "with at least 500 ms left at `send`", and the next one says that with less
  left the call is as today. The summary carries the floor.
- ess_model: resolved. Item 4 makes `sources` the directory `ess` reads. Acceptance 3 changes
  the bound `lte: 64` of `ProfileId` and asserts `"maxLength": 64` and `"maxLength": 65` in
  the two schema files, so an implementation that generates from a fixed directory fails it.

## The changes made for the other lanes

- `ess generate` on a copy needs the 4 files alone: `adapters/sql/spec/ess` holds
  `system.yaml` and three files under `domains`, `system.yaml` lists its domains by name, and
  no file refers outside the directory.
- In the default case the working directory of `ess` moves from `adapters/sql` to
  `adapters/sql/spec`, with `--path ess`. Inferred: `ess` reads nothing from the working
  directory. Acceptance 4 would catch a difference.
- The child test's name and its environment variable are used the same way in every item
  and acceptance that names them. No other file of the tree holds the variable.
- The key is made over the bytes of `sources`, which include `domains/profiles.yaml`, so the
  changed byte gives a new directory and a second line in `generations.log`.
- Provider budget: the paragraph on the measured run with 300 ms agrees with items 2 and 3
  and with the three tests at 1,000 ms. Lines 36 and 37 of `http.rs` are the end of
  `HttpConfig` and the start of `default_header`, so "adds no line above line 37" keeps the
  three cited ranges.

## Findings on wording

1. ess_model, item 1 (warning): it says the key is made over "the 4 files under
   `adapters/sql/spec/ess`", while item 4 makes it the `sources` argument. Item 1 reads "the 4
   files under the source directory".
2. ess_model, acceptance 3 (warning): `lte: 64` appears twice in `domains/profiles.yaml`, at
   line 20 and at line 29. The text names the first occurrence, line 20.

Outside this lane: acceptance 1 says each child "printed the same directory"; the test
harness captures a child's output, so the children need `--nocapture`.
