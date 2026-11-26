---
format: aep.planning-md/3
id: review-result:wave4-drafts-acceptance-round-4
kind: review-result
status: active
title: Acceptance critic, round 4, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at 9fc5d4b18. No build and no test was run.

- Sixteen ess_model tests: needs-revision, one warning.
- Provider budget that ends in an unanswered TLS handshake: approve. No finding.

## The findings of round 3 on the ess_model draft

All three are resolved in the text as it stands.

1. The acceptance on assertions is limited to functions marked `#[test]` or `#[tokio::test]`,
   and the helper's own `assert!` is named as moved. No line of the 16 tests that holds
   `assert` holds a call of `schema`, so the edits of the call sites remove none.
2. The helper's use can be checked: `schemas_in` and `schema(name)` are named, with the three
   callers of `schemas_in`, the single `Command::new` and the signature of `schema`.
3. The changed byte is named.

## The ess_model draft

- Acceptance 1 can be checked: 1 line in `generations.log`, 4 children that exit 0, 4 equal
  printed directories. A child run that filters to no test exits 0 too; the log and the
  printed directory catch that.
- The child test has no name in the text. A name gives the children's filter a target. Not a
  finding.
- Acceptance 2 records whichever line count the run without the lock shows; it records
  whether the test can turn red and does not require it. Not a finding.
- Acceptances 4, 5, 7, 8 and 9 each name an observation.

Finding (warning): acceptance 3 changes a byte inside a comment line, so the generated output
is the same, and acceptance 6 checks the text of the body of `schemas_in` alone. An
implementation that keeps the working directory and the path in a constant outside that body
passes every line and generates from the original directory. Acceptance 3 changes a byte that
the schema carries, for example the bound `lte: 64` of `connectors_sql.profiles.ProfileId`,
and asserts that the schema file differs between the two directories. Not established by
reading: that `ess` writes that bound into the schema.

## The provider budget

- Acceptance 1 can be checked: the peer's byte count, the bound of 2 s, the unmarked timeout
  and the red-first run on the base are named.
- Its margin: the connect's bound is at about 750 ms and the request's own timer at about
  1,000 ms. If the test process is not scheduled for more than 250 ms in that window, both
  timers fire in one turn, the request's timer is polled first, and the test fails without a
  defect. Inferred, not measured. The neighbours in `http_budget.rs` run on a margin of 30 ms.
  A larger budget in the test does not widen that margin; a longer lead would (lead 500 ms,
  floor 1,000 ms, test budget 2,000 ms). Not a finding.
- Acceptance 3 is the one that tells a bound on the connect from a bound around the whole
  `send`. Acceptance 2 cannot turn red from any form of this change: it pins `body_error`.
- Acceptances 4 to 8 name files, tests and functions that exist. "The read POST" is
  `post_json` in `http.rs`.
- The floor of 500 ms has no observation at its edge; the points at 150 ms and 200 ms are
  observed.
