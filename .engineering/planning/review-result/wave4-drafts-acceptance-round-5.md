---
format: aep.planning-md/3
id: review-result:wave4-drafts-acceptance-round-5
kind: review-result
status: active
title: Acceptance critic, round 5, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at d7124123d. No build, no test and no `ess` call was run. Both approvals are for the
text at that commit.

- Sixteen ess_model tests: approve. No finding.
- Provider budget that ends in an unanswered TLS handshake: approve. No finding.

## The warning of round 4 on the ess_model draft

Resolved. Acceptance 3 changes `lte: 64` to `lte: 65`, a byte the schema carries, and asserts
`"maxLength": 64` in the first directory and `"maxLength": 65` in the second.

- Verified: line 20 of `adapters/sql/spec/ess/domains/profiles.yaml` is the first `lte: 64`,
  under `connectors_sql.profiles.ProfileId`.
- Not verified by reading: that `ess` 0.97.4 writes the bound as `maxLength` into the schema
  file. The draft records its own read of 2026-11-25; taken as a recorded observation.
- No implementation passes every line and generates from the wrong directory. Generating
  from the default directory in both calls shows 64 twice. A key over the default directory
  gives one directory and one log line. Acceptance 6 keeps the manifest directory and
  `spec/ess` out of the body of `schemas_in`.

## The other changes of the ess_model draft

- The child test has a name, and the children run with `--exact` and `--nocapture`, so the
  filter has a target and each printed directory reaches the test.
- Item 1 makes the key over the files of the source directory the helper is given. That
  agrees with item 4 and acceptance 6.
- Acceptances 1 to 9 each name an observation. Acceptance 8 adds up: two new tests and the
  child test are three.

## The provider budget

The approval of round 4 holds. The new paragraph of "Defect" agrees with item 3 and with the
tests at 1,000 ms. The summary agrees with item 2. `post_json` in `http.rs` is the read POST
that acceptance 5 means.
